//! Public source transport, never a second election-results parser.
use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_FILE: usize = 20_000_000;
const MAX_PACKAGE: u64 = 110_000_000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    url: String,
    sha256: String,
    body: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema_version: u8,
    election: String,
    source_id: String,
    version: String,
    observed_at: String,
    attempted_at: String,
    error: Option<String>,
    documents: BTreeMap<String, Document>,
}

fn source(sources: &[Source]) -> Result<&Source> {
    sources
        .iter()
        .find(|s| s.id == "madison")
        .context("Missing Madison source configuration")
}

fn date(value: &str) -> Result<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_rfc3339(value).context("Invalid relay timestamp")
}

fn document<'a>(snapshot: &'a Snapshot, key: &str, url: &str) -> Result<Option<&'a Document>> {
    let Some(doc) = snapshot.documents.get(key) else {
        return Ok(None);
    };
    ensure!(doc.url == url, "Unexpected relay source URL for {key}");
    ensure!(
        doc.body.len() <= MAX_FILE,
        "Relay document exceeds size limit"
    );
    ensure!(
        format!("{:x}", Sha256::digest(doc.body.as_bytes())) == doc.sha256,
        "Relay document hash mismatch for {key}"
    );
    Ok(Some(doc))
}

fn validate(snapshot: &Snapshot, source: &Source) -> Result<Batch> {
    ensure!(
        snapshot.schema_version == 1
            && snapshot.election == ELECTION
            && snapshot.source_id == source.id,
        "Wrong relay schema, election or source"
    );
    let observed = date(&snapshot.observed_at)?;
    let attempted = date(&snapshot.attempted_at)?;
    ensure!(
        attempted >= observed && attempted.signed_duration_since(Utc::now()).num_seconds() <= 120,
        "Relay timestamps out of order or in the future"
    );
    ensure!(
        snapshot.error.as_ref().is_none_or(|e| e.len() <= 4096),
        "Oversized relay error"
    );
    ensure!(
        !snapshot.version.is_empty()
            && snapshot.version.len() <= 32
            && snapshot.version.bytes().all(|b| b.is_ascii_digit()),
        "Invalid relay revision"
    );
    ensure!(
        snapshot.documents.keys().all(|k| matches!(
            k.as_str(),
            "settings" | "summary" | "details" | "status" | "map"
        )),
        "Unknown relay document"
    );
    let base = format!(
        "{}/{}/json",
        source
            .endpoint
            .as_ref()
            .context("No configured endpoint")?
            .trim_end_matches('/'),
        snapshot.version
    );
    let read = |key: &str, suffix: &str| -> Result<Option<Value>> {
        document(snapshot, key, &format!("{base}/{suffix}"))?
            .map(|doc| serde_json::from_str(&doc.body).context("Invalid relayed JSON"))
            .transpose()
    };
    let settings =
        read("settings", "en/electionsettings.json")?.context("Missing relay settings")?;
    let summary = read("summary", "en/summary.json")?.context("Missing relay summary")?;
    // Validate transport identity before allowing the same optional-file degradation
    // used by direct collection (e.g. a publisher's empty/unavailable detail file).
    let detail_doc = document(snapshot, "details", &format!("{base}/details.json"))?;
    let detail = detail_doc.and_then(|doc| serde_json::from_str::<Value>(&doc.body).ok());
    let mut batch = parse(
        source,
        &snapshot.version,
        &settings,
        &summary,
        detail.as_ref().unwrap_or(&json!({"Contests":[]})),
    )?;
    if detail.is_none() {
        for race in &mut batch.contests {
            race.detail_note =
                Some("County summary retained; local relay has no accepted precinct detail".into());
        }
    }
    // Missing/invalid completion metadata never implies completion, just as in direct collection.
    let status = document(snapshot, "status", &format!("{base}/status.json"))?;
    let completion = status
        .context("No relayed precinct status")
        .and_then(|doc| {
            serde_json::from_str::<Value>(&doc.body).context("Invalid relayed precinct status JSON")
        })
        .and_then(|s| alabama_election_night::apply_precinct_status(&mut batch, &s));
    if let Err(error) = completion {
        for unit in batch.contests.iter_mut().flat_map(|c| &mut c.units) {
            unit.reporting_complete = None;
            unit.reporting_basis = Some(format!("Precinct completion unavailable: {error}"));
        }
    }
    if let Some(map) = &batch.map_id {
        ensure!(
            map.len() < 128
                && map.ends_with(".json")
                && map
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'),
            "Invalid relay map identifier"
        );
        batch.map_sha256 =
            document(snapshot, "map", &format!("{base}/{map}"))?.map(|doc| doc.sha256.clone());
    } else {
        ensure!(
            !snapshot.documents.contains_key("map"),
            "Unexpected map without source identifier"
        );
    }
    Ok(batch)
}

