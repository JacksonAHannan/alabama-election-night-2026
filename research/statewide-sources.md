# Alabama statewide county feed verification

Observed 2026-11-03T21:31:01.931528+00:00. This note covers public feed discovery and verified setup payloads; it does not establish positive-result latency on election night.

## Coverage and identity

Twelve individual statewide ballot contests have verified public civicAPI feeds, each with all 67 Alabama counties. The first-party [null-district date inventory](https://civicapi.org/api/v2/race/search?province=AL&district=null&startDate=2026-11-03&endDate=2026-11-03&limit=1000) returned 14 records: these twelve contests and two chamber aggregates. The broader [Alabama date inventory](https://civicapi.org/api/v2/race/search?province=AL&startDate=2026-11-03&endDate=2026-11-03&limit=1000) returned 161 records, with no Public Service Commission or statewide judiciary contests. Race IDs were discovered from the [APR public inventory](https://www.alreporter.com/api/election-results) and confirmed through civicAPI, rather than guessed.

Each directly fetched payload explicitly identifies country US, province AL, election date 2026-11-03, `test_data: false`, `is_disputed: false`, and an actual JSON `null` district. U.S. Senate has scope `General`; all other feeds below have scope `Statewide`. Office, exact election name, candidate ID/name/party, county region ID/name/type/FIPS are retained in the accompanying [machine-readable profiles](statewide-sources.json).

| Dashboard contest | civicAPI race | Office | County coverage |
| --- | --- | --- | --- |
| us-senate | [86907](https://civicapi.org/api/v2/race/86907) | US Senate | 67 / 67 |
| governor | [86908](https://civicapi.org/api/v2/race/86908) | Governor | 67 / 67 |
| lieutenant-governor | [86909](https://civicapi.org/api/v2/race/86909) | Lieutenant Governor | 67 / 67 |
| attorney-general | [86910](https://civicapi.org/api/v2/race/86910) | Attorney General | 67 / 67 |
| secretary-of-state | [86911](https://civicapi.org/api/v2/race/86911) | Secretary of State | 67 / 67 |
| treasurer | [86912](https://civicapi.org/api/v2/race/86912) | State Treasurer | 67 / 67 |
| auditor | [86913](https://civicapi.org/api/v2/race/86913) | State Auditor | 67 / 67 |
| agriculture | [86914](https://civicapi.org/api/v2/race/86914) | Commissioner of Agriculture and Industries | 67 / 67 |
| amendment-1 | [179834](https://civicapi.org/api/v2/race/179834) | Referendum | 67 / 67 |
| amendment-2 | [179835](https://civicapi.org/api/v2/race/179835) | Referendum | 67 / 67 |
| amendment-3 | [179836](https://civicapi.org/api/v2/race/179836) | Referendum | 67 / 67 |
| amendment-4 | [179837](https://civicapi.org/api/v2/race/179837) | Referendum | 67 / 67 |

## Validation and canonical candidates

For each of the twelve raw first-party payloads, validation required exactly the 67 distinct Alabama county FIPS (`01001` through `01133`, odd suffixes), `County` region type, and one matching row per configured candidate in every county. All 804 county records passed. County sums equal the top-level vote total for each candidate. Every observed vote total and reporting percentage is zero, so these are setup payloads. Zero is not evidence that any county has reported. The raw bytes, SHA-256 hashes and unchanged test fixtures are retained separately.

No candidate name aliases are required for these twelve contests: every source candidate name matches the existing official ballot roster exactly. The four amendments use publisher party `Nonpartisan`; their exact choices `Yes` and `No` map to dashboard ballot choice labels `YES` and `NO`, while preserving publisher party in the profile. Candidate races supply the two named D/R candidates but no write-in rows. Missing write-ins must remain unknown rather than being represented as observed zero. [Governor payload](https://civicapi.org/api/v2/race/86908), [Amendment 1 payload](https://civicapi.org/api/v2/race/179834).

## Explicit feed gaps

Neither civicAPI date inventory contains a public statewide feed for the following eight existing ballot contests. This is an observed gap in these inventories, not a claim that no other publisher exists. These races should retain their Madison County official feed and display statewide coverage as partial. They must not show a statewide total as if the 66 unobserved counties had zero votes.

- Public Service Commission Place 1 (`psc-1`)
- Public Service Commission Place 2 (`psc-2`)
- Associate Justice Supreme Court Place 7 (`supreme-court-7`)
- Associate Justice Supreme Court Place 8 (`supreme-court-8`)
- Court of Civil Appeals Place 4 (`civil-appeals-4`)
- Court of Civil Appeals Place 5 (`civil-appeals-5`)
- Court of Criminal Appeals Place 4 (`criminal-appeals-4`)
- Court of Criminal Appeals Place 5 (`criminal-appeals-5`)

The State House (`159587`) and State Senate (`159588`) statewide-looking records are chamber aggregates with party/seats candidates. They are not statewide voter choices and must not be connected to the U.S. Senate contest. State Board of Education District 8 and local judicial races are also not statewide contests. [Full source inventory](https://civicapi.org/api/v2/race/search?province=AL&startDate=2026-11-03&endDate=2026-11-03&limit=1000).

## Integration contract

Use the existing single-source-per-contest/county selection rule. Healthy official Madison precinct returns can contribute Madison once, while civicAPI provides the other counties. County totals must remain county observations; do not distribute them to precinct rows or add them atop the same county’s official precinct totals. Validate exact identity, expected roster and all county coverage before accepting an update. Preserve the last accepted values on malformed, missing, disputed, test, or wrong-election updates. Publisher reporting percentages are estimates, not precinct counts.

civicAPI is a public aggregator rather than the election authority, requires attribution, and documents its public endpoints and sourcing in its [API documentation](https://civicapi.org/api-documentation) and [FAQ](https://civicapi.org/faq). The existing app attribution link to [civicAPI](https://civicapi.org/) should remain visible. These payloads do not identify a per-county upstream authority URL.

Artifacts: `research/statewide-sources.json` contains source entries, coverage, reconciliation and hashes; `.firecrawl/statewide-civic-*.json` preserves discovery/payload raw bytes; `tests/fixtures/civic-statewide-*-2026-setup.json` preserves parser fixtures. All findings were read-only public retrieval. No credentials, account actions, deployment or messaging were involved.

## Additional official and media gap check

A final bounded check made five ordinary public requests on November 3, 2026; all returned HTTP 200. The [official Alabama election-night root](https://www2.alabamavotes.gov/electionnight/) returned an empty ASP.NET form containing only hidden view-state fields and an empty div, with no election or result links. The [official 2026 election information page](https://www.sos.alabama.gov/alabama-votes/voter/election-information/2026) links November 3 general-election sample ballots and candidate/amendment certifications, but supplied no November 3 live-result link. The [official election data page](https://www.sos.alabama.gov/alabama-votes/voter/election-data) supplied historical and primary-result downloads, but no current November 3 county feed in the observed document. These observations do not justify guessing unlinked election IDs or endpoints.

The [WAFF public menu](https://s3.amazonaws.com/grayfilestore-waff/electionData/WAFF_Menu.json) still advertises the current Alabama November 3 configuration, whose [published results JSON](https://s3.amazonaws.com/grayfilestore-waff/electionData/WAFF_2797_Web.json) contains 77 races and `updatedDate: null`. It includes PSC Place 1 (`59123`), PSC Place 2 (`59122`), and Supreme Court Place 8 (`59125`), but each of those races has an empty `countyName`, and all 77 race records have empty county names. Those statewide totals cannot populate county-level views. The other five missing judiciary contests are absent from this current configuration. No additional county feed was verified by these targeted requests; the eight partial-coverage labels remain warranted.

The five exact responses are preserved as `.firecrawl/statewide-official-{enr,current,data}.html` and `.firecrawl/statewide-waff-{menu,current}.json`; per-response URL, HTTP status and SHA-256 are in `research/statewide-sources.json` under `additional_gap_check`. Existing accepted `source_entries` were not changed by this follow-up.
