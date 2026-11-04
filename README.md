# Alabama Election Night 2026

[Open the shared dashboard](https://jacksonahannan.github.io/alabama-election-night-2026/) · [GitHub repository](https://github.com/JacksonAHannan/alabama-election-night-2026)

A Rust/Axum dashboard with a local collector and a GitHub Pages edition following the national dashboard’s Libre Caslon/Libre Franklin design. Covers all **49 Madison sample-ballot contests**, including uncontested offices and ballot measures, plus dedicated AL-02, CD-5, HD-10/20/21/25 and SD-2/7 views. Senate, Governor and all 20 statewide ballot contests also have Alabama county views, with a Coverage switch back to Madison precinct detail. Statewide and district totals are explicitly geographic subtotals where coverage is incomplete.

## Run

```bash
cd /home/stolas/Workspace/Projects/alabama-election-night-2026
./run-dashboard.sh
```

Open **http://127.0.0.1:8052/**. The national dashboard on port 8050 is untouched. The local server binds to loopback. The shared edition is published through GitHub Pages, as described below.

Rust and Cargo are runtime/build prerequisites. Browser code is buildless and uses local OFL fonts; no Node installation, remote map tiles, API keys or Python runtime is required for production.

```bash
cargo build --locked
# Serve with the central collector (default)
target/debug/alabama-election-night serve --port 8052
# Read-only cached viewing, without a collector
target/debug/alabama-election-night serve --port 8052 --no-collector
# One collection attempt; inspect individual outcomes in SQLite or the API
target/debug/alabama-election-night collect-once
```

The launcher builds the debug executable for fast local iteration. All commands accept `--root PATH`; the default is the project directory. Only one collector may own a runtime; its advisory file lock is released when the process exits. Stop the foreground server with Ctrl-C. To use another port, invoke the binary directly.

## Results update alerts

The dashboard checks for a new snapshot every 30 seconds, with its interval shown beside Refresh view. A Results alerts strip announces changed race votes, first positive returns, corrections and changed precinct/county vote observations while the page stays open. New alerts stay visible as you scroll and link to the affected race; expand or collapse Recent updates, or use Dismiss alerts. The open tab retains the latest 20 events. Initial page load, unchanged snapshots, retrieval-time changes and setup zeros do not create alerts. Browser polling does not change the source collection or GitHub publication intervals; new results depend on the publisher and delivery pipeline.

## GitHub Pages

The public edition uses the same frontend and prepared references, with accepted JSON/CSV snapshots exported by the Rust collector. County names are omitted from map polygons at all zoom levels; hover, keyboard inspection, selected details and tables retain county identities.

```bash
cargo build --locked
target/debug/alabama-election-night export-site --output site
# Preview under a project subpath, like GitHub Pages:
python -m http.server 8053 --bind 127.0.0.1
# Open http://127.0.0.1:8053/site/
```

The export contains frontend assets, prepared public references and cached accepted observations. It excludes runtime databases, raw captures, credentials and task history, and removes workstation home-directory prefixes from published provenance. All paths work beneath the repository’s Pages URL. Browsers refresh the published snapshot every 30 seconds; sources are fetched centrally, rather than per visitor. Staleness is recalculated in the browser so a stalled publication cannot keep showing old results as fresh.

`deployment/github-pages.yml` is the verified workflow template, installed at `.github/workflows/pages.yml` in GitHub. It runs on main-branch changes, manual dispatch and an offset five-minute schedule during November 3–5 UTC. Scheduled runs are limited to the 2026 election year. GitHub schedules can be delayed; check the displayed source retrieval times. The job caches the last accepted SQLite state with a unique key and uses `deployment/initial-results.sqlite` only as an initial fallback. This seed contains accepted public aggregate election observations, not synthetic test results. Source failures retain last-good returns. The seed and cache never enter the public Pages bundle.

Pinned official checkout/cache/Pages actions build and deploy the `site/` artifact. Source code and the workflow are public in GitHub; the CLI OAuth login cannot write workflow files, so the connected GitHub app created that file successfully. [Deployment evidence and GitHub documentation](research/github-pages-deployment.md).

### Local precinct and county relay

Madison’s Clarity site returns an AWS WAF browser challenge to GitHub runners, while the collector on this workstation can fetch its public feed normally. A local relay bridges that access difference and cloud county-source failures: the existing collector polls Madison every two minutes, and `deployment/publish-relay.py` uploads accepted raw Madison source files and all configured civicAPI county-feed snapshots to the public `observations` branch every two minutes. `madison.json` retains the precinct transport format; `county-feeds.json` carries independently validated county-source observations. Each upload requests a Pages rebuild; the independent five-minute cloud schedule is a backup. Collection, queue and deployment delays can add several minutes. [Original access diagnostic](research/github-pages-deployment.md#observed-madison-access-limitation).

The Pages job imports both relay files and runs the existing election, candidate/county/precinct reconciliation, completion-status and map checks again. Each county feed is validated separately, so one rejected source cannot block the others. Exact configured URLs and SHA-256 hashes identify each transported document. Original observation and attempt times survive every import; older observations cannot replace newer data. Civic feeds also reject a publication timestamp older than their last accepted publication, including a cached setup response retrieved later. A genuine correction with a newer publication time remains eligible. GitHub continues collecting the 20 county feeds independently. Missing or invalid relay data falls back to normal source collection and last-good observations; stale or failing precinct data allows healthy county fallbacks where available. Sources & coverage identifies relayed observations and their delivery time separately from their actual retrieval time.

**Keep this computer awake, online and logged in for automatic precinct updates.** The user services restart failed processes and start at login. They do not change sleep, lid-close or power settings. If the computer sleeps, disconnects or shuts down, the public site stays available and county feeds continue; Madison precinct observations become visibly stale after ten minutes. The existing GitHub login supplies upload credentials locally; tokens never enter Git, the relay file or the public site. No browser-challenge bypass is used.

The checked-in user-unit templates are installed under `~/.config/systemd/user/` on this workstation:

```bash
# Inspect the local collector and two-minute publisher
systemctl --user status alabama-election-collector.service alabama-election-relay.timer
journalctl --user -u alabama-election-relay.service -n 25
# Send the latest accepted observation now
systemctl --user start alabama-election-relay.service
# Stop automatic local collection and uploads after election night
systemctl --user disable --now alabama-election-relay.timer alabama-election-collector.service
```

The local dashboard remains at port 8052. When the service is running, use `systemctl --user restart alabama-election-collector.service` after rebuilding instead of starting a second collector. Source code changes use `main`; the publisher commits only the two public source-document files in a separate repository under ignored `runtime/relay-publisher`, leaving the working tree untouched. Relay operation additionally requires Python 3, Git and the existing authenticated GitHub CLI. Rust export/import commands can also be run directly:

```bash
target/debug/alabama-election-night export-relay --output runtime/madison-relay.json --county-output runtime/county-relay.json
target/debug/alabama-election-night collect-once --relay runtime/madison-relay.json --county-relay runtime/county-relay.json
```


## Source coverage and Limestone

**Limestone updates automatically** for CD-5, HD-25 and SD-2 through the public [civicAPI feed](https://civicapi.org/). Manual entry is unnecessary while that feed is available. Precinct detail remains unavailable there, so the dashboard uses explicitly labeled county totals.

| Coverage | Active source | Reporting level |
|---|---|---|
| Senate, Governor, Lt. Governor, Attorney General, Secretary of State, Treasurer, Auditor, Agriculture and statewide amendments 1–4 | civicAPI per-race feeds | All 67 counties; preferred official Madison contribution counted once |
| PSC places 1–2 and six statewide judicial contests | Madison official feed only | Statewide views explicitly mark the remaining 66 counties unavailable |
| AL-02 | [civicAPI race 86901](https://civicapi.org/api/v2/race/86901) | District-specific candidate totals for all 16 current counties; no current precinct feed verified |
| All 49 Madison contests | [Official Clarity election 127483](https://results.enr.clarityelections.com/AL/Madison/127483/web.345435/#/summary) | Preferred precinct source; 49 contests and 3,012 reconciling contest-unit rows |
| Jackson, Lauderdale, Lawrence, Limestone, Morgan CD-5 | [civicAPI CD-5](https://civicapi.org/api/v2/race/86904) | County candidate totals for the district portion of each county |
| Limestone HD-25 and SD-2 | [HD-25](https://civicapi.org/api/v2/race/176524),[SD-2](https://civicapi.org/api/v2/race/176606) | County candidate totals for the selected legislative seat |
| Madison fallback for all 7 requested seats | civicAPI per-race feeds | County totals used if the preferred precinct source fails or becomes stale |
| Morgan direct county precinct report | Official probate preflight | Still quarantined while its authority labels it TEST RESULTS; independent civicAPI county returns remain available |

Both active providers currently publish pre-close setup zeros. The new statewide feeds recovered on the next scheduled collection after transient HTTP 502 responses; failed attempts retain accepted snapshots and source-health messages. Their positive election-night latency is unproven. All sources are centrally polled at 120 seconds; Morgan’s authority preflight is every 10 minutes. civicAPI is a public results publisher with attribution, not a direct county authority connection; its API explicitly identifies the election and test status. Its county rows currently omit write-ins, so those counts remain unknown outside sources supplying them. Its reporting percentage is an estimate, not an exact precinct count.

Only one contribution per race and county is counted. A healthy precinct source wins over a county fallback; a healthy county source can replace a failed or stale precinct source. Raw snapshots remain separate. County totals are never added to that same county’s precinct sums and never produce artificial precinct rows.

CD-5 includes Jackson, Lawrence, Limestone, Madison, Morgan and **eastern Lauderdale**; [official district description](https://strong.house.gov/about/our-district). Feeds supply contest-specific county portions. Details: [statewide feed research](research/statewide-sources.md), [statewide county reference provenance](research/statewide-reference.md), [county fallback research](research/county-fallback-sources.md),[Limestone audit](research/limestone-county-fallback.md),[Madison source](research/madison.md). Firecrawl returned HTTP 402; public web/direct HTTP retrieval was used without bypassing access controls.

## AL-02

Open [AL-02](http://127.0.0.1:8052/#race/cd2) or choose CD-2 in the quick links. The dashboard tracks 50 contests: the original 49 Madison ballot contests plus AL-02. Figures and Marques are verified against the [current Houston sample ballot](https://www.sos.alabama.gov/sites/default/files/sample-ballots/2026/gen/Houston-Sample.pdf); the publisher’s “Shomari Figures” label is preserved while the displayed name uses the official “Shomari C. Figures.”

The view follows the current 2026 plan and its 16-county extent. Covington and Elmore are partial counties. This differs from the district used in the 2024 House election; historical comparisons use 2024 **presidential** results modeled onto the current boundary. `data/al02-geography.json` clips county outlines to that district, and `data/al02-benchmarks.json` uses the existing Census adult-point method without allocating live votes. Fourteen whole-county references reproduce the source totals exactly; the two split counties retain fractional estimated counts. All 559,067 selected Census adults are accounted for. [Reference provenance](research/al02-reference.md).

Every constituent county’s official election pages were checked. **No eligible current November 3 precinct feed was verified.** Houston, Henry and Russell offer older-election reports; Montgomery restricts automated access. Other inspected pages provide election information, ballots or polling locations without a current precinct returns endpoint. The view lists all 16 source checks and uses the validated automatic county fallback. This finding describes inspected sources before polls close, not a guarantee that precinct reports will never be published. [County-by-county audit and primary links](research/al02-sources.md).

## Data integrity and interactions

- The [official Madison sample ballot](https://www.sos.alabama.gov/sites/default/files/sample-ballots/2026/gen/Madison-Sample.pdf) supplies all 49 contests and candidate names. Each current provider contest key was checked against that roster and retained in `data/sources.json`. Straight-party voting remains in raw snapshots; it is not added to candidate totals or listed as a candidate race.
- Candidate results, reporting counts, source publication/retrieval times, county coverage, search, starred races, district navigation, precinct pagination/sorting/selection, historical maps, target controls, correction history and CSV exports are available. Stars are saved only in this browser.
- The central collector validates election date, county/state, election ID/name and test status. Source-summary totals are used once; precinct detail appears only when each candidate’s unit sum reconciles. Failed detail reconciliation preserves a qualified summary, never an invented precinct distribution. Zero votes do not prove a precinct is reporting.
- Administrative units (including ABSENTEE, PROVISIONAL ABSENTEE and Provisional) remain separate from residential map polygons. Candidate votes mean votes for choices, not ballots cast or turnout. Reporting counts do not establish complete counting.
- `runtime/results.sqlite` retains last good batches, source errors, observation history and SHA-256 evidence paths. Response entities after HTTP content decoding are stored under `runtime/raw/`; gzip support is required by the current Clarity summary. Browsers read cached APIs every 30 seconds and never poll election authorities. Refresh view only reloads that cache.
- `data/sources.json` configures sources and verified contest maps. Clarity discovery follows explicitly published county links and validates identity; it never enumerates election IDs. Source changes require server restart. Unverified county sources remain disabled. A disabled source is never polled.
- `data/current-geography.json` contains 79 current provider Madison precincts. Its map ID and actual source-file SHA-256 must match the accepted Clarity feed before geographic vote joins/comparisons are enabled.
- `data/seat-geography.json` clips current Madison precincts and external county portions to each of the seven requested seat outlines. County-only map outlines are clipped too. Five tiny county boundary artifacts are flagged/excluded. No live votes are allocated by map area. [Map provenance](research/seat-geography.md).
- `data/seat-benchmarks.json` contains 349 records: 270 seat features plus 79 countywide Madison current-precinct references. All available values are **modeled 2024 presidential benchmarks**. Original 2024 precinct votes are weighted by 2020 Census adult-population block internal points assigned to the current reporting area and original unsimplified seat boundary. The source already allocates central ballots. No land-area or live-vote allocation is used, and unusable population weights remain null. [Benchmark method and audit](research/madison-benchmark-crosswalk.md).
- Every meaningful outside county has a reference; all six CD-5 counties are included. Independent full-county checks reproduce 2024 Jackson, Lawrence, Limestone and Morgan source totals. Source weights conserve candidate votes with residuals outside each seat. All seven seats account for their positive raw-district adult mass. Census-point allocation remains an estimate of geographic vote distribution; it does not establish exact split-precinct returns or voting eligibility.
- The target control shifts local 2024 D/R shares, calibrating the historical two-party-weighted seat share to the chosen target while constraining local targets to 0–100%. County-only returns use county-part benchmarks; available precinct returns use their modeled precinct-part benchmarks. A reported gap compares actual and target shares with the same current reported two-party vote weights. Administrative precinct rows are excluded, and a D/R gap requires both parties in the ballot’s candidate field. This is a provisional historical reference comparison, not a forecast or projected finish.
- Statewide maps use all 67 full counties from the existing national reference, with direct whole-county 2024 presidential counts. The statewide D/R/other totals are 772,412 / 1,462,616 / 30,062. No new spatial allocation is applied; original RDH source qualifications remain attached. Statewide target gaps compare whole-county observations once, including Madison’s county aggregate and administrative ballots. The Madison switch uses its precinct reference and excludes administrative rows from geographic comparisons. County search filters the table; exports retain the complete selected coverage.
- Other Madison ballot races use the countywide current-precinct reference, consistent with their Madison-only reporting scope. The old raw historical maps/baseline remain separate provenance assets; an exact observed historical crosswalk is not claimed.
- Precinct, county-total and 2024 benchmark CSV exports are separate and retain their respective source levels. Benchmarks retain fractional historical counts and explicit estimated/unavailable flags. Stars remain in this browser only.

## Turnout compared with 2024

County and precinct tables, map area details, and the race/county CSV exports include **Turnout vs 2024**: current reported contest votes divided by **total 2024 presidential votes across all choices**, multiplied by 100. Values may exceed 100%; no cap or two-party substitution is applied. This compares vote volume, not registered-voter turnout or ballots cast. It varies with contest participation and the available historical geographic reference.

Percentages appear only for source-completed areas. Madison precincts use exact-name joins to the same verified Clarity revision’s `json/status.json`; only status 4 (CompletelyReported) or 64 (ElectionCompleted) qualifies. An official county contribution requires every participating geographic precinct in that contest to qualify; administrative units do not establish geographic progress, and their published totals may still change. A civicAPI county qualifies only when that county’s estimated `percent_reporting` is exactly 100. Reporting completion is separate from certification and does not prevent later corrections. Positive votes, candidate calls, and Clarity’s summary reporting fraction alone never mark precincts complete. [Verified provider fields](research/completion-fields.md).

Incomplete or unknown completion, administrative precinct rows, and missing/zero historical totals show a dash. Estimated historical references are marked **≈**. Sources omitting write-ins show **≥** because their reported choice counts form a lower bound against the historical reference; the historical denominator may itself be estimated. County comparisons in split districts use the district’s county portion. A status-fetch failure preserves valid vote totals and withholds completion-based precinct comparisons.

The cached API retains nullable `reporting_complete` and `reporting_basis` on precinct units and selected county contributions. Older cached batches remain readable with unknown completion until refreshed. Race/county exports add these fields plus `presidential_votes_2024`, `turnout_pct_of_2024`, `turnout_lower_bound` and `historical_estimate`. The bulk raw-precinct export remains an observed-results export.

## API and verification

`/api/state`, `/api/health`, `/api/export.csv`, `/data/geography.json`, `/data/current-geography.json`, `/data/baseline.json`, `/data/seat-geography.json`, `/data/seat-benchmarks.json`, `/data/statewide-geography.json`, `/data/statewide-benchmarks.json`, `/data/al02-geography.json`, `/data/al02-benchmarks.json`. The CSV exports accepted observed precinct candidate rows only; unreported placeholder votes never become zero-valued exports.

```bash
cargo test --locked
node --check web/app.js
node --test tests/benchmarks.cjs
../Election-Maps/.venv/bin/python tests/geography_benchmarks.py
../Election-Maps/.venv/bin/python tests/statewide_reference.py
../Election-Maps/.venv/bin/python tests/al02_reference.py
python scripts/prepare-statewide-reference.py --verify
# Against a running server on port 8052, with current cached results:
/home/stolas/Workspace/Projects/national-election-night-2026/.venv/bin/python tests/browser.py
/home/stolas/Workspace/Projects/national-election-night-2026/.venv/bin/python tests/statewide_browser.py
/home/stolas/Workspace/Projects/national-election-night-2026/.venv/bin/python tests/al02_browser.py
/home/stolas/Workspace/Projects/national-election-night-2026/.venv/bin/python tests/turnout_browser.py
/home/stolas/Workspace/Projects/national-election-night-2026/.venv/bin/python tests/pages_browser.py
```

The existing national development environment provides Playwright; it is not a production dependency. Browser acceptance uses real current setup/reference data and a separate intercepted synthetic positive-return response for pagination, selection, export and failure behavior. Synthetic returns never enter the production database. Screenshots and QA output go to `runtime/qa/`. The Morgan fixture is an explicitly quarantined official setup/test excerpt; a hypothetical eligible authority heading exists only inside its parser test.

Offline reference regeneration, with the existing Election-Maps GIS development environment:

```bash
../Election-Maps/.venv/bin/python scripts/prepare-seat-geography.py --verify
../Election-Maps/.venv/bin/python scripts/prepare-seat-benchmarks.py
python scripts/prepare-statewide-reference.py
../Election-Maps/.venv/bin/python scripts/prepare-al02-reference.py
```

Source projects are read-only. Regenerate benchmarks after changing input geography; every referenced input hash is retained. A page reload reads newly prepared reference assets. Production still requires no GIS or Python installation.
