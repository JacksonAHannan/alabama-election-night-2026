//! Legacy fixed-width precinct report, guarded by its election authority's current label.
use crate::{Batch, Candidate, Contest, Source, Unit, add, number};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub fn validate_authority(html: &str) -> Result<()> {
    // The authority retains an old 2020 results tab inside an HTML comment.
    // Only visible current publication context may authorize this standing URL.
    let mut visible = String::new();
    let mut rest = html;
    while let Some((before, after)) = rest.split_once("<!--") {
        visible.push_str(before);
        rest = after.split_once("-->").map(|(_, tail)| tail).unwrap_or("");
    }
    visible.push_str(rest);
    let lower = visible.to_lowercase();
    let link = lower
        .find("/temp/el30.htm")
        .context("Current precinct report link no longer published")?;
    let preceding = &lower[..link];
    let start = preceding
        .rfind("<strong>")
        .context("Missing explicit current election heading")?;
    let label = &preceding[start..];
    ensure!(
        label.contains("november 3, 2026") && label.contains("general election"),
        "Authority has not identified this as the November 3 general election"
    );
    ensure!(
        !["test", "mock", "practice", "primary", "runoff"]
            .iter()
            .any(|w| label.contains(w)),
        "Morgan authority labels current report TEST or another ineligible election; quarantined"
    );
    Ok(())
}
fn candidate(line: &str) -> Result<Candidate> {
    let cut = line
        .find(".  .")
        .context("Missing fixed-width vote separator")?;
    let label = line[..cut].trim().trim_end_matches('.').trim();
    let words = line[cut..]
        .split_whitespace()
        .filter(|w| *w != ".")
        .collect::<Vec<_>>();
    ensure!(!words.is_empty(), "Missing Morgan vote count");
    let votes = number(&json!(words[0].replace(',', "")))?;
    let (name, party) = if let Some((name, p)) = label.rsplit_once(" (") {
        (name, p.trim_end_matches(')'))
    } else {
        (label, "WRITEIN")
    };
    ensure!(!name.is_empty(), "Missing candidate name");
    Ok(Candidate {
        source_name: None,
        id: format!("{name}:{party}"),
        name: name.into(),
        party: party.into(),
        votes,
    })
}
pub fn parse(source: &Source, authority: &str, report: &str) -> Result<Batch> {
    validate_authority(authority)?;
    ensure!(
        source.county == "Morgan" && source.county_id == "01103",
        "Wrong report jurisdiction"
    );
    let up = report.to_uppercase();
    ensure!(
        up.contains("MORGAN COUNTY")
            && up.contains("2026 GENERAL ELECTION")
            && up.contains("NOVEMBER 3, 2026"),
        "Wrong Morgan election identity"
    );
    ensure!(
        !up.contains("TEST") && !up.contains("PRIMARY"),
        "Ineligible Morgan report"
    );
    ensure!(
        up.trim_end().ends_with("</HTML>") && up.contains("</PRE>"),
        "Incomplete Morgan HTML report"
    );
    let mut units = Vec::new();
    let mut current: Option<(String, String)> = None;
    let mut parsing = false;
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let mut published = None;
    for line in report.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("RUN DATE:") && published.is_none() {
            published = Some(trimmed.to_owned());
        }
        let words = trimmed.split_once(' ');
        if let Some((code, name)) = words {
            if code.len() == 4 && code.chars().all(|c| c.is_ascii_digit()) {
                ensure!(candidates.is_empty(), "Unfinished Morgan candidate block");
                current = Some((code.into(), name.trim().into()));
            }
        }
        if trimmed == "UNITED STATES REPRESENTATIVE, 5TH CONGRESSIONAL DISTRICT" {
            ensure!(!parsing, "Duplicate Morgan contest block");
            parsing = true;
            continue;
        }
        if parsing {
            if trimmed.starts_with("(VOTE FOR)") || trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("Over Votes") {
                let (id, name) = current.as_ref().context("Contest has no precinct header")?;
                ensure!(seen.insert(id.clone()), "Duplicate Morgan CD5 precinct");
                ensure!(
                    candidates.len() >= 2,
                    "Morgan CD5 candidate list incomplete"
                );
                let total = candidates
                    .iter()
                    .try_fold(0u64, |sum, c: &Candidate| add(sum, c.votes))?;
                units.push(Unit {
                    reporting_complete: None,
                    reporting_basis: None,
                    id: format!("{}:unit:{id}", source.id),
                    name: name.clone(),
                    county: source.county.clone(),
                    county_id: source.county_id.clone(),
                    administrative: ["ABSENTEE", "PROVISIONAL"]
                        .iter()
                        .any(|s| name.to_uppercase().contains(s)),
                    candidates: std::mem::take(&mut candidates),
                    total,
                });
                parsing = false;
                continue;
            }
            candidates.push(candidate(line)?);
        }
    }
    ensure!(
        !parsing && !units.is_empty(),
        "Missing or unfinished Morgan CD5 precinct data"
    );
    let mut totals = units[0].candidates.clone();
    for c in &mut totals {
        c.votes = 0;
    }
    for u in &units {
        ensure!(
            u.candidates.len() == totals.len(),
            "Morgan precinct candidate dimensions changed"
        );
        for (c, t) in u.candidates.iter().zip(totals.iter_mut()) {
            ensure!(
                c.name == t.name && c.party == t.party,
                "Morgan precinct candidate order changed"
            );
            t.votes = add(t.votes, c.votes)?;
        }
    }
    let total = totals.iter().try_fold(0u64, |s, c| add(s, c.votes))?;
    let version = format!("{:x}", Sha256::digest(report.as_bytes()));
    Ok(Batch{map_sha256:None,map_id:None,source_id:source.id.clone(),version,published_at:published,contests:vec![Contest{reporting_complete:None,reporting_basis:None,estimated_reporting:None,id:format!("{}:cd5",source.id),source_id:source.id.clone(),provider_id:"CD5".into(),name:"UNITED STATES REPRESENTATIVE, 5TH CONGRESSIONAL DISTRICT".into(),group:"Federal".into(),scope:"cd5".into(),county:source.county.clone(),candidates:totals,units,precincts_reporting:None,precincts_total:None,precinct_detail:true,detail_note:Some("Precinct report supplies no trustworthy reporting/completion flags; county contribution is the sum of its published CD5 precinct rows.".into()),total}]})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarantines_current_test_and_parses_only_with_eligible_authority() {
        let s:Source=serde_json::from_value(json!({"id":"morgan","county":"Morgan","county_id":"01103","url":"http://www.morgancountyprobate.com/elections.aspx","endpoint":null,"election_id":null,"enabled":false,"status":"fixture","note":"test-only"})).unwrap();
        let raw = include_str!("../tests/fixtures/morgan-2026-test-precinct.html");
        let authority = "<strong>November 3, 2026 - GENERAL ELECTION TEST RESULTS</strong><a href='/temp/el30.htm'>PRECINCT REPORT</a>";
        assert!(parse(&s, authority, raw).is_err());
        // Hypothetical eligible heading exists only inside this test, never in runtime data.
        let eligible = authority.replace(" TEST", "");
        let batch = parse(&s, &eligible, raw).unwrap();
        assert_eq!(batch.contests[0].units.len(), 1);
        assert_eq!(batch.contests[0].candidates.len(), 3);
        assert_eq!(batch.contests[0].total, 0);
        assert!(
            parse(
                &s,
                &eligible,
                &raw.replace("NOVEMBER 3, 2026", "NOVEMBER 5, 2024")
            )
            .is_err()
        );
    }
    #[test]
    fn fixed_width_votes_preserve_candidate_and_count() {
        let c = candidate(" ANDREW SNEED (DEM)  .  .  .  .    1,245   54.20").unwrap();
        assert_eq!(c.name, "ANDREW SNEED");
        assert_eq!(c.party, "DEM");
        assert_eq!(c.votes, 1245);
    }
}
