# Madison reporting geography and 2024 presidential benchmarks

This is an offline audit and historical estimate, prepared November 3, 2026. No live results are allocated. Source files in the adjacent national and Election-Maps projects were read only.

## What the geometry comparison establishes

The current election's Clarity map has 79 reporting-name polygons; the original unsimplified RDH 2024 Madison layer also has 79 precincts. Only **7** names match after stripping punctuation and case. Every current polygon has one dominant historical polygon with intersection-over-union above **95%**; 78 exceed 98%, 76 exceed 99%, and 61 exceed 99.5%. None is mathematically topologically equal. None has a second historical polygon covering more than 1% of its area. These observations indicate near-continuity of the cartography, not proof of legally unchanged precincts. Provider polyline precision and repairs alone can prevent exact geometry equality; a changed name is not proof of a changed electorate.

The current map has approximately 0.406 square kilometres of overlapping polygon area, although **no positive-adult-population Census point** falls in an ambiguous overlap in this audit. Historical and current unions cover 99.976% and 99.987% of one another respectively. Full per-polygon intersection diagnostics and input hashes are in `data/benchmark-crosswalk-audit.json`. These area fractions are diagnostics only and never become vote weights.

All **5,874** Madison Census blocks with positive 2020 voting-age population, representing **304,143 adults**, fall strictly within exactly one original historical polygon and exactly one current provider polygon. Every current polygon receives positive mass. The sole source split is historical `Madison-:-FAITH LUTHERAN CH`: 4,481 of its 4,500 adults fall in current Faith Lutheran Church; 19 fall in current Asbury Church. Asbury also contains all 5,388 adults from historical St. Johns Catholic Church. The other 78 historical parents each route all accepted adult mass to one current polygon. This is a block-point crosswalk, not an exact vote crosswalk.

## Prepared artifact and reproducible command

Run:

```bash
../Election-Maps/.venv/bin/python scripts/prepare-seat-benchmarks.py
```

The preparation script produces `data/seat-benchmarks.json`, with 349 feature-keyed records: all 270 clipped display features for seven selected seats, plus all 79 countywide Madison current reporting units. Records use exact `seat:kind:parent_id` identities from `data/seat-geography.json`; countywide Madison keys use `madison:precinct:PROVIDER NAME`. Record fields include availability, candidate counts, two-party and all-candidate totals, Democratic margin in percentage points, source adult denominators, weights, fractional candidate allocations, and caveats. Unavailable records contain null counts and never invent zero-voter results.

For each positive-population point, assign its 2024 original parent precinct and current selected district uniquely. In Madison also assign its current provider reporting name uniquely. Outside Madison, group by county FIPS and selected district. Allocate each original candidate count by the target's adult mass divided by the mass in the **entire original precinct**, including mass outside the selected seat. This uses the same weight for Harris, Trump and other candidates; the method preserves parent preference shares and does not infer demographic party preferences. Fractional votes remain in calculations. No land-area, nearest-polygon, zero-population, or live-vote fallback is used.

Allocation uses original **unsimplified** RDH, Census CD120, Census lower-house and remedial Senate inputs. Provider current precinct polygons retain their published coordinates. Display seat outlines have 30/50-metre simplification; raw and display district membership disagree for only two adults in HD-20 and zero adults in every other selected seat. The calculation follows the raw district boundaries. Three geometry repairs are recorded: two historical polygons after projection and one remedial Senate polygon before projection.

| Scope | Display features | With usable adult mass | Accepted adult mass |
|---|---:|---:|---:|
| CD-5 | 84 | 84 | 560,406 |
| HD-10 | 22 | 11 | 39,003 |
| HD-20 | 25 | 13 | 39,976 |
| HD-21 | 24 | 13 | 40,137 |
| HD-25 | 21 | 14 | 39,137 |
| SD-2 | 37 | 27 | 117,745 |
| SD-7 | 57 | 38 | 117,931 |
| Madison countywide | 79 | 79 | 304,143 |

A rejected initial build guessed a county filter and incorrectly included DeKalb (`049`) while omitting Jackson (`071`). Its prior CD-5 coverage claim was invalid because the denominator shared that omission. The corrected generator scans **all statewide positive-population Census points before filtering counties**, then asserts discovered county sets against meaningful seat-map county features. CD-5 must include `01071, 01077, 01079, 01083, 01089, 01103`, and all five outside-Madison county benchmark records must be usable. Whole Jackson, Lawrence, Limestone and Morgan candidate totals must also reconcile to the independent national import; Lauderdale is a partial county.

