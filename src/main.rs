use alabama_election_night::{Batch, ELECTION, Source, parse, requested};
use anyhow::{Context, Result, ensure};
use axum::{Router, extract::State, http::header, response::IntoResponse, routing::get};
use chrono::Utc;
use fs2::FileExt;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use tower_http::{compression::CompressionLayer, services::ServeDir};
mod relay;

#[derive(Clone)]
struct App {
    cache: Arc<RwLock<Value>>,
    root: Arc<PathBuf>,
}
fn now() -> String {
    Utc::now().to_rfc3339()
}
fn db(root: &PathBuf) -> Result<Connection> {
    let c = Connection::open(root.join("runtime/results.sqlite"))?;
    c.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS source_state(source TEXT PRIMARY KEY,batch TEXT,fetched_at TEXT,attempted_at TEXT,error TEXT); CREATE TABLE IF NOT EXISTS history(id INTEGER PRIMARY KEY,source TEXT,at TEXT,version TEXT,payload TEXT); CREATE TABLE IF NOT EXISTS evidence(id INTEGER PRIMARY KEY,source TEXT,at TEXT,url TEXT,sha256 TEXT,path TEXT); CREATE TABLE IF NOT EXISTS relay_state(source TEXT PRIMARY KEY,observed_at TEXT,received_at TEXT);")?;
    Ok(c)
}
fn load_json(path: PathBuf) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn canonical(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}
fn candidate_key(name: &str, party: &str) -> String {
    let party = match party.to_uppercase().as_str() {
        "D" | "DEM" | "DEMOCRATIC" => "DEM".to_owned(),
        "R" | "REP" | "REPUBLICAN" => "REP".to_owned(),
        _ => party.to_uppercase(),
    };
    format!("{}:{party}", canonical(name))
}
fn ballot_match(
    contest: &alabama_election_night::Contest,
    races: &BTreeMap<String, Value>,
) -> Option<String> {
    let mut matches = races.iter().filter(|(_, r)| {
        r.get("provider_name")
            .and_then(Value::as_str)
            .is_some_and(|s| canonical(s) == canonical(&contest.name))
    });
    if let Some((id, _)) = matches.next() {
        if matches.next().is_none() {
            return Some(id.clone());
        }
    }
    // Preserve raw labels in source rows. Match ballot navigation only when the full
    // non-write-in candidate/party set identifies exactly one ballot contest.
    let mut keys = contest
        .candidates
        .iter()
        .filter(|c| canonical(&c.name) != "WRITEIN")
        .map(|c| candidate_key(&c.name, &c.party))
        .collect::<Vec<_>>();
    keys.sort();
    if keys.is_empty()
        || keys
            .iter()
            .all(|k| k.starts_with("YES:") || k.starts_with("NO:"))
    {
        return None;
    }
    let matches = races
        .iter()
        .filter(|(_, r)| {
            let mut ballot = r["candidates"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| canonical(c["name"].as_str().unwrap_or("")) != "WRITEIN")
                .map(|c| {
                    candidate_key(
                        c["name"].as_str().unwrap_or(""),
                        c["party"].as_str().unwrap_or(""),
                    )
                })
                .collect::<Vec<_>>();
            ballot.sort();
            ballot == keys
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}
fn selected_contributions(
    entries: Vec<(String, alabama_election_night::Contest)>,
    sources: &[Source],
    reports: &[Value],
) -> Vec<(String, alabama_election_night::Contest)> {
    let rank = |c: &alabama_election_night::Contest| {
        let report = reports
            .iter()
            .find(|r| r["source"]["id"].as_str() == Some(&c.source_id));
        let unhealthy = report.is_none_or(|r| r["stale"] == true || !r["error"].is_null());
        let priority = sources
            .iter()
            .find(|s| s.id == c.source_id)
            .map(|s| s.priority)
            .unwrap_or(u32::MAX);
        (unhealthy, !c.precinct_detail, priority)
    };
    let observed = |c: &alabama_election_night::Contest| {
        reports
            .iter()
            .find(|r| r["source"]["id"].as_str() == Some(&c.source_id))
            .and_then(|r| r["fetched_at"].as_str())
            .unwrap_or("")
            .to_owned()
    };
    let mut selected: BTreeMap<(String, String), alabama_election_night::Contest> = BTreeMap::new();
    for (id, contest) in entries {
        let key = (id, contest.county.clone());
        let replace = selected.get(&key).is_none_or(|old| {
            rank(&contest) < rank(old)
                || (rank(&contest) == rank(old) && observed(&contest) > observed(old))
        });
        if replace {
            selected.insert(key, contest);
        }
    }
    selected.into_iter().map(|((id, _), c)| (id, c)).collect()
}
fn compose(root: &PathBuf, sources: &[Source]) -> Result<Value> {
    let c = db(root)?;
    let mut reports = Vec::new();
    let mut contests = Vec::new();
    let mut statewide_reports = Vec::new();
    let mut latest = None;
    let current_map = load_json(root.join("data/current-geography.json")).unwrap_or(Value::Null);
    let mut current_map_compatible = false;
    for source in sources {
        let result = c.query_row(
            "SELECT batch,fetched_at,attempted_at,error FROM source_state WHERE source=?1",
            [&source.id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        );
        let (batch, fetched, attempted, error) = result.unwrap_or((None, None, None, None));
        let mut published = None;
        if let Some(batch) = batch {
            let b: Batch = serde_json::from_str(&batch)?;
            if source.id == "madison" {
                current_map_compatible = b.map_id.as_ref().is_some_and(|id| {
                    current_map["metadata"]["source_election_id"].as_str()
                        == source.election_id.as_deref()
                        && current_map["metadata"]["source_url"]
                            .as_str()
                            .is_some_and(|url| url.ends_with(&format!("/{id}")))
                });
            }
            if source.id == "madison" {
                current_map_compatible = current_map_compatible
                    && b.map_sha256.as_deref() == current_map["metadata"]["source_sha256"].as_str()
                    && b.map_sha256.is_some();
            }
            published = b.published_at.clone();
            if source.format == "gray_statewide" {
                statewide_reports.extend(b.contests);
            } else {
                contests.extend(b.contests);
            }
        }
        if fetched > latest {
            latest = fetched.clone();
        }
        let stale = fetched
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .is_some_and(|t| Utc::now().signed_duration_since(t).num_seconds() > 600);
        let relay = c.query_row("SELECT observed_at,received_at FROM relay_state WHERE source=?1", [&source.id], |r| Ok(json!({"observed_at":r.get::<_,String>(0)?,"received_at":r.get::<_,String>(1)?}))).ok().filter(|r| r["observed_at"].as_str() == fetched.as_deref());
        reports.push(json!({"source":source,"fetched_at":fetched,"attempted_at":attempted,"published_at":published,"error":error,"stale":stale,"relay":relay}));
    }
    let ballot = load_json(root.join("data/ballot.json")).unwrap_or(json!({"contests":[]}));
    let mut races: BTreeMap<String, Value> = BTreeMap::new();
    let definitions = ballot
        .get("contests")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for mut r in requested()
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .chain(definitions)
    {
        let id = r["id"]
            .as_str()
            .context("Ballot race has no ID")?
            .to_owned();
        r["total"] = Value::Null;
        r["units"] = json!([]);
        r["contributions"] = json!([]);
        r["status"] = json!("Awaiting results");
        if r.get("candidates").is_none() {
            r["candidates"] = json!([]);
        }
        for cand in r["candidates"]
            .as_array_mut()
            .context("Bad ballot candidates")?
        {
            cand["votes"] = Value::Null;
        }
        races.insert(id, r);
    }
    let mut entries = Vec::new();
    for contest in contests {
        let id = sources
            .iter()
            .find(|s| s.id == contest.source_id)
            .and_then(|s| s.contest_map.get(&contest.provider_id))
            .cloned()
            .unwrap_or_else(|| {
                if contest.scope != "madison" {
                    contest.scope.clone()
                } else {
                    contest.id.clone()
                }
            });
        // Keep the official ballot's navigation stable while preserving publisher keys.
        let id = if races.contains_key(&id) {
            id
        } else {
            ballot_match(&contest, &races).unwrap_or(id)
        };
        entries.push((id, contest));
    }
    for (id, contest) in selected_contributions(entries, sources, &reports) {
        let r=races.entry(id.clone()).or_insert_with(||json!({"id":id,"name":contest.name,"group":contest.group,"scope":contest.scope,"candidates":[],"units":[],"contributions":[],"total":null}));
        let old = r["total"].as_u64().unwrap_or(0);
        r["total"] = json!(
            old.checked_add(contest.total)
                .context("Vote total overflow")?
        );
        if old == 0 && r["contributions"].as_array().unwrap().is_empty() {
            r["candidates"] = json!([]);
        }
        let candidates = r["candidates"].as_array_mut().unwrap();
        for cand in &contest.candidates {
            if let Some(existing) = candidates.iter_mut().find(|v| {
                candidate_key(
                    v["name"].as_str().unwrap_or(""),
                    v["party"].as_str().unwrap_or(""),
                ) == candidate_key(&cand.name, &cand.party)
            }) {
                existing["votes"] = json!(
                    existing["votes"]
                        .as_u64()
                        .unwrap_or(0)
                        .checked_add(cand.votes)
                        .context("Vote total overflow")?
                );
            } else {
                candidates.push(serde_json::to_value(cand)?);
            }
        }
        r["status"] = json!(if r["total"].as_u64().unwrap_or(0) > 0 {
            "Preliminary results"
        } else {
            "Setup zeros; awaiting votes"
        });
        r["units"].as_array_mut().unwrap().extend(
            contest
                .units
                .iter()
                .map(|u| serde_json::to_value(u).unwrap()),
        );
        let source = sources
            .iter()
            .find(|s| s.id == contest.source_id)
            .context("Unknown contribution source")?;
        let county_id = if source.format == "civic_county" {
            source.expected["counties"]
                .as_array()
                .and_then(|cs| {
                    cs.iter()
                        .find(|c| c["name"].as_str() == Some(&contest.county))
                })
                .and_then(|c| c["fips"].as_str())
                .unwrap_or("")
        } else {
            &source.county_id
        };
        let report = reports
            .iter()
            .find(|r| r["source"]["id"].as_str() == Some(&contest.source_id));
        r["contributions"].as_array_mut().unwrap().push(json!({"source_id":contest.source_id,"county":contest.county,"county_id":county_id,"provider_id":contest.provider_id,"precincts_reporting":contest.precincts_reporting,"precincts_total":contest.precincts_total,"estimated_reporting":contest.estimated_reporting,"reporting_complete":contest.reporting_complete,"reporting_basis":contest.reporting_basis,"precinct_detail":contest.precinct_detail,"detail_note":contest.detail_note,"total":contest.total,"candidates":contest.candidates,"granularity":if contest.precinct_detail{"precinct"}else{"county"},"publisher":if source.format=="civic_county"{"civicAPI"}else{"Official county source"},"url":source.endpoint.as_ref().unwrap_or(&source.url),"published_at":report.map(|r|&r["published_at"]),"fetched_at":report.map(|r|&r["fetched_at"]),"write_ins_supplied":source.format!="civic_county","write_ins_applicable":!contest.candidates.iter().all(|c|c.party=="YES"||c.party=="NO")}));
    }
    for r in races.values_mut() {
        r["county_results"] = r["contributions"].clone();
    }
    for contest in statewide_reports {
        if let Some(race) = races.get_mut(&contest.scope) {
            let source = sources
                .iter()
                .find(|s| s.id == contest.source_id)
                .context("Unknown statewide publisher")?;
            let report = reports
                .iter()
                .find(|r| r["source"]["id"] == contest.source_id)
                .context("Missing statewide source health")?;
            let county_ready = race["county_results"].as_array().is_some_and(|cs| {
                cs.iter().any(|c| {
                    c["source_id"] == "civic-statewide-us-senate"
                        && c["total"].as_u64().is_some_and(|n| n > 0)
                })
            }) && reports.iter().any(|r| {
                r["source"]["id"] == "civic-statewide-us-senate"
                    && r["error"].is_null()
                    && r["stale"] == false
            });
            race["statewide_report"] = json!({"source_id":source.id,"publisher":"WAFF / Gray","preferred":!county_ready,"url":source.url,"granularity":"statewide","candidates":contest.candidates,"total":contest.total,"published_at":report["published_at"],"fetched_at":report["fetched_at"],"error":report["error"],"stale":report["stale"],"write_ins_supplied":false,"reporting_complete":null});
        }
    }
    let mut history = Vec::new();
    let mut stmt =
        c.prepare("SELECT source,at,version,payload FROM history ORDER BY id DESC LIMIT 100")?;
    for row in stmt.query_map([],|r|Ok(json!({"source_id":r.get::<_,String>(0)?,"at":r.get::<_,String>(1)?,"version":r.get::<_,String>(2)?,"changes":serde_json::from_str::<Value>(&r.get::<_,String>(3)?).unwrap_or(Value::Null)})))? {history.push(row?);}
    let county_notes = load_json(root.join("research/county-sources.json")).unwrap_or(json!([]));
    let al02_notes = load_json(root.join("research/al02-sources.json")).unwrap_or(Value::Null);
    Ok(
        json!({"election":ELECTION,"prepared_at":now(),"latest_observation":latest,"sources":reports,"races":races.into_values().collect::<Vec<_>>(),"history":history,"ballot":ballot.get("metadata").or(ballot.get("source")),"coverage_research":county_notes,"al02_research":{"checked_at":al02_notes["checked_at"],"counties":al02_notes["official_precinct_checks"]},"current_map_compatible":current_map_compatible,"map_matches":load_json(root.join("data/crosswalk.json")).unwrap_or(json!({"matches":[]}))}),
    )
}
async fn api(State(a): State<App>) -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "no-store")],
        axum::Json(a.cache.read().await.clone()),
    )
}
async fn health(State(a): State<App>) -> impl IntoResponse {
    let d = a.cache.read().await;
    axum::Json(
        json!({"ok":true,"election":ELECTION,"prepared_at":d["prepared_at"],"sources":d["sources"]}),
    )
}
fn cell(v: &str) -> String {
    format!("\"{}\"", v.replace('"', "\"\""))
}
fn precinct_csv(d: &Value) -> String {
    let mut csv = String::from(
        "election,race,source,county,reporting_unit_id,unit,administrative,candidate,party,votes,observed_at\r\n",
    );
    for r in d["races"].as_array().unwrap() {
        for u in r["units"].as_array().unwrap() {
            let source_id = u["id"]
                .as_str()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("");
            let observed = d["sources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["source"]["id"].as_str() == Some(source_id))
                .and_then(|s| s["fetched_at"].as_str())
                .unwrap_or("");
            for c in u["candidates"].as_array().unwrap() {
                let vals = [
                    ELECTION,
                    r["name"].as_str().unwrap_or(""),
                    source_id,
                    u["county"].as_str().unwrap_or(""),
                    u["id"].as_str().unwrap_or(""),
                    u["name"].as_str().unwrap_or(""),
                    if u["administrative"] == true {
                        "true"
                    } else {
                        "false"
                    },
                    c["name"].as_str().unwrap_or(""),
                    c["party"].as_str().unwrap_or(""),
                    &c["votes"].to_string(),
                    observed,
                ];
                csv.push_str(&vals.into_iter().map(cell).collect::<Vec<_>>().join(","));
                csv.push_str("\r\n");
            }
        }
    }
    csv
}
async fn export(State(a): State<App>) -> impl IntoResponse {
    let d = a.cache.read().await;
    let csv = precinct_csv(&d);
    (
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=alabama-2026-precinct-returns.csv",
            ),
        ],
        csv,
    )
}
fn copy_site_files(source: &std::path::Path, target: &std::path::Path) -> Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        ensure!(!kind.is_symlink(), "Static export rejects symlinks");
        if kind.is_dir() {
            copy_site_files(&entry.path(), &target.join(entry.file_name()))?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target.join(entry.file_name()))?;
        }
    }
    Ok(())
}
fn public_provenance(value: &mut Value) {
    match value {
        Value::String(s) if s.starts_with("/home/") => {
            if let Some((_, rest)) = s.split_once("/Workspace/Projects/") {
                *s = format!("source-projects/{rest}");
            } else {
                *s = std::path::Path::new(s)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
            }
        }
        Value::Array(items) => items.iter_mut().for_each(public_provenance),
        Value::Object(items) => items.values_mut().for_each(public_provenance),
        _ => {}
    }
}
fn export_site(root: &PathBuf, sources: &[Source], output: &std::path::Path) -> Result<()> {
    let output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()?.join(output)
    };
    let root = fs::canonicalize(root)?;
    let output = if output.exists() {
        fs::canonicalize(output)?
    } else {
        fs::canonicalize(
            output
                .parent()
                .context("Export output needs an existing parent")?,
        )?
        .join(
            output
                .file_name()
                .context("Missing output directory name")?,
        )
    };
    ensure!(
        !output.starts_with(root.join("web"))
            && !output.starts_with(root.join("data"))
            && !output.starts_with(root.join("runtime")),
        "Export cannot overwrite source assets or runtime"
    );
    ensure!(
        !output.exists()
            || output.read_dir()?.next().is_none()
            || output.join(".nojekyll").is_file(),
        "Export directory contains unrelated files"
    );
    ensure!(
        output != root && !root.starts_with(&output),
        "Export cannot overwrite the project or its parent"
    );
    // Only the frontend, prepared public reference assets and accepted observed
    // snapshots are published. Runtime, research captures and credentials stay out.
    copy_site_files(&root.join("web"), &output)?;
    copy_site_files(&root.join("data"), &output.join("data"))?;
    for entry in fs::read_dir(output.join("data"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|s| s == "json") {
            let mut value = load_json(path.clone())?;
            public_provenance(&mut value);
            fs::write(path, serde_json::to_vec(&value)?)?;
        }
    }
    let html = fs::read_to_string(output.join("index.html"))?.replace(
        "<html lang=\"en\">",
        "<html lang=\"en\" data-snapshot=\"true\">",
    );
    fs::write(output.join("index.html"), html)?;
    fs::write(output.join(".nojekyll"), "")?;
    let mut state = compose(&root, sources)?;
    public_provenance(&mut state);
    fs::create_dir_all(output.join("api"))?;
    fs::write(output.join("api/state.json"), serde_json::to_vec(&state)?)?;
    fs::write(output.join("api/export.csv"), precinct_csv(&state))?;
    println!("Static dashboard exported to {}", output.display());
    Ok(())
}

