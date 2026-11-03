pub mod civic;
pub mod morgan;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub const ELECTION: &str = "2026-11-03";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub county: String,
    pub county_id: String,
    pub url: String,
    pub endpoint: Option<String>,
    pub election_id: Option<String>,
    pub enabled: bool,
    pub status: String,
    pub note: String,
    #[serde(default = "cadence")]
    pub interval_seconds: u64,
    #[serde(default)]
    pub contest_map: HashMap<String, String>,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub expected: Value,
    #[serde(default)]
    pub priority: u32,
}
fn cadence() -> u64 {
    120
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub party: String,
    pub votes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: String,
    pub name: String,
    pub county: String,
    pub county_id: String,
    pub administrative: bool,
    #[serde(default)]
    pub reporting_complete: Option<bool>,
    #[serde(default)]
    pub reporting_basis: Option<String>,
    pub candidates: Vec<Candidate>,
    pub total: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contest {
    pub id: String,
    pub source_id: String,
    pub provider_id: String,
    pub name: String,
    pub group: String,
    pub scope: String,
    pub county: String,
    pub candidates: Vec<Candidate>,
    pub units: Vec<Unit>,
    pub precincts_reporting: Option<u64>,
    pub precincts_total: Option<u64>,
    pub precinct_detail: bool,
    pub detail_note: Option<String>,
    pub total: u64,
    #[serde(default)]
    pub estimated_reporting: Option<f64>,
    #[serde(default)]
    pub reporting_complete: Option<bool>,
    #[serde(default)]
    pub reporting_basis: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    #[serde(default)]
    pub map_id: Option<String>,
    #[serde(default)]
    pub map_sha256: Option<String>,
    pub source_id: String,
    pub version: String,
    pub published_at: Option<String>,
    pub contests: Vec<Contest>,
}

fn string(v: &Value) -> String {
    v.as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| v.to_string())
}
fn number(v: &Value) -> Result<u64> {
    if let Some(n) = v.as_u64() {
        return Ok(n);
    }
    if let Some(s) = v.as_str() {
        return s.parse().context("Invalid vote count");
    }
    bail!("Missing or invalid vote count")
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).context("Vote total overflow")
}
pub fn scope(name: &str) -> String {
    let up = name.to_uppercase();
    // Require an exact office phrase and trailing district number; no substring match of 2 to 20.
    let words: Vec<_> = up
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let district = words.iter().position(|w| *w == "DISTRICT").and_then(|i| {
        words
            .iter()
            .skip(i + 1)
            .find(|w| **w != "NO")
            .and_then(|w| w.parse::<u32>().ok())
            .or_else(|| {
                words[..i].iter().rev().find_map(|w| {
                    w.trim_end_matches(|c: char| c.is_ascii_alphabetic())
                        .parse::<u32>()
                        .ok()
                })
            })
    });
    match district {
        Some(n)
            if [2, 5].contains(&n)
                && (up.contains("UNITED STATES REPRESENTATIVE")
                    || up.contains("U.S. REPRESENTATIVE")
                    || up.contains("U.S. HOUSE")) =>
        {
            format!("cd{n}")
        }
        Some(n)
            if [10, 20, 21, 25].contains(&n)
                && (up.contains("STATE REPRESENTATIVE")
                    || up.contains("HOUSE OF REPRESENTATIVES")) =>
        {
            format!("hd{n}")
        }
        Some(n) if [2, 7].contains(&n) && up.contains("STATE SENAT") => format!("sd{n}"),
        _ => "madison".into(),
    }
}
fn group(name: &str, scope: &str) -> String {
    let n = name.to_uppercase();
    if scope.starts_with("hd") || scope.starts_with("sd") {
        return "Legislature".into();
    }
    if scope.starts_with("cd") || n.contains("UNITED STATES SENAT") {
        return "Federal".into();
    }
    if n.contains("AMENDMENT") || n.contains("REFERENDUM") {
        return "Ballot measures".into();
    }
    if [
        "SHERIFF",
        "COMMISSION",
        "CORONER",
        "LICENSE",
        "PROBATE",
        "MADISON",
        "COUNTY",
        "CIRCUIT",
        "DISTRICT COURT",
    ]
    .iter()
    .any(|s| n.contains(s))
    {
        return "Local".into();
    }
    "Statewide & other".into()
}
fn truth(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
        || matches!(v.as_str(), Some("1" | "true" | "True"))
        || v.as_u64() == Some(1)
}
pub fn validate_settings(source: &Source, settings: &Value) -> Result<()> {
    let e = &settings["settings"]["electiondetails"];
    let id = source
        .election_id
        .as_ref()
        .context("Missing verified election ID")?;
    ensure!(string(&e["electionid"]) == *id, "Election ID changed");
    let date = e["electiondate"]
        .as_str()
        .context("Missing election date")?;
    let parsed = chrono::NaiveDate::parse_from_str(date, "%m/%d/%Y")
        .or_else(|_| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d"))?;
    ensure!(
        parsed.to_string() == ELECTION,
        "Wrong election date: {date}"
    );
    ensure!(
        !truth(&e["istestmode"]) && !truth(&e["showtestdatawatermark"]),
        "Test feed rejected"
    );
    let name = e["internalname"]
        .as_str()
        .context("Missing election name")?
        .to_lowercase();
    ensure!(
        name.contains("general")
            && !["primary", "runoff", "municipal", "test", "mock", "practice"]
                .iter()
                .any(|s| name.contains(s)),
        "Not a general election: {name}"
    );
    ensure!(
        settings["entityname"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case(&source.county)),
        "Wrong county"
    );
    ensure!(
        settings["parententityname"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case("al")),
        "Wrong state"
    );
    Ok(())
}
fn details_for(d: &Value, source: &Source, candidates: &[Candidate]) -> Result<Vec<Unit>> {
    let names = d["P"].as_array().context("Missing precinct names")?;
    let rows = d["V"].as_array().context("Missing precinct vote matrix")?;
    ensure!(names.len() == rows.len(), "Precinct dimensions disagree");
    let totals = d["T"].as_array().context("Missing precinct totals")?;
    ensure!(
        totals.len() == rows.len(),
        "Precinct total dimensions disagree"
    );
    let mut units = Vec::new();
    let mut seen = HashSet::new();
    let mut sums = vec![0u64; candidates.len()];
    for (i, (name, values)) in names.iter().zip(rows).enumerate() {
        let name = name.as_str().context("Invalid precinct name")?;
        ensure!(
            !name.trim().is_empty() && seen.insert(name.to_owned()),
            "Empty or duplicate precinct"
        );
        let vals = values.as_array().context("Invalid precinct row")?;
        ensure!(
            vals.len() == candidates.len(),
            "Candidate dimensions disagree"
        );
        let mut cs = Vec::new();
        let mut total = 0;
        for (j, base) in candidates.iter().enumerate() {
            let votes = number(&vals[j])?;
            total = add(total, votes)?;
            sums[j] = add(sums[j], votes)?;
            let mut candidate = base.clone();
            candidate.votes = votes;
            cs.push(candidate);
        }
        ensure!(
            total == number(&totals[i])?,
            "Precinct candidate counts do not reconcile"
        );
        let upper = name.to_uppercase();
        units.push(Unit {
            reporting_complete: None,
            reporting_basis: None,
            id: format!("{}:unit:{}", source.id, name),
            name: name.into(),
            county: source.county.clone(),
            county_id: source.county_id.clone(),
            administrative: ["ABSENTEE", "PROVISIONAL", "CURBSIDE", "UOCAVA", "CENTRAL"]
                .iter()
                .any(|s| upper.contains(s)),
            candidates: cs,
            total,
        });
    }
    ensure!(
        candidates.iter().zip(sums).all(|(c, n)| c.votes == n),
        "Precinct candidate sums do not reconcile to summary; summary retained alone"
    );
    Ok(units)
}
pub fn parse(
    source: &Source,
    version: &str,
    settings: &Value,
    summary: &Value,
    details: &Value,
) -> Result<Batch> {
    validate_settings(source, settings)?;
    ensure!(
        !version.is_empty() && version.chars().all(|c| c.is_ascii_digit()),
        "Invalid provider version"
    );
    let rows = summary.as_array().context("Missing summary contests")?;
    let detail_rows = details["Contests"]
        .as_array()
        .context("Missing detail contests")?;
    let mut detail_index = HashMap::new();
    for row in detail_rows {
        let key = row["K"].as_str().context("Missing detail contest ID")?;
        ensure!(
            detail_index.insert(key, row).is_none(),
            "Duplicate detail contest"
        );
    }
    let mut seen = HashSet::new();
    let mut contests = Vec::new();
    for c in rows {
        let key = c["K"].as_str().context("Missing contest ID")?;
        ensure!(seen.insert(key), "Duplicate contest");
        let name = c["C"].as_str().context("Missing contest name")?;
        if name.eq_ignore_ascii_case("STRAIGHT PARTY VOTING") {
            continue;
        }
        let scope = source
            .contest_map
            .get(key)
            .filter(|s| {
                ["cd2", "cd5", "hd10", "hd20", "hd21", "hd25", "sd2", "sd7"].contains(&s.as_str())
            })
            .cloned()
            .unwrap_or_else(|| scope(name));
        if source.county != "Madison" && scope == "madison" {
            continue;
        }
        ensure!(!truth(&c["IsRCV"]), "Unsupported ranked-choice contest");
        let names = c["CH"].as_array().context("Missing candidate names")?;
        let votes = c["V"].as_array().context("Missing candidate totals")?;
        let parties = c["P"].as_array().context("Missing candidate parties")?;
        ensure!(
            names.len() == votes.len() && names.len() == parties.len(),
            "Candidate dimensions disagree"
        );
        let mut candidates = Vec::new();
        let mut total = 0;
        for (i, n) in names.iter().enumerate() {
            let votes = number(&votes[i])?;
            total = add(total, votes)?;
            candidates.push(Candidate {
                source_name: None,
                id: format!("{key}:{i}"),
                name: n.as_str().context("Invalid candidate name")?.into(),
                party: parties[i].as_str().context("Invalid party")?.into(),
                votes,
            });
        }
        let (units, detail_note) = match detail_index.get(key) {
            Some(d) => match details_for(d, source, &candidates) {
                Ok(u) => (u, None),
                Err(e) => (Vec::new(), Some(e.to_string())),
            },
            None => (
                Vec::new(),
                Some("Precinct detail has not been published for this contest".into()),
            ),
        };
        let precinct_detail = !units.is_empty();
        contests.push(Contest {
            reporting_complete: None,
            reporting_basis: None,
            estimated_reporting: None,
            id: format!("{}:{key}", source.id),
            source_id: source.id.clone(),
            provider_id: key.into(),
            name: name.into(),
            group: group(name, &scope),
            scope,
            county: source.county.clone(),
            candidates,
            units,
            precincts_reporting: c["PR"].as_u64(),
            precincts_total: c["TP"].as_u64(),
            precinct_detail,
            detail_note,
            total,
        });
    }
    ensure!(!contests.is_empty(), "No in-scope contests in source");
    Ok(Batch {
        map_sha256: None,
        map_id: settings["settings"]["kmlmap"].as_str().map(str::to_owned),
        source_id: source.id.clone(),
        version: version.into(),
        published_at: settings["websiteupdatedat"].as_str().map(str::to_owned),
        contests,
    })
}

/// Apply the provider's exact-name precinct status from the same verified revision.
/// Clarity's client treats 4 and 64 as completely reported; certification is separate.
pub fn apply_precinct_status(batch: &mut Batch, status: &Value) -> Result<()> {
    let names = status["P"]
        .as_array()
        .context("Missing precinct status names")?;
    let codes = status["S"]
        .as_array()
        .context("Missing precinct status codes")?;
    ensure!(
        names.len() == codes.len(),
        "Precinct status dimensions disagree"
    );
    let mut index = HashMap::new();
    for (name, code) in names.iter().zip(codes) {
        let name = name.as_str().context("Invalid precinct status name")?;
        ensure!(!name.trim().is_empty(), "Empty precinct status name");
        let code = code.as_u64().context("Invalid precinct status code")?;
        ensure!(
            index.insert(name, code).is_none(),
            "Duplicate precinct status name"
        );
    }
    // Validate the complete join before mutating the batch. A malformed status
    // response must never leave only some rows labeled complete.
    ensure!(
        batch
            .contests
            .iter()
            .flat_map(|c| &c.units)
            .all(|u| index.contains_key(u.name.as_str())),
        "Precinct status identity mismatch"
    );
    for unit in batch.contests.iter_mut().flat_map(|c| &mut c.units) {
        let code = index[unit.name.as_str()];
        unit.reporting_complete = match code {
            4 | 64 => Some(true),
            1 | 2 | 3 => Some(false),
            _ => None,
        };
        unit.reporting_basis = Some(format!("Official Clarity precinct status {code}"));
    }
    for contest in &mut batch.contests {
        let participating = contest
            .units
            .iter()
            .filter(|u| !u.administrative && !matches!(index[u.name.as_str()], 8 | 16))
            .collect::<Vec<_>>();
        contest.reporting_complete = if participating.is_empty() {
            None
        } else if participating
            .iter()
            .any(|u| u.reporting_complete == Some(false))
        {
            Some(false)
        } else if participating.iter().any(|u| u.reporting_complete.is_none()) {
            None
        } else {
            Some(true)
        };
        contest.reporting_basis = (!participating.is_empty()).then(|| {
            "Official geographic precinct statuses; administrative totals may change".into()
        });
    }
    Ok(())
}

pub fn requested() -> Value {
    json!([
        {"id":"cd5","name":"U.S. House, District 5","group":"Federal","scope":"cd5"},
        {"id":"hd10","name":"State House, District 10","group":"Legislature","scope":"hd10"},
        {"id":"hd20","name":"State House, District 20","group":"Legislature","scope":"hd20"},
        {"id":"hd21","name":"State House, District 21","group":"Legislature","scope":"hd21"},
        {"id":"hd25","name":"State House, District 25","group":"Legislature","scope":"hd25"},
        {"id":"sd2","name":"State Senate, District 2","group":"Legislature","scope":"sd2"},
        {"id":"sd7","name":"State Senate, District 7","group":"Legislature","scope":"sd7"}
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Source, Value, Value, Value) {
        let s = Source {
            format: String::new(),
            expected: Value::Null,
            priority: 0,
            id: "madison".into(),
            county: "Madison".into(),
            county_id: "01089".into(),
            url: "https://example.invalid".into(),
            endpoint: None,
            election_id: Some("fixture".into()),
            enabled: false,
            status: "fixture".into(),
            note: "Synthetic test only".into(),
            interval_seconds: 120,
            contest_map: HashMap::new(),
        };
        let e = json!({"entityname":"Madison","parententityname":"AL","settings":{"electiondetails":{"electionid":"fixture","electiondate":"11/3/2026","internalname":"2026 General Election","istestmode":"false"}}});
        let c = json!([{"K":"01","C":"STATE REPRESENTATIVE, DISTRICT 20","CH":["Example Democrat","Example Republican"],"P":["DEM","REP"],"V":[6,4],"PR":1,"TP":2}]);
        let d = json!({"Contests":[{"K":"01","P":["Example Precinct","Absentee"],"V":[[5,3],[1,1]],"T":[8,2]}]});
        (s, e, c, d)
    }
    #[test]
    fn validates_and_separates_administrative() {
        let (s, e, c, d) = fixture();
        let b = parse(&s, "1", &e, &c, &d).unwrap();
        assert_eq!(b.contests[0].scope, "hd20");
        assert_eq!(b.contests[0].total, 10);
        assert!(b.contests[0].units[1].administrative);
    }
    #[test]
    fn rejects_wrong_date_test_primary_and_county() {
        let (s, e, c, d) = fixture();
        for (p, v) in [
            ("electiondate", json!("11/5/2024")),
            ("istestmode", json!(true)),
            ("internalname", json!("2026 General Primary")),
            ("showtestdatawatermark", json!("1")),
            ("electionid", json!("different")),
        ] {
            let mut x = e.clone();
            x["settings"]["electiondetails"][p] = v;
            assert!(parse(&s, "1", &x, &c, &d).is_err(), "{p}");
        }
        let mut x = e.clone();
        x["entityname"] = json!("Morgan");
        assert!(parse(&s, "1", &x, &c, &d).is_err());
    }
    #[test]
    fn nonreconciling_detail_keeps_summary_without_double_count() {
        let (s, e, c, mut d) = fixture();
        d["Contests"][0]["V"][0][0] = json!(100);
        let b = parse(&s, "1", &e, &c, &d).unwrap();
        assert_eq!(b.contests[0].total, 10);
        assert!(!b.contests[0].precinct_detail);
        assert!(b.contests[0].units.is_empty());
    }
    #[test]
    fn malformed_summary_rejected() {
        let (s, e, mut c, d) = fixture();
        c[0]["V"][0] = json!(-1);
        assert!(parse(&s, "1", &e, &c, &d).is_err());
    }
    #[test]
    fn scopes_are_exact() {
        assert_eq!(scope("STATE SENATOR, DISTRICT 20"), "madison");
        assert_eq!(scope("STATE SENATOR, DISTRICT 2"), "sd2");
        assert_eq!(scope("UNITED STATES REPRESENTATIVE, DISTRICT 5"), "cd5");
        assert_eq!(scope("STATE REPRESENTATIVE DISTRICT 25"), "hd25");
        assert_eq!(
            scope("UNITED STATES REPRESENTATIVE, 2ND CONGRESSIONAL DISTRICT"),
            "cd2"
        );
        assert_eq!(
            scope("UNITED STATES REPRESENTATIVE, DISTRICT 22"),
            "madison"
        );
    }
}