pub(super) fn export(root: &PathBuf, sources: &[Source], output: &Path) -> Result<()> {
    let source = source(sources)?;
    let mut c = db(root)?;
    let tx = c.transaction()?;
    let (batch, observed_at, attempted_at, error): (String, String, String, Option<String>) = tx.query_row(
        "SELECT batch,fetched_at,attempted_at,error FROM source_state WHERE source=?1 AND batch IS NOT NULL", [&source.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ).context("No accepted local Madison observation")?;
    let batch: Batch = serde_json::from_str(&batch)?;
    let base = format!(
        "{}/{}/json",
        source.endpoint.as_ref().context("Missing endpoint")?,
        batch.version
    );
    let mut documents = BTreeMap::new();
    let mut paths = vec![
        ("settings", "en/electionsettings.json".to_owned()),
        ("summary", "en/summary.json".into()),
        ("details", "details.json".into()),
        ("status", "status.json".into()),
    ];
    if let Some(map) = &batch.map_id {
        paths.push(("map", map.clone()));
    }
    let earliest = (date(&observed_at)? - chrono::Duration::minutes(5)).to_rfc3339();
    for (key, suffix) in paths {
        let url = format!("{base}/{suffix}");
        let evidence = tx.query_row("SELECT sha256,path FROM evidence WHERE source=?1 AND url=?2 AND at<=?3 AND at>=?4 ORDER BY id DESC LIMIT 1", params![source.id, url, observed_at, earliest], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).ok();
        if let Some((sha256, path)) = evidence {
            let raw_dir = root.join("runtime/raw").canonicalize()?;
            let file = root.join(path).canonicalize()?;
            ensure!(
                file.starts_with(raw_dir) && fs::metadata(&file)?.len() <= MAX_FILE as u64,
                "Unsafe raw evidence path or size"
            );
            documents.insert(
                key.into(),
                Document {
                    url,
                    sha256,
                    body: fs::read_to_string(file)?,
                },
            );
        }
    }
    let snapshot = Snapshot {
        schema_version: 1,
        election: ELECTION.into(),
        source_id: source.id.clone(),
        version: batch.version,
        observed_at,
        attempted_at,
        error,
        documents,
    };
    let verified = validate(&snapshot, source)?;
    // Never transport a less detailed package than the accepted local observation.
    ensure!(
        verified
            .contests
            .iter()
            .map(|c| c.units.len())
            .sum::<usize>()
            == batch.contests.iter().map(|c| c.units.len()).sum::<usize>(),
        "Relay lacks accepted precinct evidence"
    );
    ensure!(
        verified.map_sha256 == batch.map_sha256,
        "Relay lacks accepted map evidence"
    );
    let bytes = serde_json::to_vec(&snapshot)?;
    ensure!(
        bytes.len() as u64 <= MAX_PACKAGE,
        "Relay package exceeds size limit"
    );
    let temporary = output.with_extension("json.tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, output)?;
    Ok(())
}

pub(super) fn import(root: &PathBuf, sources: &[Source], input: &Path) -> Result<()> {
    ensure!(
        fs::metadata(input)?.len() <= MAX_PACKAGE,
        "Relay package exceeds size limit"
    );
    let snapshot: Snapshot = serde_json::from_slice(&fs::read(input)?)?;
    let source = source(sources)?;
    let batch = validate(&snapshot, source)?;
    let c = db(root)?;
    let previous = c
        .query_row(
            "SELECT fetched_at,attempted_at FROM source_state WHERE source=?1",
            [&source.id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .ok();
    let prior_observed = previous
        .as_ref()
        .and_then(|p| p.0.as_ref())
        .map(|s| date(s))
        .transpose()?;
    let observed = date(&snapshot.observed_at)?;
    if prior_observed.is_some_and(|p| p > observed) {
        println!("Ignored older Madison relay; newer accepted data retained.");
        return Ok(());
    }
    if prior_observed.is_none_or(|p| p < observed) {
        for doc in snapshot.documents.values() {
            let path = format!("runtime/raw/{}-{}.body", source.id, doc.sha256);
            fs::write(root.join(&path), &doc.body)?;
            c.execute(
                "INSERT INTO evidence(source,at,url,sha256,path) VALUES(?1,?2,?3,?4,?5)",
                params![source.id, snapshot.observed_at, doc.url, doc.sha256, path],
            )?;
        }
        retain_at(root, &source.id, Ok(batch), &snapshot.observed_at)?;
    }
    let prior_attempt = previous
        .as_ref()
        .and_then(|p| p.1.as_ref())
        .map(|s| date(s))
        .transpose()?;
    if prior_attempt.is_none_or(|p| date(&snapshot.attempted_at).is_ok_and(|a| a >= p)) {
        c.execute(
            "UPDATE source_state SET attempted_at=?1,error=?2 WHERE source=?3",
            params![snapshot.attempted_at, snapshot.error, source.id],
        )?;
    }
    c.execute("INSERT INTO relay_state(source,observed_at,received_at) VALUES(?1,?2,?3) ON CONFLICT(source) DO UPDATE SET observed_at=excluded.observed_at,received_at=excluded.received_at", params![source.id,snapshot.observed_at,now()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Vec<Source>, Snapshot) {
        let sources: Vec<Source> =
            serde_json::from_str(include_str!("../data/sources.json")).unwrap();
        let s = source(&sources).unwrap();
        let base = format!("{}/382270/json", s.endpoint.as_ref().unwrap());
        let mut documents = BTreeMap::new();
        for (key, suffix, body) in [
            (
                "settings",
                "en/electionsettings.json",
                include_str!("../research/madison-settings.json"),
            ),
            (
                "summary",
                "en/summary.json",
                include_str!("../research/madison-summary.json"),
            ),
            (
                "details",
                "details.json",
                include_str!("../research/madison-details.json"),
            ),
            (
                "status",
                "status.json",
                include_str!("../tests/fixtures/madison-status-2026-setup.json"),
            ),
            (
                "map",
                "78711aa6-2e06-4898-914d-beee88029fcc.json",
                include_str!("../research/madison-live-map.raw.json"),
            ),
        ] {
            documents.insert(
                key.into(),
                Document {
                    url: format!("{base}/{suffix}"),
                    sha256: format!("{:x}", Sha256::digest(body.as_bytes())),
                    body: body.into(),
                },
            );
        }
        let observed_at = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        (
            sources,
            Snapshot {
                schema_version: 1,
                election: ELECTION.into(),
                source_id: "madison".into(),
                version: "382270".into(),
                attempted_at: observed_at.clone(),
                observed_at,
                error: None,
                documents,
            },
        )
    }

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "election-relay-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(root.join("runtime/raw")).unwrap();
        root
    }

    fn write(root: &Path, snapshot: &Snapshot) -> PathBuf {
        let path = root.join("runtime/relay.json");
        fs::write(&path, serde_json::to_vec(snapshot).unwrap()).unwrap();
        path
    }

    #[test]
    fn real_precinct_source_revalidated_and_transport_tampering_rejected() {
        let (sources, snapshot) = fixture();
        let s = source(&sources).unwrap();
        let batch = validate(&snapshot, s).unwrap();
        assert_eq!(batch.contests.len(), 49);
        assert!(
            batch
                .contests
                .iter()
                .flat_map(|c| &c.units)
                .all(|u| u.reporting_complete != Some(true))
        );
        assert_eq!(
            batch.contests.iter().map(|c| c.units.len()).sum::<usize>(),
            3012
        );
        assert!(
            batch
                .contests
                .iter()
                .flat_map(|c| &c.units)
                .any(|u| u.reporting_complete == Some(false))
        );
        assert_eq!(
            batch.map_sha256.as_deref(),
            Some("7b6cddb163d7518aac9f107fea083d9f6889810c56b7f36d6c888d81fc86ce2d")
        );
        let mut bad = snapshot.clone();
        bad.documents.get_mut("summary").unwrap().body.push(' ');
        assert!(validate(&bad, s).is_err());
        let mut bad = snapshot.clone();
        bad.documents.get_mut("settings").unwrap().url = "https://example.com/settings.json".into();
        assert!(validate(&bad, s).is_err());
        let mut bad = snapshot.clone();
        bad.election = "2024-11-05".into();
        assert!(validate(&bad, s).is_err());
        let mut bad = snapshot.clone();
        bad.attempted_at = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        assert!(validate(&bad, s).is_err());
        let mut bad = snapshot.clone();
        bad.version = "../../main".into();
        assert!(validate(&bad, s).is_err());
        let mut bad = snapshot.clone();
        let doc = bad.documents.get_mut("settings").unwrap();
        doc.body = doc.body.replace("11/3/2026", "11/5/2024");
        doc.sha256 = format!("{:x}", Sha256::digest(doc.body.as_bytes()));
        assert!(
            validate(&bad, s).is_err(),
            "Election parser must run even when hashes match"
        );
    }

    #[test]
    fn roundtrip_preserves_observation_age_errors_and_prevents_rollback() {
        let (sources, mut snapshot) = fixture();
        let root = temp_root();
        let input = write(&root, &snapshot);
        import(&root, &sources, &input).unwrap();
        let c = db(&root).unwrap();
        let stamps = || {
            c.query_row(
                "SELECT fetched_at,attempted_at,error FROM source_state WHERE source='madison'",
                [],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .unwrap()
        };
        assert_eq!(
            stamps(),
            (
                snapshot.observed_at.clone(),
                snapshot.attempted_at.clone(),
                None
            )
        );
        let history = || {
            c.query_row("SELECT COUNT(*) FROM history", [], |r| r.get::<_, i64>(0))
                .unwrap()
        };
        assert_eq!(history(), 1);
        import(&root, &sources, &input).unwrap();
        assert_eq!(history(), 1);
        assert_eq!(
            stamps().0,
            snapshot.observed_at,
            "Re-import must not refresh the observation timestamp"
        );
        snapshot.attempted_at = (Utc::now() - chrono::Duration::minutes(2)).to_rfc3339();
        snapshot.error = Some("Local source offline".into());
        import(&root, &sources, &write(&root, &snapshot)).unwrap();
        assert_eq!(
            stamps(),
            (
                snapshot.observed_at.clone(),
                snapshot.attempted_at.clone(),
                snapshot.error.clone()
            )
        );
        let out = root.join("runtime/export.json");
        export(&root, &sources, &out).unwrap();
        let exported: Snapshot = serde_json::from_slice(&fs::read(out).unwrap()).unwrap();
        assert_eq!(exported.observed_at, snapshot.observed_at);
        assert_eq!(exported.error, snapshot.error);
        let original = stamps();
        let mut older = snapshot.clone();
        older.observed_at =
            (date(&snapshot.observed_at).unwrap() - chrono::Duration::minutes(1)).to_rfc3339();
        older.error = None;
        import(&root, &sources, &write(&root, &older)).unwrap();
        assert_eq!(stamps(), original);
        let mut bad = snapshot.clone();
        bad.documents.get_mut("summary").unwrap().sha256 = "bad".into();
        assert!(import(&root, &sources, &write(&root, &bad)).is_err());
        assert_eq!(stamps(), original);
        // Source evidence is still required to republish an accepted precinct batch.
        c.execute("DELETE FROM evidence WHERE url LIKE '%details.json'", [])
            .unwrap();
        assert!(export(&root, &sources, &root.join("runtime/missing.json")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unavailable_optional_source_files_keep_valid_votes_without_claiming_completion() {
        let (sources, mut snapshot) = fixture();
        let s = source(&sources).unwrap();
        let status = snapshot.documents.get_mut("status").unwrap();
        status.body.clear();
        status.sha256 = format!("{:x}", Sha256::digest(status.body.as_bytes()));
        let batch = validate(&snapshot, s).unwrap();
        assert_eq!(
            batch.contests.iter().map(|c| c.units.len()).sum::<usize>(),
            3012
        );
        assert!(
            batch
                .contests
                .iter()
                .flat_map(|c| &c.units)
                .all(|u| u.reporting_complete.is_none())
        );
        let detail = snapshot.documents.get_mut("details").unwrap();
        detail.body = "temporarily unavailable".into();
        detail.sha256 = format!("{:x}", Sha256::digest(detail.body.as_bytes()));
        let batch = validate(&snapshot, s).unwrap();
        assert_eq!(batch.contests.len(), 49);
        assert!(
            batch
                .contests
                .iter()
                .all(|c| c.units.is_empty() && !c.precinct_detail && c.detail_note.is_some())
        );
        // An altered transported body still fails, even for optional source files.
        snapshot.documents.get_mut("status").unwrap().body.push('x');
        assert!(validate(&snapshot, s).is_err());
    }
}