async fn raw(
    client: &reqwest::Client,
    root: &PathBuf,
    source: &Source,
    url: &str,
) -> Result<Vec<u8>> {
    let res = client
        .get(url)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .send()
        .await?
        .error_for_status()?;
    ensure!(
        res.content_length().unwrap_or(0) <= 20_000_000,
        "Source exceeds size limit"
    );
    let body = res.bytes().await?;
    ensure!(body.len() <= 20_000_000, "Source exceeds size limit");
    let hash = format!("{:x}", Sha256::digest(&body));
    let name = format!("runtime/raw/{}-{hash}.body", source.id);
    fs::write(root.join(&name), &body)?;
    db(root)?.execute(
        "INSERT INTO evidence(source,at,url,sha256,path) VALUES(?1,?2,?3,?4,?5)",
        params![source.id, now(), url, hash, name],
    )?;
    Ok(body.to_vec())
}
// Follow only explicitly published Clarity links, never enumerate or guess election IDs.
fn clarity_links(html: &str, county: &str) -> Vec<(String, String)> {
    let prefix = format!("https://results.enr.clarityelections.com/AL/{county}/");
    let mut found = Vec::new();
    for rest in html.split(&prefix).skip(1) {
        let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !id.is_empty() && rest[id.len()..].starts_with('/') {
            let pair = (format!("{prefix}{id}"), id);
            if !found.contains(&pair) {
                found.push(pair);
            }
        }
    }
    found.truncate(3);
    found
}
async fn collect_one(client: &reqwest::Client, root: &PathBuf, source: &Source) -> Result<Batch> {
    if source.format == "gray_statewide" {
        let menu: Value = serde_json::from_slice(
            &raw(
                client,
                root,
                source,
                source.expected["menu_url"]
                    .as_str()
                    .context("No publisher menu")?,
            )
            .await?,
        )?;
        let data: Value = serde_json::from_slice(
            &raw(
                client,
                root,
                source,
                source
                    .endpoint
                    .as_deref()
                    .context("No publisher endpoint")?,
            )
            .await?,
        )?;
        let batch = alabama_election_night::gray::parse(source, &menu, &data)?;
        ensure_current_publication(root, source, &batch)?;
        return Ok(batch);
    }
    if source.format == "civic_county" {
        let endpoint = source
            .endpoint
            .as_ref()
            .context("Missing civicAPI endpoint")?;
        let payload: Value = serde_json::from_slice(&raw(client, root, source, endpoint).await?)?;
        let batch = alabama_election_night::civic::parse(source, &payload)?;
        ensure_current_publication(root, source, &batch)?;
        return Ok(batch);
    }
    if source.id == "morgan" {
        let authority = String::from_utf8(raw(client, root, source, &source.url).await?)?;
        alabama_election_night::morgan::validate_authority(&authority)?;
        let report = String::from_utf8(
            raw(
                client,
                root,
                source,
                "http://www.morgancountyprobate.com/temp/el30.htm",
            )
            .await?,
        )?;
        return alabama_election_night::morgan::parse(source, &authority, &report);
    }

    let candidates = if let (Some(endpoint), Some(id)) = (&source.endpoint, &source.election_id) {
        vec![(endpoint.trim_end_matches('/').to_owned(), id.clone())]
    } else {
        let html = raw(client, root, source, &source.url).await?;
        let links = clarity_links(&String::from_utf8_lossy(&html), &source.county);
        ensure!(
            !links.is_empty(),
            "No current machine-readable feed linked from official entry page"
        );
        links
    };
    let mut errors = Vec::new();
    for (endpoint, id) in candidates {
        let mut s = source.clone();
        s.election_id = Some(id);
        let attempt: Result<Batch> = async {
            let version = String::from_utf8(
                raw(client, root, &s, &format!("{endpoint}/current_ver.txt")).await?,
            )?
            .trim()
            .to_owned();
            ensure!(
                version.chars().all(|c| c.is_ascii_digit()) && !version.is_empty(),
                "Invalid version endpoint response: {:?}",
                version.chars().take(96).collect::<String>()
            );
            let base = format!("{endpoint}/{version}/json");
            let settings: Value = serde_json::from_slice(
                &raw(
                    client,
                    root,
                    &s,
                    &format!("{base}/en/electionsettings.json"),
                )
                .await?,
            )?;
            alabama_election_night::validate_settings(&s, &settings)?;
            let summary: Value = serde_json::from_slice(
                &raw(client, root, &s, &format!("{base}/en/summary.json")).await?,
            )?;
            let detail_result: Result<Value> =
                match raw(client, root, &s, &format!("{base}/details.json")).await {
                    Ok(body) => {
                        serde_json::from_slice(&body).context("Invalid precinct detail JSON")
                    }
                    Err(e) => Err(e),
                };
            let detail_error = detail_result.as_ref().err().map(ToString::to_string);
            let details = detail_result.unwrap_or(json!({"Contests":[]}));
            let mut batch = parse(&s, &version, &settings, &summary, &details)?;
            let status_result: Result<()> = async {
                let status: Value = serde_json::from_slice(
                    &raw(client, root, &s, &format!("{base}/status.json")).await?,
                )?;
                alabama_election_night::apply_precinct_status(&mut batch, &status)
            }
            .await;
            if let Err(error) = status_result {
                for unit in batch.contests.iter_mut().flat_map(|c| &mut c.units) {
                    unit.reporting_complete = None;
                    unit.reporting_basis =
                        Some(format!("Precinct completion unavailable: {error}"));
                }
            }

            if let Some(error) = detail_error {
                for c in &mut batch.contests {
                    c.detail_note = Some(format!(
                        "County summary retained; precinct detail unavailable: {error}"
                    ));
                }
            }
            // The filename can survive a publication revision; compare actual map bytes
            // before using the prepared geographic joins with new returns.
            if s.id == "madison" {
                if let Some(map) = &batch.map_id {
                    if map.len() < 128
                        && map.ends_with(".json")
                        && map
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
                    {
                        if let Ok(body) = raw(client, root, &s, &format!("{base}/{map}")).await {
                            batch.map_sha256 = Some(format!("{:x}", Sha256::digest(&body)));
                        }
                    }
                }
            }
            Ok(batch)
        }
        .await;
        match attempt {
            Ok(batch) => return Ok(batch),
            Err(e) => errors.push(e.to_string()),
        }
    }
    anyhow::bail!("Published feed candidates rejected: {}", errors.join("; "))
}
fn ensure_current_publication(root: &PathBuf, source: &Source, batch: &Batch) -> Result<()> {
    if !matches!(source.format.as_str(), "civic_county" | "gray_statewide") {
        return Ok(());
    }
    let prior = db(root)?
        .query_row(
            "SELECT batch FROM source_state WHERE source=?1",
            [&source.id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten();
    if let Some(prior) = prior {
        let prior: Batch = serde_json::from_str(&prior)?;
        if let (Some(before), Some(after)) = (&prior.published_at, &batch.published_at) {
            let before = chrono::DateTime::parse_from_rfc3339(before)?;
            let after = chrono::DateTime::parse_from_rfc3339(after)?;
            ensure!(
                after >= before,
                "Publisher returned an older publication than the accepted county results ({after} < {before})"
            );
        }
    }
    Ok(())
}
fn retain(root: &PathBuf, source: &str, result: Result<Batch>) -> Result<()> {
    retain_at(root, source, result, &now())
}
fn retain_at(root: &PathBuf, source: &str, result: Result<Batch>, at: &str) -> Result<()> {
    let mut c = db(root)?;
    let tx = c.transaction()?;
    match result {
        Ok(batch) => {
            let serialized = serde_json::to_string(&batch)?;
            let previous = tx
                .query_row(
                    "SELECT batch FROM source_state WHERE source=?1",
                    [source],
                    |r| r.get::<_, Option<String>>(0),
                )
                .ok()
                .flatten();
            let prior = previous.and_then(|s| serde_json::from_str::<Batch>(&s).ok());
            let changed = prior.as_ref().is_none_or(|p| {
                p.version != batch.version
                    || serde_json::to_value(&p.contests).ok()
                        != serde_json::to_value(&batch.contests).ok()
            });
            if changed {
                let mut changes = Vec::new();
                for race in &batch.contests {
                    let prev = prior
                        .as_ref()
                        .and_then(|p| p.contests.iter().find(|r| r.id == race.id));
                    let old = prev.map(|r| r.total);
                    if old != Some(race.total)
                        || prev.is_some_and(|p| p.candidates != race.candidates)
                    {
                        changes.push(json!({"race":race.name,"county":race.county,"provider_id":race.provider_id,"scope":race.scope,"before":old,"after":race.total,"correction":old.is_some_and(|n|n>race.total) || prev.is_some_and(|p|race.candidates.iter().any(|c|p.candidates.iter().find(|x|candidate_key(&x.name,&x.party)==candidate_key(&c.name,&c.party)).is_some_and(|old|old.votes>c.votes)))}));
                    }
                }
                tx.execute(
                    "INSERT INTO history(source,at,version,payload) VALUES(?1,?2,?3,?4)",
                    params![source, at, batch.version, serde_json::to_string(&changes)?],
                )?;
            }
            tx.execute("INSERT INTO source_state(source,batch,fetched_at,attempted_at,error) VALUES(?1,?2,?3,?3,NULL) ON CONFLICT(source) DO UPDATE SET batch=excluded.batch,fetched_at=excluded.fetched_at,attempted_at=excluded.attempted_at,error=NULL",params![source,serialized,at])?;
            tx.execute("DELETE FROM relay_state WHERE source=?1", [source])?;
        }
        Err(e) => {
            tx.execute("INSERT INTO source_state(source,attempted_at,error) VALUES(?1,?2,?3) ON CONFLICT(source) DO UPDATE SET attempted_at=excluded.attempted_at,error=excluded.error",params![source,at,e.to_string()])?;
        }
    }
    tx.commit()?;
    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|s| s == "--help") {
        println!(
            "alabama-election-night [serve|collect-once [--relay PATH] [--county-relay PATH]|export-site --output PATH|export-relay --output PATH [--county-output PATH]] [--root PATH] [--port 8052] [--no-collector]\nCentral official-source collection; browsers read cached results. Default root: project directory."
        );
        return Ok(());
    }
    let option = |name: &str| {
        args.iter()
            .position(|s| s == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let root = PathBuf::from(option("--root").unwrap_or_else(|| env!("CARGO_MANIFEST_DIR").into()));
    fs::create_dir_all(root.join("runtime/raw"))?;
    let sources: Vec<Source> = serde_json::from_slice(&fs::read(root.join("data/sources.json"))?)?;
    if args.get(1).is_some_and(|s| s == "export-relay") {
        let output =
            PathBuf::from(option("--output").context("export-relay requires --output PATH")?);
        relay::export(&root, &sources, &output)?;
        if let Some(path) = option("--county-output") {
            relay::export_counties(&root, &sources, &PathBuf::from(path))?;
        }
        return Ok(());
    }
    if args.get(1).is_some_and(|s| s == "export-site") {
        let output =
            PathBuf::from(option("--output").context("export-site requires --output PATH")?);
        return export_site(&root, &sources, &output);
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .user_agent("Alabama-election-night/0.1 (public official results; central cache)")
        .build()?;
    let collect = !args.iter().any(|s| s == "--no-collector");
    let _lock = if collect {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("runtime/collector.lock"))?;
        file.try_lock_exclusive()
            .context("Another collector owns this runtime")?;
        Some(file)
    } else {
        None
    };
    if args.get(1).is_some_and(|s| s == "collect-once") {
        if let Some(path) = option("--county-relay") {
            match relay::import_counties(&root, &sources, &PathBuf::from(path)) {
                Ok(count) => println!(
                    "Validated {count} local county source observations; original times retained."
                ),
                Err(e) => eprintln!(
                    "County relay unavailable/rejected; independent collection continues: {e:#}"
                ),
            }
        }
        let relayed = option("--relay").is_some_and(|path| {
            match relay::import(&root, &sources, &PathBuf::from(path)) {
                Ok(()) => {
                    println!("Validated local Madison relay; original observation times retained.");
                    true
                }
                Err(e) => {
                    eprintln!(
                        "Madison relay unavailable/rejected; using normal source collection: {e:#}"
                    );
                    false
                }
            }
        });
        for source in sources.iter().filter(|s| s.enabled) {
            if relayed && source.id == "madison" {
                continue;
            }
            retain(&root, &source.id, collect_one(&client, &root, source).await)?;
        }
        println!(
            "Collection attempt recorded; check /api/state or runtime/results.sqlite for source outcomes."
        );
        return Ok(());
    }
    let app = App {
        cache: Arc::new(RwLock::new(compose(&root, &sources)?)),
        root: Arc::new(root),
    };
    if collect {
        let a = app.clone();
        tokio::spawn(async move {
            let mut due = BTreeMap::new();
            loop {
                for source in sources.iter().filter(|s| s.enabled) {
                    let time = std::time::Instant::now();
                    if due.get(&source.id).is_none_or(|d| time >= *d) {
                        let result = collect_one(&client, &a.root, source).await;
                        if let Err(e) = retain(&a.root, &source.id, result) {
                            eprintln!("Retaining {}: {e}", source.id);
                        }
                        let interval = if source.endpoint.is_some() {
                            source.interval_seconds.max(120)
                        } else {
                            600
                        };
                        due.insert(
                            source.id.clone(),
                            std::time::Instant::now() + Duration::from_secs(interval),
                        );
                        if let Ok(data) = compose(&a.root, &sources) {
                            *a.cache.write().await = data;
                        }
                    }
                }
                // Recompute freshness even while a source is unavailable.
                if let Ok(data) = compose(&a.root, &sources) {
                    *a.cache.write().await = data;
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }
    let router = Router::new()
        .route("/api/state", get(api))
        .route("/api/health", get(health))
        .route("/api/export.csv", get(export))
        .nest_service("/data", ServeDir::new(app.root.join("data")))
        .fallback_service(ServeDir::new(app.root.join("web")).not_found_service(
            tower_http::services::ServeFile::new(app.root.join("web/index.html")),
        ))
        .with_state(app)
        .layer(CompressionLayer::new());
    let port = option("--port").unwrap_or("8052".into()).parse::<u16>()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!("Alabama Election Night: http://127.0.0.1:{port}");
    axum::serve(listener, router).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn statewide_publisher_report_never_enters_county_or_precinct_totals() {
        let sources: Vec<Source> =
            serde_json::from_str(include_str!("../data/sources.json")).unwrap();
        let publisher = sources
            .iter()
            .find(|s| s.format == "gray_statewide")
            .unwrap();
        let county = sources
            .iter()
            .find(|s| s.id == "civic-statewide-us-senate")
            .unwrap();
        let batch = alabama_election_night::gray::parse(
            publisher,
            &serde_json::from_str(include_str!("../tests/fixtures/waff-menu-2026.json")).unwrap(),
            &serde_json::from_str(include_str!("../tests/fixtures/waff-senate-2026-live.json"))
                .unwrap(),
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!(
            "senate-isolation-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(root.join("runtime")).unwrap();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::write(
            root.join("data/ballot.json"),
            include_str!("../data/ballot.json"),
        )
        .unwrap();
        retain(&root, &publisher.id, Ok(batch)).unwrap();
        let county_batch = alabama_election_night::civic::parse(
            county,
            &serde_json::from_str(include_str!(
                "../tests/fixtures/civic-statewide-us-senate-2026-setup.json"
            ))
            .unwrap(),
        )
        .unwrap();
        retain(&root, &county.id, Ok(county_batch)).unwrap();
        let data = compose(&root, &[publisher.clone(), county.clone()]).unwrap();
        let race = data["races"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "us-senate")
            .unwrap();
        assert_eq!(race["statewide_report"]["total"], 10506);
        assert_eq!(race["total"], 0);
        assert_eq!(race["county_results"].as_array().unwrap().len(), 67);
        assert!(
            race["county_results"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["total"] == 0 && c["county_id"] != "AL")
        );
        assert!(race["units"].as_array().unwrap().is_empty());
        let mut positive: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/civic-statewide-us-senate-2026-setup.json"
        ))
        .unwrap();
        positive["candidates"][0]["votes"] = 5.into();
        positive["region_results"]["autauga"]["candidates"][0]["votes"] = 5.into();
        retain(
            &root,
            &county.id,
            Ok(alabama_election_night::civic::parse(county, &positive).unwrap()),
        )
        .unwrap();
        let data = compose(&root, &[publisher.clone(), county.clone()]).unwrap();
        let race = data["races"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "us-senate")
            .unwrap();
        assert_eq!(race["total"], 5);
        assert_eq!(race["statewide_report"]["preferred"], false);

        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn static_export_is_project_relative_and_excludes_private_runtime() {
        let base = std::env::temp_dir().join(format!(
            "election-export-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = base.join("project");
        let out = base.join("site");
        fs::create_dir_all(root.join("web/assets")).unwrap();
        fs::create_dir_all(root.join("data")).unwrap();
        fs::create_dir_all(root.join("runtime")).unwrap();
        fs::write(
            root.join("web/index.html"),
            "<html lang=\"en\"><head></head></html>",
        )
        .unwrap();
        fs::write(root.join("web/assets/font.ttf"), "font fixture").unwrap();
        fs::write(
            root.join("data/reference.json"),
            r#"{"path":"/home/example/Workspace/Projects/source/data.json","votes":7}"#,
        )
        .unwrap();
        fs::write(root.join("runtime/private.txt"), "not a site asset").unwrap();
        export_site(&root, &[], &out).unwrap();
        assert!(
            fs::read_to_string(out.join("index.html"))
                .unwrap()
                .contains("data-snapshot=\"true\"")
        );
        assert!(
            out.join("api/state.json").is_file()
                && out.join("api/export.csv").is_file()
                && out.join("assets/font.ttf").is_file()
        );
        assert!(!out.join("runtime").exists());
        let reference = load_json(out.join("data/reference.json")).unwrap();
        assert_eq!(reference["votes"], 7);
        assert_eq!(reference["path"], "source-projects/source/data.json");
        assert!(export_site(&root, &[], &root).is_err());
        assert!(export_site(&root, &[], &root.join("web")).is_err());
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn discovered_links_are_published_and_county_specific() {
        let h = "https://results.enr.clarityelections.com/AL/Madison/123/web/ https://results.enr.clarityelections.com/AL/Morgan/124/";
        assert_eq!(
            clarity_links(h, "Madison"),
            vec![(
                "https://results.enr.clarityelections.com/AL/Madison/123".into(),
                "123".into()
            )]
        );
        assert!(clarity_links("123 or 124", "Madison").is_empty());
    }
    #[test]
    fn county_fallback_never_duplicates_a_healthy_precinct_contribution() {
        let sources: Vec<Source> =
            serde_json::from_str(include_str!("../data/sources.json")).unwrap();
        let official = sources.iter().find(|s| s.id == "madison").unwrap();
        let fallback = sources.iter().find(|s| s.id == "civic-cd5").unwrap();
        let official_batch = parse(
            official,
            "382270",
            &serde_json::from_str(include_str!("../research/madison-settings.json")).unwrap(),
            &serde_json::from_str(include_str!("../research/madison-summary.json")).unwrap(),
            &serde_json::from_str(include_str!("../research/madison-details.json")).unwrap(),
        )
        .unwrap();
        let mut precinct = official_batch
            .contests
            .into_iter()
            .find(|c| c.scope == "cd5")
            .unwrap();
        precinct.total = 10;
        let county_batch = alabama_election_night::civic::parse(
            fallback,
            &serde_json::from_str(include_str!("../tests/fixtures/civic-cd5-2026-setup.json"))
                .unwrap(),
        )
        .unwrap();
        let mut county = county_batch
            .contests
            .iter()
            .find(|c| c.county == "Madison")
            .unwrap()
            .clone();
        county.total = 100;
        let mut limestone = county_batch
            .contests
            .iter()
            .find(|c| c.county == "Limestone")
            .unwrap()
            .clone();
        limestone.total = 30;
        let entries = vec![
            ("cd5".into(), precinct),
            ("cd5".into(), county),
            ("cd5".into(), limestone),
        ];
        let mut reports = vec![
            json!({"source":{"id":"madison"},"error":null,"stale":false,"fetched_at":"2026-11-04T01:00:00Z"}),
            json!({"source":{"id":"civic-cd5"},"error":null,"stale":false,"fetched_at":"2026-11-04T01:01:00Z"}),
        ];
        let selected = selected_contributions(entries.clone(), &sources, &reports);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected.iter().map(|(_, c)| c.total).sum::<u64>(), 40);
        assert_eq!(
            selected
                .iter()
                .find(|(_, c)| c.county == "Madison")
                .unwrap()
                .1
                .source_id,
            "madison"
        );
        reports[0]["error"] = json!("network unavailable");
        let selected = selected_contributions(entries, &sources, &reports);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected.iter().map(|(_, c)| c.total).sum::<u64>(), 130);
        assert_eq!(
            selected
                .iter()
                .find(|(_, c)| c.county == "Madison")
                .unwrap()
                .1
                .source_id,
            "civic-cd5"
        );
        assert!(selected.iter().all(|(_, c)| c.units.is_empty()));
    }
    #[test]
    fn failure_preserves_last_good_and_records_corrections() {
        let root = std::env::temp_dir().join(format!("al-dashboard-test-{}", std::process::id()));
        fs::create_dir_all(root.join("runtime")).unwrap();
        let contest=serde_json::from_value(json!({"id":"fixture:1","source_id":"fixture","provider_id":"1","name":"Example race","group":"Local","scope":"madison","county":"Madison","candidates":[{"id":"a","name":"Example A","party":"DEM","votes":6},{"id":"b","name":"Example B","party":"REP","votes":4}],"units":[],"precincts_reporting":null,"precincts_total":null,"precinct_detail":false,"detail_note":"Summary-only test","total":10})).unwrap();
        let mut batch = Batch {
            map_id: None,
            map_sha256: None,
            source_id: "fixture".into(),
            version: "1".into(),
            published_at: None,
            contests: vec![contest],
        };
        retain(&root, "fixture", Ok(batch.clone())).unwrap();
        // A candidate can lose votes while the race total is unchanged.
        batch.version = "2".into();
        batch.contests[0].candidates[0].votes = 5;
        batch.contests[0].candidates[1].votes = 5;
        retain(&root, "fixture", Ok(batch.clone())).unwrap();
        retain(&root, "fixture", Ok(batch)).unwrap();
        retain(&root, "fixture", Err(anyhow::anyhow!("network offline"))).unwrap();
        let c = db(&root).unwrap();
        let x: (String, String) = c
            .query_row(
                "SELECT batch,error FROM source_state WHERE source='fixture'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(x.0.contains("fixture"));
        assert_eq!(x.1, "network offline");
        let count: u64 = c
            .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count, 2,
            "Unchanged revisions and failures must not create updates"
        );
        let payload: String = c
            .query_row(
                "SELECT payload FROM history ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let changes: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(changes[0]["correction"], true);
        assert_eq!(changes[0]["before"], 10);
        assert_eq!(changes[0]["after"], 10);
        drop(c);
        fs::remove_dir_all(root).unwrap();
    }
}
