use alabama_election_night::{Source, civic};
use serde_json::{Value, json};
fn fixture() -> (Source, Value) {
    let sources: Vec<Source> = serde_json::from_str(include_str!("../data/sources.json")).unwrap();
    (
        sources.into_iter().find(|s| s.id == "civic-cd2").unwrap(),
        serde_json::from_str(include_str!("fixtures/al02-civic-cd2-2026-setup.json")).unwrap(),
    )
}
#[test]
fn current_al02_covers_the_sixteen_counties_and_preserves_source_candidate_alias() {
    let (s, payload) = fixture();
    let batch = civic::parse(&s, &payload).unwrap();
    let expected: std::collections::BTreeSet<_> = [
        "Barbour",
        "Bullock",
        "Butler",
        "Coffee",
        "Covington",
        "Crenshaw",
        "Dale",
        "Elmore",
        "Geneva",
        "Henry",
        "Houston",
        "Lowndes",
        "Macon",
        "Montgomery",
        "Pike",
        "Russell",
    ]
    .into_iter()
    .collect();
    assert_eq!(
        batch
            .contests
            .iter()
            .map(|c| c.county.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    assert_eq!(batch.contests.len(), 16);
    assert!(batch.contests.iter().all(|c| c.scope == "cd2"
        && c.group == "Federal"
        && c.units.is_empty()
        && !c.precinct_detail
        && c.total == 0));
    let figures = batch.contests[0]
        .candidates
        .iter()
        .find(|c| c.party == "DEM")
        .unwrap();
    assert_eq!(figures.name, "Shomari C. Figures");
    assert_eq!(figures.source_name.as_deref(), Some("Shomari Figures"));
}
#[test]
fn al02_rejects_ineligible_elections_or_bad_county_coverage_and_keeps_positive_county_totals() {
    let (s, original) = fixture();
    for (field, value) in [
        ("test_data", json!(true)),
        ("test_data", Value::Null),
        ("election_date", json!("2024-11-05T05:00:00Z")),
        ("district", json!("AL-05")),
        ("election_scope", json!("Primary")),
    ] {
        let mut x = original.clone();
        x[field] = value;
        assert!(civic::parse(&s, &x).is_err(), "{field}");
    }
    let mut x = original.clone();
    x["region_results"]
        .as_object_mut()
        .unwrap()
        .remove("elmore");
    assert!(civic::parse(&s, &x).is_err());
    let mut x = original.clone();
    x["region_results"]["elmore"]["fips"] = json!("01097");
    assert!(civic::parse(&s, &x).is_err());
    let mut x = original.clone();
    x["region_results"]["elmore"]["candidates"][0]["votes"] = json!(9);
    assert!(civic::parse(&s, &x).is_err());
    let name = x["region_results"]["elmore"]["candidates"][0]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    x["candidates"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"].as_str() == Some(&name))
        .unwrap()["votes"] = json!(9);
    let batch = civic::parse(&s, &x).unwrap();
    assert_eq!(batch.contests.iter().map(|c| c.total).sum::<u64>(), 9);
    assert!(batch.contests.iter().all(|c| c.units.is_empty()));
}
