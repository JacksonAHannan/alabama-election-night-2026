//! Public civicAPI county totals. County rows never become precinct observations.
use crate::{Batch, Candidate, Contest, ELECTION, Source, add, number};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .with_context(|| format!("Missing civicAPI {key}"))
}
fn party(value: &str, name: &str) -> Result<&'static str> {
    match value {
        "Republican" => Ok("REP"),
        "Democratic" => Ok("DEM"),
        "Nonpartisan" if name.eq_ignore_ascii_case("Yes") => Ok("YES"),
        "Nonpartisan" if name.eq_ignore_ascii_case("No") => Ok("NO"),
        _ => anyhow::bail!("Unconfigured civicAPI party {value}"),
    }
}
pub fn parse(source: &Source, data: &Value) -> Result<Batch> {
    let e = &source.expected;
    let race_id = source
        .election_id
        .as_ref()
        .context("Missing civicAPI race ID")?;
    let scope = source
        .contest_map
        .get(race_id)
        .context("Unmapped civicAPI race")?;
    ensure!(
        data["test_data"] == false,
        "civicAPI test flag must explicitly be false"
    );
    ensure!(
        text(data, "country")? == "US" && text(data, "province")? == "AL",
        "Wrong civicAPI country/state"
    );
    let date = chrono::DateTime::parse_from_rfc3339(text(data, "election_date")?)?;
    ensure!(
        date.date_naive().to_string() == ELECTION,
        "Wrong civicAPI election date"
    );
    for (field, expected) in [
        ("election_name", "name"),
        ("election_type", "office"),
        ("election_scope", "election_scope"),
    ] {
        ensure!(
            text(data, field)? == text(e, expected)?,
            "Wrong civicAPI {field}"
        );
    }
    ensure!(
        data.get("district").is_some()
            && e.get("district").is_some()
            && data["district"] == e["district"],
        "Wrong civicAPI district"
    );
    ensure!(
        data["is_disputed"] == false,
        "Disputed civicAPI race withheld"
    );
    let roster = data["candidates"]
        .as_array()
        .context("Missing civicAPI candidate roster")?;
    let configured = e["candidates"]
        .as_array()
        .context("Missing verified civicAPI candidate map")?;
    ensure!(
        !configured.is_empty() && roster.len() == configured.len(),
        "civicAPI candidate roster changed"
    );
    let mut candidate_ids = HashSet::new();
    for c in roster {
        let id = c["candidate_id"]
            .as_u64()
            .context("Missing civicAPI candidate ID")?
            .to_string();
        ensure!(
            candidate_ids.insert(id.clone()),
            "Duplicate civicAPI candidate ID"
        );
        let config = configured
            .iter()
            .find(|k| k["id"].as_str() == Some(&id))
            .context("Unknown civicAPI candidate ID")?;
        ensure!(
            text(c, "name")? == text(config, "source_name")?
                && party(text(c, "party")?, text(c, "name")?)? == text(config, "party")?,
            "civicAPI candidate identity changed"
        );
        number(&c["votes"])?;
    }
    let regions = data["region_results"]
        .as_object()
        .context("Missing civicAPI county results")?;
    let counties = e["counties"]
        .as_array()
        .context("Missing configured county coverage")?;
    ensure!(
        regions.len() == counties.len(),
        "civicAPI county coverage changed"
    );
    let mut seen = HashSet::new();
    let mut contests = Vec::new();
    for (region_id, region) in regions {
        let fips = text(region, "fips")?;
        let county = counties
            .iter()
            .find(|c| c["fips"].as_str() == Some(fips))
            .context("Unconfigured civicAPI county FIPS")?;
        ensure!(
            text(region, "type")? == "County"
                && text(region, "name")? == text(county, "name")?
                && region_id == text(county, "id")?,
            "Wrong civicAPI county identity"
        );
        ensure!(seen.insert(fips), "Duplicate civicAPI county");
        let rows = region["candidates"]
            .as_array()
            .context("Missing civicAPI county candidates")?;
        ensure!(
            rows.len() == configured.len(),
            "Incomplete civicAPI county candidate roster"
        );
        let mut candidates = Vec::new();
        let mut total = 0;
        let mut names = HashSet::new();
        for row in rows {
            let name = text(row, "name")?;
            let p = party(text(row, "party")?, name)?;
            let config = configured
                .iter()
                .find(|c| c["source_name"].as_str() == Some(name) && c["party"].as_str() == Some(p))
                .context("Unknown civicAPI county candidate")?;
            ensure!(
                names.insert((name, p)),
                "Duplicate civicAPI county candidate"
            );
            let votes = number(&row["votes"])?;
            total = add(total, votes)?;
            candidates.push(Candidate {
                id: text(config, "id")?.into(),
                name: text(config, "name")?.into(),
                party: p.into(),
                votes,
                source_name: Some(name.into()),
            });
        }
        let reporting = region["percent_reporting"].as_f64();
        ensure!(
            reporting.is_none_or(|p| p.is_finite() && (0.0..=100.0).contains(&p)),
            "Invalid civicAPI reporting estimate"
        );
        contests.push(Contest{id:format!("{}:{fips}:{race_id}",source.id),source_id:source.id.clone(),provider_id:race_id.clone(),name:text(data,"election_name")?.into(),group:e["group"].as_str().unwrap_or(if scope=="cd5"{"Federal"}else{"Legislature"}).into(),scope:scope.clone(),county:text(region,"name")?.into(),candidates,units:vec![],precincts_reporting:None,precincts_total:None,precinct_detail:false,detail_note:Some("civicAPI county candidate totals only; no precinct breakdown or write-in counts supplied. Reporting percentage is a publisher estimate.".into()),total,estimated_reporting:reporting,reporting_complete:reporting.map(|p|p==100.0),reporting_basis:reporting.map(|_|"civicAPI estimated reporting".into())});
    }
    // Each snapshot is a single race: its county candidate sums must reconcile
    // with the publisher's top-level candidate totals before it can be accepted.
    for row in roster {
        let id = row["candidate_id"].as_u64().unwrap().to_string();
        let mut sum = 0;
        for c in &contests {
            sum = add(
                sum,
                c.candidates
                    .iter()
                    .find(|x| x.id == id)
                    .context("Missing county candidate")?
                    .votes,
            )?;
        }
        ensure!(
            sum == number(&row["votes"])?,
            "civicAPI county candidate totals do not reconcile"
        );
    }
    let published = text(data, "last_updated")?;
    chrono::DateTime::parse_from_rfc3339(published)?;
    Ok(Batch {
        map_id: None,
        map_sha256: None,
        source_id: source.id.clone(),
        version: format!("{:x}", Sha256::digest(serde_json::to_vec(data)?)),
        published_at: Some(published.into()),
        contests,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Source, Value) {
        let sources: Vec<Source> =
            serde_json::from_str(include_str!("../data/sources.json")).unwrap();
        (
            sources.into_iter().find(|s| s.id == "civic-cd5").unwrap(),
            serde_json::from_str(include_str!("../tests/fixtures/civic-cd5-2026-setup.json"))
                .unwrap(),
        )
    }
    #[test]
    fn county_totals_are_not_precincts() {
        let (s, d) = fixture();
        let b = parse(&s, &d).unwrap();
        assert_eq!(b.contests.len(), 6);
        assert!(
            b.contests.iter().all(|c| c.units.is_empty()
                && !c.precinct_detail
                && c.precincts_reporting.is_none())
        );
        let l = b.contests.iter().find(|c| c.county == "Limestone").unwrap();
        assert_eq!(l.candidates[0].name, "Dale W. Strong");
        assert_eq!(l.candidates[0].source_name.as_deref(), Some("Dale Strong"));
    }
    #[test]
    fn rejects_test_wrong_date_district_and_missing_county() {
        let (s, d) = fixture();
        for (field, value) in [
            ("test_data", Value::Bool(true)),
            ("test_data", Value::Null),
            (
                "election_date",
                Value::String("2024-11-05T05:00:00Z".into()),
            ),
            ("district", Value::String("AL-04".into())),
            ("election_scope", Value::String("Primary".into())),
        ] {
            let mut x = d.clone();
            x[field] = value;
            assert!(parse(&s, &x).is_err(), "{field}");
        }
        let mut x = d.clone();
        x["region_results"]
            .as_object_mut()
            .unwrap()
            .remove("limestone");
        assert!(parse(&s, &x).is_err());
    }
    #[test]
    fn rejects_unknown_or_duplicate_county_candidate() {
        let (s, mut d) = fixture();
        d["region_results"]["limestone"]["candidates"][0]["name"] =
            Value::String("Other Person".into());
        assert!(parse(&s, &d).is_err());
    }
}
