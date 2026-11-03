# Limestone County automatic fallback

Checked November 3, 2026, 20:58 UTC (2:58 p.m. Central), before polls closed.

**Manual entry is unnecessary for Limestone’s CD-5, SD-2, and HD-25 county totals:** a current public media JSON feed from Alabama Political Reporter supplies candidate-specific county results for all three. Its [election-results page](https://www.alreporter.com/election-results/) links the current race pages, and the public first-party endpoints below return HTTP 200 with `date: 2026-11-03`, `historical: false`, `status: ready`, `refreshFailed: false`, the current candidate roster, and a `regions` entry for Limestone FIPS `01083`.

| Race | Publisher page | Machine-readable county feed | Current Limestone candidates |
| --- | --- | --- | --- |
| CD-5 | [District 5](https://www.alreporter.com/election-results/86904) | [race=86904](https://www.alreporter.com/api/election-results?race=86904) | Dale Strong (Republican), Andrew Sneed (Democratic) |
| SD-2 | [Senate District 2](https://www.alreporter.com/election-results/176606) | [race=176606](https://www.alreporter.com/api/election-results?race=176606) | Rudolph Valentino Drake (Democratic), John Roberts (Republican) |
| HD-25 | [House District 25](https://www.alreporter.com/election-results/176524) | [race=176524](https://www.alreporter.com/api/election-results?race=176524) | Allison T Montgomery (Democratic), Phillip K. Rigsby (Republican) |

The endpoints currently publish setup zeros and zero reporting. This verifies the automatic retrieval route and current election identity; actual post-close updating has not yet been demonstrated. Publisher `checkedAt` is November 3, 2026; each race’s `updatedAt` remains October 30. Treat these as awaiting results. `reporting` is county-level publisher reporting and must not be presented as a number of precincts reporting. These are media returns published by APR, not official Limestone results; preserve that attribution.

## Schema and ingestion boundaries

The top-level response has `date`, `checkedAt`, `status`, `historical`, `electionKey`, `refreshFailed`, and `races[]`. Within the requested race, `id`, `name`, and `district` establish identity; `summary: false` distinguishes detailed county payloads. `regions[]` contains county `id`, `fips`, `name`, `reporting`, and `candidates[]`. Each county candidate has `id`, `name`, `party`, `votes`, `winner`, and `percent`. County candidate IDs differ from the statewide candidate IDs, so use an explicit verified roster crosswalk. All requested race feeds have two candidates in Limestone. CD-5 covers six counties; SD-2 and HD-25 list Limestone and Madison. The latter feeds represent each county’s portion of the district, not countywide votes from other contests.

Select one source per race/county. Prefer Madison’s official precinct returns and use APR for a county without a verified precinct feed; do not add APR’s full-district total to Madison’s total or add APR’s Madison region to official Madison returns.

Raw JSON, response field observations, SHA-256 provenance, election identity, and activation limits are recorded in [limestone-fallback.json](limestone-fallback.json), with raw snapshots under `.firecrawl/fallback-limestone-apr-*`.

## Other routes inspected

The [official Limestone Elections page](https://www.limestonecounty-al.gov/Departments/Elections) still advertises a [live video stream](https://limestone.viebit.com/player_live.php?hash=a1077d99-c0f2-4e05-a1a1-fac8dd0f9b14). The [official Probate Judge page](https://www.limestonecounty-al.gov/Departments/Probate-Judge) routes election users back to Elections. No official precinct table or machine-readable results export was found in the bounded audit. That is not proof the county will never publish one.

[DDHQ’s current Limestone index](https://votes.decisiondeskhq.com/elections/2026-11-03/alabama/limestone) and its [CD-5](https://votes.decisiondeskhq.com/races/2026-11-03/alabama-us-house-5-general-election), [SD-2](https://votes.decisiondeskhq.com/races/2026-11-03/alabama-state-senate-2-general-election), and [HD-25](https://votes.decisiondeskhq.com/races/2026-11-03/alabama-state-house-25-general-election) pages verify those races appear in Limestone. Web retrieval showed candidate totals and county summary tables; direct HTTP returned a 429 challenge, so no automatic collector was verified there. A leader/total-only county table cannot reconstruct every candidate’s votes.

[WAFF’s Alabama results page](https://www.waff.com/alabama-election-results/) embeds AP layouts `110246/32468.html` and `110246/32403.html`; both returned HTTP 403 directly and were unavailable through web retrieval. Their data was not inferred or used. WAFF’s accessible local-race JSON is being audited separately and is not the Limestone county split source for these district races.

Firecrawl was unavailable because credits were exhausted (HTTP 402, recorded in the earlier source audit). This bounded follow-up used public web retrieval and ordinary HTTP requests without bypassing access controls. The useful fallback is APR; no manual video transcription is needed for the three target Limestone races once that collector is connected.