An additional independent statewide scan of the cached TIGER-internal-point/PL GeoPackage against the original CD-5 shapefile confirms **560,406 adults**: Jackson 41,768; Lauderdale 13,414; Lawrence 25,878; Limestone 79,718; Madison 304,143; Morgan 95,485. These diagnostics and source hashes are retained in `data/benchmark-crosswalk-audit.json` under `independent_statewide_cd5_scan`. Jackson's historical benchmark is Harris 3,276, Trump 20,073, other 207, total 23,556, matching the national source. Corrected CD-5 estimated totals are Harris 122,380.270458, Trump 224,858.951129, other 6,357.032864, total 353,596.254451.

All seven seats assign 100% of positive adult mass inside the raw reference district to a target. There are zero unmatched/ambiguous positive-mass blocks and zero relevant historical precincts without usable adult mass. A tiny clipped map fragment can have no adult point; that benchmark is withheld, because zero 2020 mass does not prove zero 2024 voters. Map intersection does not establish a precinct's membership in a live race's return table.

The generator asserts unique block IDs/current reporting IDs, nonnegative counts, source weights no greater than one per seat, conservation when source residuals outside each seat are retained, and exact Madison candidate conservation. Madison estimated totals reconcile to the source: Harris **87,824**, Trump **105,430**, other candidates **4,363**, all listed candidates **197,617**. `audit.seat_partitions` records each scope's totals, availability, adult-assignment coverage and partial-source residual weights.

## Sources and limits

The [RDH 2024 Alabama product's local archive](../data/geography.json) was already provenance-audited in `research/geography.md`; the original retained path is `Election-Maps/alabama/data/geospatial/election-precincts/al-2024-gen-prec/al-2024-gen-all-prec/al-2024-gen-all-prec.shp`. Six `G24PRE*` fields provide Harris, Trump, Kennedy, Oliver, Stein and write-ins. RDH already allocates centrally reported absentee/other ballots using candidate-specific precinct election-day shares. Thus even apparently whole, unchanged precinct values remain **estimated source allocations**, and every available benchmark is labeled estimated.

The retained [official Census Alabama PL 94-171 archive](https://www2.census.gov/programs-surveys/decennial/2020/data/01-Redistricting_File--PL_94-171/Alabama/al2020.pl.zip) contains block internal coordinates and `P0030001` adult counts. The national project's strict `scripts/census_block_population.py` parser validates record widths, state identity, unique block/logical record IDs, one-to-one segment linkage and sums against published state rows. Fresh parsing confirms 185,976 Alabama blocks, 5,024,279 residents and 3,917,166 adults, with exact state reconciliation. The new benchmark generator uses this raw parser instead of trusting a stale derived GeoPackage. Census point CRS is handled as NAD83/EPSG:4269, consistently with the national source documentation.

Raw reference plans are [Census CD120 2026](https://www2.census.gov/geo/tiger/TIGER2026/CD/tl_2026_01_cd120.zip), [Census 2025 lower-house districts](https://www2.census.gov/geo/tiger/TIGER2025/SLDL/tl_2025_01_sldl.zip), and the retained [2025 remedial Alabama Senate plan](https://thearp.org/documents/19121/AL_SD_Remedial2025.zip). The current precinct source is the election-specific [Madison Clarity map](https://results.enr.clarityelections.com/AL/Madison/127483/382270/json/78711aa6-2e06-4898-914d-beee88029fcc.json). All raw components, parser, script and display inputs have SHA-256 hashes in the output. Legal applicability caveats from the original geography source remain; no independent plan certification is claimed.

2020 adults differ from the 2024 and 2026 electorates. Census internal points are neither residential locations nor population-weighted centroids; current precinct or district boundaries may divide Census blocks. Population growth, turnout variation and source ballot allocation add unquantified error. These data support a clearly labeled historical benchmark, not exact 2024 split-precinct results, a forecast, or allocation of live 2026 returns. No defensible numeric confidence interval follows from this crosswalk alone.
