# AL-02 larger-county precinct-feed investigation

Checked 2026-11-03, approximately 21:43 UTC. This note owns Montgomery, Elmore, and Houston checks; Mobile was inspected before the current district boundary was confirmed. Other AL-02 counties are covered by the parallel county-source investigation.

## Result

No eligible, automatic **November 3, 2026 general-election precinct return feed** was verified for these three current AL-02 counties. No Clarity election identifier or map identifier was published by the inspected pages, so none was inferred or guessed. This is a bounded negative finding, not a claim that precinct results cannot appear later. Retain the verified district-specific county feed as fallback.

## Montgomery — FIPS 01101

The [official election-results page](https://www.montgomeryvotesal.gov/elections-information/election-results) returned HTTP 403 to ordinary direct HTTP and browser retrieval. Its search-indexed version lists a 2026 archive year but exposes no current November result feed. The [official elections information page](https://www.montgomeryvotesal.gov/elections-information) lists the November 3 general election. The [Secretary of State sample ballot](https://www.sos.alabama.gov/sites/default/files/sample-ballots/2026/gen/Montgomery-Sample.pdf) independently confirms that the current ballot includes Congressional District 2, Shomari C. Figures and Rhett Marques. No access challenge was bypassed. Status: **access limited; current precinct feed unverified**.

## Houston — FIPS 01069

The [official county results archive](https://houstoncountyal.gov/probate/election_results/) was fetched successfully. Its newest election is August 11, 2026 Special Primary Congressional District 2; it also lists the June and May primaries and older general elections. There is no November 3 general-election section or machine-readable live-results link in the inspected archive. The published [August 11 official precinct report](https://houstoncountyal.gov/files/election_results/primary_election_congressional_district_2_2026_08_11_68552.pdf) contains precinct rows, including separate absentee and provisional units, but **is a historical primary and cannot supply current general returns**. The [upcoming-elections page](https://houstoncountyal.gov/probate/upcoming_elections/) lists the November 3 election and its ballot. The [state-hosted current sample ballot](https://www.sos.alabama.gov/sites/default/files/sample-ballots/2026/gen/Houston-Sample.pdf) confirms CD-2 Figures/Marques. Status: **historical precinct coverage only; current feed not found**.

## Elmore — FIPS 01051

The [official county election page](https://www.elmoreco.org/elmore-county-elections) was readable through ordinary web retrieval, though direct HTTP returned 403. It publishes a November 3 sample ballot, a voting-precinct list and precinct map, and an election-results label without an exposed linked return endpoint. The [current state-hosted sample ballot](https://www.sos.alabama.gov/sites/default/files/sample-ballots/2026/gen/Elmore-Sample.pdf) confirms CD-2 Figures/Marques. The indexed [state-hosted August 11 county summary](https://www2.alabamavotes.gov/electionNight/countyResultsByContest.aspx?cid=29&ecode=1001300) is explicitly a special primary and a county summary, not current precinct coverage. Status: **current precinct feed not found**.

## State election-night index cross-check

The [state's published county selector](https://www2.alabamavotes.gov/electionnight/chooseCounty.aspx?ecode=1001015) was fetched successfully. Montgomery, Houston and Elmore are gray, unlinked labels in the saved HTML. The page explains unlinked counties have not yet supplied results to that portal. Its observed state provides no currently published county-to-precinct endpoint. Do not treat the selector's URL election code as a substitute for validated election metadata.

## Mobile — outside current AL-02

The [official election information page](https://probate.mobilecountyal.gov/elections/election-information) links an [election-results PDF index](https://benchmark.probate.mobilecountyal.gov/uploads/eResults/ElectionResults.pdf), most recently listing the August 11 special primary. The [November 3, 2026 state-hosted sample ballot](https://www.sos.alabama.gov/sites/default/files/sample-ballots/2026/gen/Mobile-Sample.pdf) lists CD-1 Clyde Jones/Jerry Carl, matching the current district configuration rather than the 2024 remedial AL-02 configuration. Mobile must not be added merely because it belonged to AL-02 in 2024.

## Evidence and eligibility

Raw ordinary HTTP evidence is saved under `.firecrawl/al02-large-*`. `al02-large-downloads.json` records PDF/source URLs, response status, content type, byte count, and SHA-256. Raw official-page HTML is retained separately, including the Montgomery and Elmore 403 responses. The two published historical precinct/index PDFs were retained without modifying their contents.

There are no eligible candidate-by-precinct current rows to reconcile, no verified current map identity to hash, and no adapter-compatible current source configuration from this investigation. Do not join historical precinct names or district membership to current return rows by resemblance. Firecrawl had already returned HTTP 402 in the parent investigation; ordinary public web/direct HTTP was used as the documented fallback.
