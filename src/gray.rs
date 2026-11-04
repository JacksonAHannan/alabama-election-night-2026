//! WAFF's published statewide Senate report. Never a county observation.
use crate::{Batch, Candidate, Contest, ELECTION, Source};
use anyhow::{Context, Result, ensure};
use chrono::Datelike;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn count(v: &Value) -> Result<u64> {
    if let Some(n) = v.as_u64() {
        return Ok(n);
    }
    let s = v.as_str().context("Missing publisher vote count")?;
    ensure!(
        !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit() || c == b','),
        "Invalid publisher vote count"
    );
    if s.contains(',') {
        let chunks: Vec<_> = s.split(',').collect();
        ensure!(
            (1..=3).contains(&chunks[0].len()) && chunks[1..].iter().all(|c| c.len() == 3),
            "Invalid vote grouping"
        );
    }
    s.replace(',', "")
        .parse()
        .context("Publisher count overflow")
}

pub fn parse(source: &Source, menu: &Value, data: &Value) -> Result<Batch> {
    let e = &source.expected;
    ensure!(
        source.format == "gray_statewide" && e["election"] == ELECTION,
        "Wrong publisher source/election"
    );
    let config = e["config_id"]
        .as_str()
        .context("Missing publisher configuration")?;
    ensure!(
        source.election_id.as_deref() == Some(config)
            && data["id"].as_u64().map(|n| n.to_string()).as_deref() == Some(config)
            && data["Title"] == e["title"],
        "Wrong publisher election configuration"
    );
    let entries = menu["data"]
        .as_array()
        .context("Missing published election menu")?;
    ensure!(
        entries
            .iter()
            .filter(|m| m["configID"] == config
                && m["title"] == e["title"]
                && m["fileID"] == e["file_id"])
            .count()
            == 1,
        "Published menu no longer identifies configured general election"
    );
    let published = data["updatedDate"]
        .as_str()
        .context("Publisher has no publication time")?;
    let date = chrono::DateTime::parse_from_rfc3339(published)?;
    ensure!(
        date.year() == 2026
            && date.date_naive().to_string().as_str() >= ELECTION
            && date.signed_duration_since(chrono::Utc::now()).num_seconds() <= 120,
        "Wrong publisher election year or future publication"
    );
    let race_id = e["race_id"].as_str().context("Missing configured race")?;
    let matches: Vec<_> = data["Race"]
        .as_array()
        .context("Missing publisher races")?
        .iter()
        .filter(|r| r["raceNumber"] == race_id)
        .collect();
    ensure!(
        matches.len() == 1,
        "Missing or duplicate publisher Senate race"
    );
    let race = matches[0];
    ensure!(
        race["title"] == e["race_title"]
            && race["countyName"] == ""
            && race["raceFormat"] == "popular",
        "Wrong office or geographic scope"
    );
    let rows = race["candidates"]
        .as_array()
        .context("Missing publisher candidates")?;
    let roster = e["candidates"]
        .as_array()
        .context("Missing configured roster")?;
    ensure!(
        rows.len() == roster.len() && roster.len() == 2,
        "Publisher candidate roster changed"
    );
    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();
    let mut total = 0u64;
    for row in rows {
        let id = row["candidateNumber"]
            .as_str()
            .context("Missing candidate ID")?;
        let c = roster
            .iter()
            .find(|c| c["id"] == id)
            .context("Unknown publisher candidate")?;
        ensure!(
            seen.insert(id)
                && row["raceNumber"] == race_id
                && row["firstName"] == c["first"]
                && row["lastName"] == c["last"]
                && row["party"] == c["source_party"],
            "Publisher candidate identity changed"
        );
        let votes = count(&row["voteCount"])?;
        total = total
            .checked_add(votes)
            .context("Publisher total overflow")?;
        candidates.push(Candidate {
            id: id.into(),
            name: c["name"].as_str().context("Missing canonical name")?.into(),
            party: c["party"].as_str().context("Missing party")?.into(),
            votes,
            source_name: None,
        });
    }
    Ok(Batch {
        source_id: source.id.clone(),
        map_id: None,
        map_sha256: None,
        version: format!("{:x}", Sha256::digest(serde_json::to_vec(data)?)),
        published_at: Some(published.into()),
        contests: vec![Contest {
            id: format!("{}:{race_id}", source.id),
            source_id: source.id.clone(),
            provider_id: race_id.into(),
            name: e["race_title"].as_str().unwrap().into(),
            group: "Federal".into(),
            scope: "us-senate".into(),
            county: "Alabama".into(),
            candidates,
            units: vec![],
            precincts_reporting: None,
            precincts_total: None,
            precinct_detail: false,
            detail_note: Some(
                "WAFF statewide named-candidate report only; no county or precinct allocation"
                    .into(),
            ),
            total,
            estimated_reporting: None,
            reporting_complete: None,
            reporting_basis: None,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Source, Value, Value) {
        let sources: Vec<Source> =
            serde_json::from_str(include_str!("../data/sources.json")).unwrap();
        (
            sources
                .into_iter()
                .find(|s| s.format == "gray_statewide")
                .unwrap(),
            serde_json::from_str(include_str!("../tests/fixtures/waff-menu-2026.json")).unwrap(),
            serde_json::from_str(include_str!("../tests/fixtures/waff-senate-2026-live.json"))
                .unwrap(),
        )
    }
    #[test]
    fn real_report_is_statewide_only_and_keeps_no_completion_inference() {
        let (s, m, d) = fixture();
        let b = parse(&s, &m, &d).unwrap();
        assert_eq!(b.contests.len(), 1);
        let r = &b.contests[0];
        assert_eq!(r.total, 10506);
        assert_eq!(r.candidates[0].votes, 5539);
        assert_eq!(r.candidates[1].votes, 4967);
        assert!(
            r.units.is_empty() && r.reporting_complete.is_none() && r.estimated_reporting.is_none()
        );
        assert_eq!(r.county, "Alabama");
    }
    #[test]
    fn rejects_wrong_election_scope_roster_and_counts() {
        let (s, m, d) = fixture();
        let mut bad = d.clone();
        bad["id"] = 1.into();
        assert!(parse(&s, &m, &bad).is_err());
        let mut bad = d.clone();
        bad["updatedDate"] = "2024-11-05T01:00:00Z".into();
        assert!(parse(&s, &m, &bad).is_err());
        let mut bad = m.clone();
        bad["data"] = serde_json::json!([]);
        assert!(parse(&s, &bad, &d).is_err());
        let i = d["Race"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r["raceNumber"] == "59124")
            .unwrap();
        for (field, value) in [("countyName", "Mobile"), ("title", "Governor")] {
            let mut bad = d.clone();
            bad["Race"][i][field] = value.into();
            assert!(parse(&s, &m, &bad).is_err());
        }
        for (field, value) in [
            ("party", "D"),
            ("lastName", "Different"),
            ("voteCount", "-1"),
            ("voteCount", "5,,539"),
        ] {
            let mut bad = d.clone();
            bad["Race"][i]["candidates"][0][field] = value.into();
            assert!(parse(&s, &m, &bad).is_err());
        }
    }
}
