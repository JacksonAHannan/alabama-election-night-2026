# County-total fallback feeds

Checked November 3, 2026, around 20:57 UTC (2:57 p.m. Central), before polls closed. **A public, pollable county-total feed was verified for every CD-5 county and for Limestone’s HD-25 and SD-2 portions. Manual entry is unnecessary for those tracked races while the feed is available.** Precinct detail in Limestone still was not found.

## Accepted publisher feed

[civicAPI’s documented API](https://civicapi.org/api-documentation) is free and public, needs no key, and requires attribution for non-personal use. Its [FAQ](https://civicapi.org/faq) says data comes from Secretaries of State or equivalent election authorities, with local media used for some smaller elections. That is publisher provenance, not an independently verified county-to-state transmission chain. [Alabama Political Reporter’s current election page](https://www.alreporter.com/election-results) separately identifies its Alabama figures as Secretary of State data via civicAPI. The payload has no per-county upstream URL; dashboard labels should therefore say **civicAPI county totals**, rather than presenting it as a direct official county feed.

The [CD-5 JSON endpoint](https://civicapi.org/api/v2/race/86904) returned HTTP 200 and validates `election_name=Alabama US House 5`, `election_scope=General`, `country=US`, `province=AL`, `district=AL-05`, `election_date=2026-11-03T05:00:00.000Z`, and `test_data=false`. Every candidate has zero votes and every county zero estimated reporting in this pre-close snapshot. `last_updated=2026-10-30T18:52:46.878Z` is the publisher’s setup timestamp, not the time we observed results. Preserve this distinction.

| Tracked race | civicAPI race ID | Verified county regions |
| --- | --- | --- |
| CD-5 | [86904](https://civicapi.org/api/v2/race/86904) | Jackson, Lauderdale, Lawrence, Limestone, Madison, Morgan |
| HD-10 | [176509](https://civicapi.org/api/v2/race/176509) | Madison |
| HD-20 | [176519](https://civicapi.org/api/v2/race/176519) | Madison |
| HD-21 | [176520](https://civicapi.org/api/v2/race/176520) | Madison |
| HD-25 | [176524](https://civicapi.org/api/v2/race/176524) | Limestone, Madison |
| SD-2 | [176606](https://civicapi.org/api/v2/race/176606) | Limestone, Madison |
| SD-7 | [176611](https://civicapi.org/api/v2/race/176611) | Madison |

All seven endpoints were retrieved directly and checked for the same election date, Alabama identity, expected district and explicit false `test_data`. The publisher uses `election_scope=General` for CD-5 and `election_scope=Local` for all six state-legislative races; validate the expected per-race scope, not one shared scope. Raw snapshots are `.firecrawl/fallback-counties-civic-*.json`; reproducible source identities, hashes and candidate IDs are in `research/county-fallbacks.json`.

`region_results` is an object keyed by county slug; each row has `name`, `type=County`, `fips`, `percent_reporting`, and candidate objects with `name`, `party`, `votes`, `percent`, and call flags. Candidate IDs occur only in the top-level roster, so region names/parties need explicit matching. These provider snapshots list the two named candidates and omit a write-in row; missing write-ins must remain unknown rather than becoming a fabricated confirmed zero. CD-5 candidates are Dale Strong (Republican, top-level ID 219732) and Andrew Sneed (Democratic, 219733). County keys/FIPS: jackson 01071, lauderdale 01077, lawrence 01079, limestone 01083, madison 01089, morgan 01103. Lauderdale rows are for CD-5 only, not its separate CD-4 electorate.

The publisher [defines percent reporting](https://civicapi.org/faq) as an estimate of precincts or ballot batches. Do not display it as a verified number of precincts counted. Its calls are its own decision desk’s calls. This fallback does not establish official certification or enable district totals to be summed with duplicate county/precinct sources. Prefer official precinct returns when validated, choose one source per contest/county, and keep county results out of precinct map allocations. An initial 120-second central poll is reasonable; the publisher says its own system refreshes every 5–10 seconds, which is not a guaranteed latency.

## Secondary and rejected routes

[APR’s JSON mirror](https://www.alreporter.com/api/election-results?race=86904) returned HTTP 200, exact date `2026-11-03`, `status=ready`, `historical=false`, `refreshFailed=false`, and six `regions` with the same candidates/counts. The public page JS explicitly fetches `/api/election-results?race=ID`. APR is useful as a secondary transport, but the direct civicAPI endpoint retains the essential explicit `test_data` guard and therefore is the preferred collector input. APR also exposes HD-25 and SD-2 county detail at the same race IDs. Raw APR responses and client JS are retained under `.firecrawl/fallback-counties-apr-*`.

[Morgan’s summary endpoint](http://www.morgancountyprobate.com/temp/el45.htm) returned HTTP 200, a current November 3, 2026 General EL45 fixed-width `<PRE>` report, `0 OF 41 PRECINCTS COUNTED`, and zero CD-5 counts. Candidates are ANDREW SNEED (DEM), DALE W. STRONG (REP), WRITE-IN. Its [parent authority page](http://www.morgancountyprobate.com/elections.aspx) labels the active section **GENERAL ELECTION TEST RESULTS**. This endpoint must remain quarantined alongside EL30; a summary URL does not remove the parent test warning. civicAPI is a separate publisher feed explicitly declaring `test_data=false`; label its provenance accordingly.

[DDHQ’s current CD-5 page](https://votes.decisiondeskhq.com/races/2026-11-03/alabama-us-house-5-general-election) visibly lists the six current counties, but direct HTTP requests returned 429/security checkpoint. No automated DDHQ collector was accepted. [WAFF’s Alabama page](https://www.waff.com/alabama-election-results/) and [WVTM’s CD-5 coverage](https://www.wvtm13.com/article/alabama-election-results-2026-us-house/74194798) embed current AP reports; the linked AP iframe returned 403, so those were not accepted as pollable sources. [The state election-night root](https://www2.alabamavotes.gov/electionnight/) returned only an empty form; the [2026 election information page](https://www.sos.alabama.gov/alabama-votes/voter/election-information/2026) had no linked November live-report endpoint in its General section. Historical `ecode` URLs were not promoted by inference.

Firecrawl credits were already exhausted (HTTP 402 in prior source research), so this bounded follow-up used web retrieval and ordinary direct HTTP without bypassing access challenges. All fetched material owned by this follow-up uses `.firecrawl/fallback-counties-*` names.

## Verification

A focused local assertion check passed against all seven saved source responses: SHA-256 hashes, November 3 identities, per-race General/Local scopes, US/AL jurisdiction, expected districts, explicit false test flags, setup-zero candidate counts and county detail. No application code was changed by this source-discovery subtask.
