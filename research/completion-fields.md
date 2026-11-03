# Reporting completion fields for turnout comparisons

Checked 2026-11-03 against the current first-party Madison Clarity client and revision-pinned payloads. Raw source copies and hashes are recorded in `completion-fields.json` and `.firecrawl/completion-*`.

## Madison precincts: use the separate status payload

The [current Madison client](https://results.enr.clarityelections.com/AL/Madison/127483/web.345435/main.2ea4cb41df122bbb.js) reads [`382270/json/status.json`](https://results.enr.clarityelections.com/AL/Madison/127483/382270/json/status.json). Its aligned arrays include `P` (exact precinct name), `S` (reporting status), `C` (separate certification indicator), `B` (ballots), and `R` (registered voters). The client joins a precinct name by exact `P.indexOf(name)` and reads its aligned `S`; it does not lowercase or strip punctuation. All 82 unique names in the current status payload exactly match the union of names in the verified contest detail payload. Contest detail membership remains necessary because the status payload describes the election's units globally, rather than an individual contest. [Sources: current client and status payload above; cached `research/madison-details.json`.]

The client declares these numeric values:

| Value | Client enum |
|---|---|
| 0 | Certified |
| 1 | NotReported |
| 2 | PartiallyReported |
| 3 | Participating |
| 4 | CompletelyReported |
| 8 | NotParticipating |
| 16 | Hidden |
| 32 | Cumulative |
| 64 | ElectionCompleted |

The client's `allCompletelyReported()` counts **exactly S = 4 or S = 64**. It uses equality, not bitwise membership. Those two values provide a verified, conservative completion gate for the requested turnout comparison. The client handles certification through `C` separately; this research does not expand the completion gate to an unobserved S = 0 or infer completion from `C`. Missing, unknown, partially reported, cumulative, hidden, or nonparticipating statuses must not receive a completed-unit percentage merely because votes are positive. [Source: current client, enum near byte 4,640,987 and status-service methods recorded in JSON evidence.]

Current observed revision 382270 has 80 units at S = 1 and two at S = 32; all C = 0. The cumulative units are `Provisional` and `PROVISIONAL ABSENTEE`; `ABSENTEE` is S = 1. Thus no current unit qualifies as completed yet. The `status.json` payload contains no election ID/date itself. Fetch it only under the same revision path whose election settings have already passed the election-ID/date/test guards, and archive it with that revision. [Source: revision-pinned status payload above.]

**Do not apply this enum to `settings.settings.precinctsreporting`.** That settings map currently contains ordinary setup values of 0, while the corresponding `status.json.S` values are 1. Its administrative values happen to include 32, but that agreement does not establish equivalent encodings. The separate status payload avoids this ambiguity. [Sources: current status payload and existing `research/madison-settings.json`.]

The [current summary](https://results.enr.clarityelections.com/AL/Madison/127483/382270/json/en/summary.json) has per-contest `PR` and `TP`. The client exposes their ratio as precinct reporting percentage, but the inspected code does not prove that PR excludes partial precincts. A stricter official county completion rule can require every participating geographic precinct in that contest to have S = 4 or S = 64. Do not promote a county to complete merely because its current votes are positive. [Sources: current client `getPrecinctRegistered`, `getTotalPrecincts`, `getPercentagePrecincts`; summary above.]

## civicAPI counties: estimated reporting percentage

The publisher's [CivicAPI documentation](https://www.computernewb.com/wiki/CivicAPI_Documentation) defines `percent_reporting` as an estimated reporting percentage. The [current AL-05 v2 feed](https://civicapi.org/api/v2/race/86904) supplies this value separately inside each named county in `region_results`, alongside county FIPS and candidate totals. All six observed county values are currently 0. [Sources: linked publisher documentation and API payload.]

A usable county gate is **that county's `region_results[region].percent_reporting === 100`**, described as publisher-estimated 100% reporting. Do not use the race-wide `percent_reporting` to mark every county complete, and do not describe this as certification or a guarantee that later corrections cannot occur. This application rule is an inference from the publisher's estimated reporting field, not a documented certified-final flag. A missing value, 99.9, positive votes alone, or a candidate winner flag does not meet the gate. [Sources: publisher documentation and current v2 field placement above.]

Firecrawl had already returned HTTP 402 in this project's research workflow, so direct public first-party HTTP retrieval and the web search fallback were used. Initial Clarity requests without a browser user agent returned 403; standard public requests with `User-Agent: Mozilla/5.0` and `Cache-Control: no-cache` returned the client and revision files with HTTP 200. No login, challenge solver, or account access was used.
