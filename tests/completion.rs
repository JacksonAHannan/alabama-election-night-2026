use alabama_election_night::{Batch, Source, apply_precinct_status, civic, parse};
use serde_json::{Value, json};
fn fixtures() -> (Batch, Value) {
    let sources: Vec<Source> = serde_json::from_str(include_str!("../data/sources.json")).unwrap();
    let s = sources.iter().find(|s| s.id == "madison").unwrap();
    let settings: Value =
        serde_json::from_str(include_str!("../research/madison-settings.json")).unwrap();
    let summary: Value =
        serde_json::from_str(include_str!("../research/madison-summary.json")).unwrap();
    let details: Value =
        serde_json::from_str(include_str!("../research/madison-details.json")).unwrap();
    (
        parse(s, "382270", &settings, &summary, &details).unwrap(),
        serde_json::from_str(include_str!("fixtures/madison-status-2026-setup.json")).unwrap(),
    )
}
#[test]
fn exact_precinct_statuses_gate_completion_and_county_requires_all_geographic_precincts() {
    let (mut b, mut status) = fixtures();
    apply_precinct_status(&mut b, &status).unwrap();
    assert!(
        b.contests
            .iter()
            .all(|c| c.reporting_complete == Some(false))
    );
    assert!(
        b.contests
            .iter()
            .flat_map(|c| &c.units)
            .all(|u| u.reporting_complete != Some(true))
    );
    let name = b.contests[0]
        .units
        .iter()
        .find(|u| !u.administrative)
        .unwrap()
        .name
        .clone();
    let i = status["P"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n == &name)
        .unwrap();
    for (code, complete) in [
        (4, Some(true)),
        (64, Some(true)),
        (1, Some(false)),
        (2, Some(false)),
        (3, Some(false)),
        (0, None),
        (8, None),
        (16, None),
        (32, None),
        (68, None),
        (99, None),
    ] {
        status["S"][i] = json!(code);
        apply_precinct_status(&mut b, &status).unwrap();
        assert!(
            b.contests
                .iter()
                .flat_map(|c| &c.units)
                .filter(|u| u.name == name)
                .all(|u| u.reporting_complete == complete),
            "{code}"
        );
    }
    for code in status["S"].as_array_mut().unwrap() {
        *code = json!(4);
    }
    apply_precinct_status(&mut b, &status).unwrap();
    assert!(
        b.contests
            .iter()
            .all(|c| c.reporting_complete == Some(true))
    );
    // Cumulative/provisional administrative units do not establish geographic progress.
    for admin in b.contests[0].units.iter().filter(|u| u.administrative) {
        let i = status["P"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n == &admin.name)
            .unwrap();
        status["S"][i] = json!(32);
    }
    apply_precinct_status(&mut b, &status).unwrap();
    assert!(
        b.contests
            .iter()
            .all(|c| c.reporting_complete == Some(true))
    );
    status["S"][i] = json!(2);
    apply_precinct_status(&mut b, &status).unwrap();
    assert_eq!(b.contests[0].reporting_complete, Some(false));
}
#[test]
fn invalid_status_cannot_partially_mark_results_complete_and_old_batches_remain_readable() {
    let (b, status) = fixtures();
    let mut dimensions = status.clone();
    dimensions["S"].as_array_mut().unwrap().pop();
    let mut invalid_code = status.clone();
    invalid_code["S"][0] = json!("4");
    let mut invalid_name = status.clone();
    invalid_name["P"][0] = Value::Null;
    for invalid in [dimensions, invalid_code, invalid_name] {
        let mut copy = b.clone();
        assert!(apply_precinct_status(&mut copy, &invalid).is_err());
        assert!(
            copy.contests
                .iter()
                .flat_map(|c| &c.units)
                .all(|u| u.reporting_complete.is_none())
        );
    }
    let mut wrong = status.clone();
    wrong["P"][0] = json!("Unmatched county precinct");
    let mut copy = b.clone();
    assert!(apply_precinct_status(&mut copy, &wrong).is_err());
    let mut duplicate = status.clone();
    duplicate["P"][0] = duplicate["P"][1].clone();
    assert!(apply_precinct_status(&mut copy, &duplicate).is_err());
    let mut old = serde_json::to_value(&b).unwrap();
    for c in old["contests"].as_array_mut().unwrap() {
        c.as_object_mut().unwrap().remove("reporting_complete");
        c.as_object_mut().unwrap().remove("reporting_basis");
        for u in c["units"].as_array_mut().unwrap() {
            u.as_object_mut().unwrap().remove("reporting_complete");
            u.as_object_mut().unwrap().remove("reporting_basis");
        }
    }
    let old: Batch = serde_json::from_value(old).unwrap();
    assert!(old.contests.iter().all(|c| c.reporting_complete.is_none()));
}
#[test]
fn civic_county_completion_is_local_exactly_one_hundred_and_estimated() {
    let sources: Vec<Source> = serde_json::from_str(include_str!("../data/sources.json")).unwrap();
    let source = sources.iter().find(|s| s.id == "civic-cd5").unwrap();
    let original: Value =
        serde_json::from_str(include_str!("fixtures/civic-cd5-2026-setup.json")).unwrap();
    for (p, expected) in [
        (json!(100), Some(true)),
        (json!(99.9), Some(false)),
        (json!(0), Some(false)),
        (Value::Null, None),
    ] {
        let mut data = original.clone();
        data["percent_reporting"] = json!(100);
        data["region_results"]["limestone"]["percent_reporting"] = p;
        let b = civic::parse(source, &data).unwrap();
        assert_eq!(
            b.contests
                .iter()
                .find(|c| c.county == "Limestone")
                .unwrap()
                .reporting_complete,
            expected
        );
        assert!(
            b.contests
                .iter()
                .filter(|c| c.county != "Limestone")
                .all(|c| c.reporting_complete == Some(false))
        );
    }
}
