# County results from public media dashboards

Investigated November 3, 2026. This is source evidence, not an official county feed.

## Working county fallback

[Alabama Political Reporter](https://www.alreporter.com/election-results) publishes a current election dashboard. Its public JavaScript calls `/api/election-results?race=ID` with `Accept: application/json`, `cache: no-cache`; no account, token, or paid endpoint was used. The publisher describes its source as Alabama Secretary of State via [civicAPI](https://civicapi.org/results), with 30-second updates. That attribution is the publisher's statement; direct county provenance was not independently verified.

Each independently requested legislative race returned HTTP 200, date `2026-11-03`, `historical:false`, `status:ready`, `refreshFailed:false`, and `summary:false`. The full `races[0].regions[]` array contains county identity (`id`, `fips`, `name`), reporting estimate, and all candidate names, parties and vote totals. All observed votes/reporting were setup zeros. County candidate IDs are synthetic names with prefixes, while top-level candidates have numeric IDs; join by normalized candidate name/party with race identity safeguards, not by expecting IDs to match.

| Race | APR ID | District | County scope |
|---|---|---|---|
| [Alabama State House 10](https://www.alreporter.com/api/election-results?race=176509) | 176509 | `AL-HD-010` | Madison 01089 |
| [Alabama State House 20](https://www.alreporter.com/api/election-results?race=176519) | 176519 | `AL-HD-020` | Madison 01089 |
| [Alabama State House 21](https://www.alreporter.com/api/election-results?race=176520) | 176520 | `AL-HD-021` | Madison 01089 |
| [Alabama State House 25](https://www.alreporter.com/api/election-results?race=176524) | 176524 | `AL-HD-025` | Limestone 01083, Madison 01089 |
| [Alabama State Senate 2](https://www.alreporter.com/api/election-results?race=176606) | 176606 | `AL-SD-002` | Limestone 01083, Madison 01089 |
| [Alabama State Senate 7](https://www.alreporter.com/api/election-results?race=176611) | 176611 | `AL-SD-007` | Madison 01089 |

Raw full payloads are `research/media-county-sample-{id}.json`; the discovery/index payload is `research/media-county-sample.json`. A machine-readable endpoint and scope manifest is `research/media-county-source.json`.

`checkedAt` is the publisher's check time, while race `updatedAt` was October 30 setup metadata. Neither should be presented as a fresh vote report. Reject wrong election dates, historical payloads, missing county identity, wrong race/district, and mismatched candidates. Treat reporting as an estimate, not precincts counted. Madison official precinct results should take precedence when available; replace a county whole, and never add its county total to its precinct sum.

## WAFF district totals (secondary fallback only)

[WAFF's public page](https://www.waff.com/politics/election-results/) embeds Fusion content. Its first-party JS defines `https://s3.amazonaws.com/grayfilestore-waff/electionData/WAFF_{id}_Web.json` and a [public election menu](https://s3.amazonaws.com/grayfilestore-waff/electionData/WAFF_Menu.json). The [current general election JSON](https://s3.amazonaws.com/grayfilestore-waff/electionData/WAFF_2797_Web.json) returned HTTP 200, title `Alabama Nov. 3 General Election`, id 2797 and `updatedDate:null`. Every general-election `Race[].countyName` is empty, so these are district/state totals, unsuitable for county sums.

Its requested race IDs are CD5 `59118`, HD10 `59085`, HD20 `59089`, HD21 `59090`, HD25 `59091`, SD2 `59064`, SD7 `59067`. Candidate totals are `Race[].candidates[].voteCount` (strings), name components and party. [Local election JSON](https://s3.amazonaws.com/grayfilestore-waff/electionData/WAFF_2758_Web.json) has named Limestone commission races but no Limestone portion of CD5/SD2/HD25. Raw HTML, JS, menu and JSON are under `.firecrawl/fallback-media-*`.

WHNT returned HTTP 403 and DDHQ returned HTTP 429/challenge to ordinary requests; no bypass was attempted. The working APR endpoint removes the need to use those sources for this bounded task.
