# CD-5 county source audit

Checked November 3, 2026, at 20:29 UTC (2:29 p.m. Central), before polls closed. Madison is covered by the separate Madison source audit. This audit found one additional current precinct-report endpoint, in **Morgan County**, but the county explicitly labels it **test results**. It must remain quarantined until that label changes and live provenance is revalidated. No live precinct feed was verified for the other counties inspected.

## District coverage

The [official congressional district description](https://strong.house.gov/about/our-district) identifies **Jackson, Madison, Lawrence, Morgan, Limestone, and the eastern portion of Lauderdale**. Lawrence must be included; Lauderdale is a split county. The [Secretary of State map index](https://www.sos.alabama.gov/alabama-votes/state-district-maps) currently links the [June 2026 congressional plan](https://www.sos.alabama.gov/sites/default/files/06-03-2026/2023%20Congressional%20Plan_Map.pdf), its [legal description](https://www.sos.alabama.gov/sites/default/files/06-03-2026/2023_Congressional%20Plan_BILL%20TEXT.pdf), and [June 10 certification](https://www.sos.alabama.gov/sites/default/files/2023_Congressional_Plan_CERTIFICATION_Jun_10_2026.pdf). The district text was retrieved directly; the map/legal PDFs were linked but not inspected in this subtask. Use contest-specific CD-5 votes from Lauderdale, never the countywide total for another contest.

## County findings

| County | Official reporting route | Verified status | Dashboard treatment |
| --- | --- | --- | --- |
| Morgan | [Probate elections page](http://www.morgancountyprobate.com/elections.aspx), [EL30 precinct report](http://www.morgancountyprobate.com/temp/el30.htm) | Current November 3, 2026 general roster, 41 reporting units, all zero; parent page explicitly says **GENERAL ELECTION TEST RESULTS** | Parser can be prepared; disable live ingestion until test status is removed and current feed rechecked |
| Limestone | [County Elections](https://www.limestonecounty-al.gov/Departments/Elections), [officially linked Viebit stream](https://limestone.viebit.com/player_live.php?hash=a1077d99-c0f2-4e05-a1a1-fac8dd0f9b14) | Page advertises a live video stream; no precinct machine-readable table/export found | Link for operator viewing; unknown precinct returns, no automatic import |
| Lauderdale | [Alabama county Probate Office](https://www.lauderdalecountyal.gov/home/departments/probate-office/), [sample-ballot page](https://www.lauderdalecountyal.gov/home/departments/probate-office/sample-ballot/) | Current sample ballot and SOS links, no current return endpoint found | Unknown; preserve eastern-county CD-5 restriction |
| Jackson | [Official Probate Office](https://jacksoncountyal.gov/243/Probate-Office) | Official indexed page readable through web retrieval; direct requests returned 404. Registration/forms/contact links, no results feed found | Unknown; no automatic import |
| Lawrence | [County Commission](https://lawrencecountyal.org/), [Probate Judge](https://lawrencecountyal.org/probate-judge/), [Board of Registrars](https://lawrencecountyal.org/board-of-registrars/) | Homepage readable; direct Probate/Registrars requests returned 403. Indexed official Probate page lists resources/contact, no results feed found | Unknown; no automatic import |

These are bounded discovery results, not proof that other counties will never publish precinct returns. None of the four unknown counties should display zero votes or zero percent reporting merely because no feed was found.

## Morgan report schema and activation gate

The official county homepage links the Probate Office. Its active Election Results tab contains the heading “November 3, 2026 - GENERAL ELECTION TEST RESULTS”; the precinct report itself says “UNOFFICIAL REPORT” and does **not** repeat the test warning. Checking only the payload header would miss that warning. HTTP access worked; HTTPS requests to the probate hostname timed out. The `morgancountyelections.com` route resolves to the same Probate elections page.

The report is a fixed-width HTML `<PRE>` EL30 report, not JSON. Its header states `RUN DATE:11/02/26`, `RUN TIME:11:07 AM`, `2026 GENERAL ELECTION`, `NOVEMBER 3, 2026`. Four-digit precinct headers identify 41 distinct units: 39 polling-place rows plus `0040 ABSENTEE` and `0041 PROVISIONAL`. Every unit includes CD-5 candidate rows: `ANDREW SNEED (DEM)`, `DALE W. STRONG (REP)`, and `WRITE-IN`, followed by overvotes and undervotes. All 41 ballot totals and all inspected CD-5 rows are zero. The absentee/provisional units must not be mapped as geographic precincts.

The [standing summary](http://www.morgancountyprobate.com/temp/el45.htm) and [canvass](http://www.morgancountyprobate.com/temp/el111.lst.txt) are also linked, but were not fetched in this subtask. The [2026 primary precinct report](http://www.morgancountyprobate.com/media/157084/el30.htm) and [2024 general precinct report](http://www.morgancountyprobate.com/media/129748/el30-precinct-report.htm) were fetched and have separate dates/rosters; they are history, never November 2026 returns.

The reproducible local inspector is `.firecrawl/cd5-inspect-morgan.py`; running it verified 41 units, three CD-5 candidate rows per unit, and zero ballots. Its structured output is `.firecrawl/cd5-morgan-schema.json`. A live collector should separately retain the authority-page status, validate election date/type and roster, preserve raw snapshots and timestamps, and reject active TEST labels. Do not promote an endpoint because it contains nonzero votes: tests can also contain nonzero votes.

## State fallback and historical precinct files

The [current 2026 information page](https://www.sos.alabama.gov/alabama-votes/voter/election-information/2026) has general-election sample ballots and candidate certifications, but no November general election-night endpoint on the inspected page. The legacy `ecode=1001285` county chooser and empty election-night root do not establish a 2026 precinct feed; no state collector was verified.

The [official election data archive](https://www.sos.alabama.gov/alabama-votes/voter/election-data) links [2024 general precinct results](https://www.sos.alabama.gov/sites/default/files/election-data/2024-12/2024-General%20Precinct%20Level%20Results.zip) and [2022 general precinct results](https://www.sos.alabama.gov/sites/default/files/election-data/2022-11/2022%20General%20Precinct%20Level%20Results.zip), along with 2026 primary precinct ZIPs. Archive links are verified; ZIP contents were not downloaded here. They may support later historical analysis after precinct-boundary and method matching, but are not evidence of live precinct publication.

## Retrieval evidence and limitations

Firecrawl was attempted first. The local CLI reported exhausted credits and the search returned HTTP 402. Research used public web search/open and ordinary direct HTTP requests as fallback; no challenge was bypassed. Raw pages and request outcomes are preserved under `.firecrawl/cd5-*`. `research/county-sources.json` records URLs, HTTP results, SHA-256 hashes, readiness, schema, and the disabled Morgan test endpoint.

Targeted searches often returned same-name counties in other states. In particular, `lauderdalecounty.org/election-results/` links a 2019 runoff and was not accepted as the Alabama authority; use the verified Alabama `.gov` site above. Jackson County TN/FL/GA results and Morgan County TN/GA results were excluded.
