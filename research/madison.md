# Madison County precinct returns investigation

Checked 2026-11-03T20:29:06.923029+00:00. Research owner: bounded Madison source discovery.

## Finding

The November 3, 2026 election identity and candidate roster are verified from the Alabama Secretary of State’s [official Madison sample ballot](https://www.sos.alabama.gov/sites/default/files/sample-ballots/2026/gen/Madison-Sample.pdf), linked from the [2026 general sample-ballot index](https://www.sos.alabama.gov/alabama-votes/2026-general-election-sample-ballots). The downloaded original is `madison-sample.pdf`; `madison-sample.txt` is a reproducible `pdftotext -raw` extraction. The ballot explicitly names “GENERAL AND CONSTITUTIONAL AMENDMENT ELECTION”, Madison County, Alabama, November 3, 2026. It is a common ballot and explicitly warns that some offices appear only in applicable precincts.

No exact November Clarity election ID, payload, or precinct contest key could be verified. Automatic ingestion must remain disabled until a current payload establishes the election date and non-test identity. There are 49 distinct candidate/ballot-measure contests in the attached transcription, plus the separate straight-party selection. The provisional slug IDs in JSON are internal identifiers, **not publisher keys**.

## Discovery evidence and limitations

Firecrawl was attempted first through `/home/stolas/.local/bin/firecrawl-auth`: status authenticated with credits -1,000/1,000; an actual search returned HTTP 402. Ordinary web-tool search and direct HTTP retrieval were used as fallback. No challenge bypass was attempted.

| Source | Observed result | Consequence |
|---|---|---|
| [Current county election department](https://www.madisoncountyvotesal.gov/) | Direct HTTP 202 and empty body; web reader HTTP 403 | Cannot extract the current results link |
| [Current department absentee page](https://www.madisoncountyvotesal.gov/quick_link/absentee-voting/) | Search-indexed official page identifies department and 2026 calendar; reader 403 | Verifies domain/calendar lead only |
| [Legacy county election website](https://www.madisoncountyvotes.com/) | HTTP 530 | No results link extracted |
| [County Clarity root](https://results.enr.clarityelections.com/AL/Madison/) | Direct HTTP 403; web reader presents JavaScript verification challenge | Exact current election remains unresolved |
| [County government service directory](https://www.madisoncountyal.gov/residents/advanced-components/list-detail-pages/service-directory-list) | Search index has Election Results entry; direct/reader HTTP 403 | Cannot resolve entry target |
| [SOS 2026 election page](https://www.sos.alabama.gov/alabama-votes/voter/election-information/2026) | Current general section links ballots/certifications; inspected page did not expose November live results | Authoritative election/roster source, no live endpoint established |

Raw failure response bodies are retained under `.firecrawl/madison-*`, with URL/status evidence in `.firecrawl/madison-probe-log.json` for the exact historical payload probes. An empty HTTP 202 is not treated as source content.

The commercial [DDHQ Madison results page](https://votes.decisiondeskhq.com/elections/2026-11-03/alabama/madison) was a discovery lead only. Its indexed list says 32 races and omits unopposed races and some district offices present in the official common ballot; it is unsuitable for defining “every Madison race.” No commercial totals were imported.

## Current roster

Every candidate contest also has a write-in choice; ballot amendments have Yes/No. The separate straight-party selection must never be added to candidate totals. All names and offices in the table below come from the official sample ballot above.

| Proposed ID | Contest | Democratic choice | Republican choice |
|---|---|---|---|
| governor | Governor | Doug Jones | Tommy Tuberville |
| lieutenant-governor | Lieutenant Governor | Phillip Ensler | John Wahl |
| us-senate | U.S. Senate | Everett Wess | Barry Moore |
| us-house-5 | U.S. House District 5 | Andrew Sneed | Dale W. Strong |
| attorney-general | Attorney General | Jeff McLaughlin | Katherine Robertson |
| secretary-of-state | Secretary of State | Wayne Rogers | Caroleene Dobson |
| treasurer | State Treasurer | Rosilyn Houston | Young Boozer |
| auditor | State Auditor | Violet Edwards | Andrew Sorrell |
| agriculture | Commissioner of Agriculture and Industries | Ron Sparks | Corey Hill |
| psc-1 | Public Service Commission Place 1 | James O. Gordon | Matt Gentry |
| psc-2 | Public Service Commission Place 2 | Sheila D. McNeil | Jim Zig Zeigler |
| state-senate-2 | State Senate District 2 | Rudolph Valentino Drake | John Roberts |
| state-senate-3 | State Senate District 3 | John Dickey | Arthur Orr |
| state-senate-7 | State Senate District 7 | Jared Sluss | Sam Givhan |
| state-senate-8 | State Senate District 8 | Katie Kramer | Steve Livingston |
| state-senate-9 | State Senate District 9 | — | Wes Kitchens |
| state-house-4 | State House District 4 | Hanu Karlapalem | Parker Duncan Moore |
| state-house-6 | State House District 6 | — | Andy Whitt |
| state-house-10 | State House District 10 | Marilyn Lands | Aaron Thomas |
| state-house-19 | State House District 19 | Laura Hall | Donald Barnes |
| state-house-20 | State House District 20 | James Linderholm | James Lomax |
| state-house-21 | State House District 21 | Forrest Satterfield | Rex Reynolds |
| state-house-22 | State House District 22 | — | Ritchie Whorton |
| state-house-25 | State House District 25 | Allison T Montgomery | Phillip K. Rigsby |
| state-house-53 | State House District 53 | Anthony Daniels | — |
| supreme-court-7 | Associate Justice Supreme Court Place 7 | — | Brad Mendheim |
| supreme-court-8 | Associate Justice Supreme Court Place 8 | Ashleigh Meyer Dunham | Greg Shaw |
| civil-appeals-4 | Court of Civil Appeals Place 4 | — | Benjamin M. Bowden |
| civil-appeals-5 | Court of Civil Appeals Place 5 | — | Matt Fridy |
| criminal-appeals-4 | Court of Criminal Appeals Place 4 | — | Mary Windom |
| criminal-appeals-5 | Court of Criminal Appeals Place 5 | — | Riggs Walker |
| state-boe-8 | State Board of Education District 8 | Shatika Armstrong | Emily Jones |
| circuit-court-23-2 | Circuit Court 23rd Judicial Circuit Place 2 | — | Alison Strickland Austin |
| circuit-court-23-4 | Circuit Court 23rd Judicial Circuit Place 4 | — | Claude E. Hundley III |
| district-court-1 | Madison District Court Place 1 | Kimberly Kelley Rucker | Tim Douthit |
| district-court-4 | Madison District Court Place 4 | Jason M. Jones | John R. Allen |
| district-court-5 | Madison District Court Place 5 | Raven Beach | Jeanne Dowdle Rizzardi |
| sheriff | Madison County Sheriff | Brett Jones | Kevin H. Turner |
| tax-assessor | Madison County Tax Assessor | — | Cliff Mann |
| tax-collector | Madison County Tax Collector | Jeff Taylor | Valerie D. Miles |
| coroner | Madison County Coroner | — | Tyler Berryhill |
| license-director | Madison County License Director | — | Mark Craig |
| school-superintendent | Madison County Superintendent of Education | E. Quinn Headen | Keith Trawick |
| county-boe-3 | Madison County Board of Education District 3 | Austin Jones | Brian Brooks |
| amendment-1 | Statewide Amendment 1 | — | — |
| amendment-2 | Statewide Amendment 2 | — | — |
| amendment-3 | Statewide Amendment 3 | — | — |
| amendment-4 | Statewide Amendment 4 | — | — |
| madison-local-amendment | Madison County Local Amendment — Private sewer systems | — | — |

## Schema and existing implementation evidence

The user’s existing municipal implementation at `../Election-Maps/alabama/huntsville-d2-live-dashboard/server.mjs` uses Clarity election **127070**, with Aug. 25, 2026 municipal identity established in the follow-up dashboard tests. The runoff implementation at `../Election-Maps/alabama/huntsville-district-3-dashboard/source.mjs` explicitly requires election **127283**, date **9/22/2026**, non-test status. Both are historical municipal elections and are ineligible for the November dashboard. Primary election **126267** appeared only as a search lead and is likewise ineligible. Direct requests for historical `current_ver.txt`, `electionsettings.json`, and `details.json` returned 403 during this investigation; no downloaded historical payload was represented as current.

Existing implementation schema (to revalidate against a November payload):

- `{base}/current_ver.txt`: numeric results revision.
- `{base}/{revision}/json/en/summary.json`: array of contests; `K` key, `C` name, `CH` candidate names, `P` parties where present, `V` aligned candidate totals, `PR` precincts reporting, `TP` total precincts, `T` array of total votes by vote type. Legacy `TV` can remain zero despite populated totals.
- `{base}/{revision}/json/details.json`: `{Contests:[{K,P,V,...}]}`; `P` precinct names and `V[precinctIndex][candidateIndex]`. Join on `K`; align candidate index with that contest’s verified summary roster. Do not infer opaque `T`/status bitmasks without validation.
- `{base}/{revision}/json/en/electionsettings.json`: `settings.electiondetails.electionid`, `electiondate`, `istestmode`; top-level `websiteupdatedat` is publisher timestamp in the established municipal implementation.
- XML archive convention is known from other Clarity implementations (`{revision}/reports/detailxml.zip`), but no Madison November XML endpoint or payload was verified.

Recommended safeguards are the same ones already implemented by the municipal owner: exact date and election ID check, reject test mode, preserve all candidates/write-ins, reject duplicate precinct rows, check dimension alignment and nonnegative integer votes, reconcile precinct sums with summary candidate totals and `sum(T)`, retain prior valid data on errors, and distinguish no return from a confirmed zero. A zero-filled pre-election row is unreported unless another explicit reporting/completion signal establishes it. District results require race-specific precinct rows; county presidential totals cannot be allocated as district returns.

## Polling

No publisher minimum interval or documented refresh SLA was found. A conservative **60-second** version check is an implementation recommendation, not a publisher promise. Fetch the revision payload only when its revision changes; use single-flight requests/server caching, conditional headers when supported, and exponential backoff on 403/429/5xx. Do not poll blocked/challenge pages rapidly. Until a verified November endpoint exists, poll status remains disabled and manual official-source import remains the usable fallback.

## Completion evidence

- Official ballot retrieved HTTP 200, 348,598 bytes; SHA-256 recorded in `madison-source.json`.
- 49 unique contest slugs, each with unresolved publisher key (`null`); election ID/endpoint remain `null`, `enabled:false`.
- Current-election precinct payload **not verified**; no fabricated source identifiers or live returns.
- Required next action: resolve the actual November results link from the current county department, then inspect settings + summary + details together before activation.

## Current-feed verification after user supplied the results URL

The user supplied <https://results.enr.clarityelections.com/AL/Madison/127483/web.345435/#/summary>. Ordinary direct HTTP access to the exact version and JSON files succeeded despite the web reader's failures. `current_ver.txt` returned **382270**. Its settings identify election **127483**, **11/3/2026**, **2026 General Election**, entity **Madison**, parent **al**, `istestmode:false`, `showtestdatawatermark:0`; publication string **11/2/2026 5:00:25 PM CST**. These fields pass the production adapter's identity guard.

Saved decoded source entities are `research/madison-settings.json`, `madison-summary.json` and `madison-details.json`. The summary is served with HTTP gzip content encoding, so the Rust client enables automatic gzip decoding. There are **50 provider categories**: 49 candidate/ballot-measure contests plus straight-party voting. Every candidate vote is zero in this pre-election snapshot; zeros are identified as setup data, never reporting evidence or race calls. Straight-party votes are retained in source snapshots and excluded from the candidate-contest inventory. All 49 contest keys and all non-write-in candidate names were checked against the official ballot (case/punctuation normalized); the explicit mapping is retained in `data/sources.json`, with publisher titles/keys in `data/ballot.json`. Publisher titles truncate some offices (including CD5), so verified keys take precedence over title parsing.

Configured polling is 120 seconds, one central collector. Each poll validates identity and retrieves versioned summary/detail entities. A no-cache request and two-minute cache key on the revision endpoint reduce stale intermediary caching. No publisher minimum cadence or positive-return latency has been established. Historical precinct geography remains unverified against the current reporting-unit identities. The production runtime retains accepted source bodies with SHA-256 hashes and observation timestamps separately from these research captures.
