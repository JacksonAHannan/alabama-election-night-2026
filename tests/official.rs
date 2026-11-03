use alabama_election_night::{Source, parse};
use serde_json::Value;
#[test]
fn actual_madison_setup_reconciles_all_contests_and_requested_districts() {
    let sources: Vec<Source> = serde_json::from_str(include_str!("../data/sources.json")).unwrap();
    let s = sources.iter().find(|s| s.id == "madison").unwrap();
    let settings: Value =
        serde_json::from_str(include_str!("../research/madison-settings.json")).unwrap();
    let summary: Value =
        serde_json::from_str(include_str!("../research/madison-summary.json")).unwrap();
    let details: Value =
        serde_json::from_str(include_str!("../research/madison-details.json")).unwrap();
    let b = parse(s, "382270", &settings, &summary, &details).unwrap();
    assert_eq!(b.contests.len(), 49);
    assert_eq!(
        b.contests.iter().map(|c| c.units.len()).sum::<usize>(),
        3012
    );
    assert!(
        b.contests
            .iter()
            .all(|c| c.precinct_detail && c.detail_note.is_none() && c.total == 0)
    );
    for key in ["cd5", "hd10", "hd20", "hd21", "hd25", "sd2", "sd7"] {
        assert_eq!(
            b.contests.iter().filter(|c| c.scope == key).count(),
            1,
            "{key}"
        );
    }
    assert!(
        b.contests
            .iter()
            .all(|c| s.contest_map.contains_key(&c.provider_id))
    );
    assert!(
        b.contests
            .iter()
            .flat_map(|c| &c.units)
            .filter(|u| u.administrative)
            .all(|u| ["ABSENTEE", "PROVISIONAL ABSENTEE", "PROVISIONAL"]
                .contains(&u.name.to_uppercase().as_str()))
    );
}

#[test]
fn all_current_civic_county_snapshots_validate_and_limestone_has_three_races() {
    let sources: Vec<Source> = serde_json::from_str(include_str!("../data/sources.json")).unwrap();
    let mut limestone = Vec::new();
    for source in sources.iter().filter(|s| {
        s.format == "civic_county" && s.expected["statewide"] != true && s.id != "civic-cd2"
    }) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/{}-2026-setup.json", source.id));
        let payload: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let batch = alabama_election_night::civic::parse(source, &payload).unwrap();
        assert!(
            batch
                .contests
                .iter()
                .all(|c| c.units.is_empty() && !c.precinct_detail && c.total == 0)
        );
        for contest in &batch.contests {
            if contest.county == "Limestone" {
                limestone.push(contest.scope.clone());
            }
        }
    }
    limestone.sort();
    assert_eq!(limestone, vec!["cd5", "hd25", "sd2"]);
}

#[test]
fn statewide_county_feeds_match_all_counties_and_verified_ballot_choices() {
    let sources: Vec<Source> = serde_json::from_str(include_str!("../data/sources.json")).unwrap();
    let ballot: Value = serde_json::from_str(include_str!("../data/ballot.json")).unwrap();
    let counties: Value =
        serde_json::from_str(include_str!("../data/statewide-geography.json")).unwrap();
    let expected: std::collections::BTreeSet<_> = counties["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["properties"]["county_id"].as_str().unwrap())
        .collect();
    assert_eq!(expected.len(), 67);
    let mut count = 0;
    for source in sources.iter().filter(|s| s.expected["statewide"] == true) {
        count += 1;
        let payload: Value = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("tests/fixtures/{}-2026-setup.json", source.id)),
            )
            .unwrap(),
        )
        .unwrap();
        let batch = alabama_election_night::civic::parse(source, &payload).unwrap();
        assert_eq!(batch.contests.len(), 67);
        let actual: std::collections::BTreeSet<_> = source.expected["counties"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["fips"].as_str().unwrap())
            .collect();
        assert_eq!(actual, expected);
        let rid = source.contest_map.values().next().unwrap();
        let race = ballot["contests"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"].as_str() == Some(rid))
            .unwrap();
        assert_eq!(race["statewide"], true);
        for c in &batch.contests {
            assert!(c.units.is_empty() && !c.precinct_detail);
            for candidate in &c.candidates {
                assert!(
                    race["candidates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|b| b["name"] == candidate.name && b["party"] == candidate.party)
                );
            }
        }
        let mut positive = payload.clone();
        let candidate_name = positive["candidates"][0]["name"]
            .as_str()
            .unwrap()
            .to_owned();
        positive["candidates"][0]["votes"] = serde_json::json!(17);
        let county_rows = positive["region_results"]["autauga"]["candidates"]
            .as_array_mut()
            .unwrap();
        county_rows
            .iter_mut()
            .find(|c| c["name"].as_str() == Some(&candidate_name))
            .unwrap()["votes"] = serde_json::json!(17);
        let observed = alabama_election_night::civic::parse(source, &positive).unwrap();
        assert_eq!(
            observed
                .contests
                .iter()
                .find(|c| c.county == "Autauga")
                .unwrap()
                .total,
            17
        );
        assert_eq!(observed.contests.iter().map(|c| c.total).sum::<u64>(), 17);
        let mut mismatch = payload.clone();
        mismatch["region_results"]["autauga"]["candidates"][0]["votes"] = serde_json::json!(1);
        assert!(alabama_election_night::civic::parse(source, &mismatch).is_err());
        let mut wrong_choice = payload.clone();
        wrong_choice["region_results"]["autauga"]["candidates"][0]["name"] =
            serde_json::json!("Unknown choice");
        assert!(alabama_election_night::civic::parse(source, &wrong_choice).is_err());
        let mut invalid = payload.clone();
        invalid["district"] = serde_json::json!("AL-05");
        assert!(alabama_election_night::civic::parse(source, &invalid).is_err());
        let mut missing = payload.clone();
        missing.as_object_mut().unwrap().remove("district");
        assert!(alabama_election_night::civic::parse(source, &missing).is_err());
        let mut invalid = payload.clone();
        invalid["test_data"] = serde_json::json!(true);
        assert!(alabama_election_night::civic::parse(source, &invalid).is_err());
        let mut invalid = payload.clone();
        invalid["region_results"]
            .as_object_mut()
            .unwrap()
            .remove("madison");
        assert!(alabama_election_night::civic::parse(source, &invalid).is_err());
    }
    assert_eq!(count, 12);
}
