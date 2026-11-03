# 2024 presidential benchmarks on current tracked seats

Source audit November 3, 2026. This note covers reusable local inputs and non-Madison county portions. It does not modify live returns or claim that historical precincts match current reporting units.

## Main finding

Reusable adult-Census-weighted historical allocations exist for CD-5 and the state legislative seats. The two Limestone benchmark portions needed by the county fallback have been independently checked against the current plan’s positive-population block membership. They can be reproduced from retained source counts and explicit weights, with estimates labeled. Every full county inside CD-5 can use its retained 2024 presidential county sum. Lauderdale must use only the current CD-5 fragment of that county.

## Source counts and their meaning

The [national historical import](../../national-election-night-2026/data/historical/AL-2024.json) has 1,947 source precinct units and 67 county aggregates. Unit keys include `source_unit_id` (the original RDH `UNIQUE_ID`), `unit_id` (qualified `AL:<FIPS>:<source ID>`), `county_id`, `candidate_votes`, `democratic_votes`, `republican_votes`, `other_votes`, `total_votes`, and source limitations. Presidential fields are Harris `G24PREDHAR` and Trump `G24PRERTRU`; all six source-listed candidates contribute to `total_votes`.

Its retained raw source is `Election-Maps/alabama/data/geospatial/election-precincts/al-2024-gen-prec.zip`, SHA-256 `eb635ed3f4359be638fe2b8130ce9aa4d8e513849e2502813cd73ff2e69fb90e`. The README identifies Alabama 2024 general results and boundaries, RDH retrieval April 21, 2025, and the [Alabama SOS election-data page](https://www.sos.alabama.gov/alabama-votes/voter/election-data). RDH allocated centrally reported county/Judicial Division ballots by each candidate’s election-day precinct share. Thus even a whole historical precinct is exact only relative to the retained RDH table; it is not an observed unsplit precinct tally for all voting methods. The README’s 2022 RDH link describes a boundary input, not the 2024 download URL. See the [existing geography audit](geography.md) and the source’s `limitations` and `provenance`.

Current project display geometry is simplified and must not be used to generate allocation weights. The [national Census-split method](../../national-election-night-2026/research/census-split-precinct-method.md) uses unsimplified source precincts, unsimplified current districts, and 2020 Census adult population at block internal points. It preserves each parent precinct’s candidate shares in every fragment and makes no demographic party-preference inference. Its original [official Alabama PL94-171 archive](https://www2.census.gov/programs-surveys/decennial/2020/data/01-Redistricting_File--PL_94-171/Alabama/al2020.pl.zip) is retained locally at `Election-Maps/alabama/data/demographics/census-2020-blocks/al2020.pl.zip`. The national parser audits 185,976 Alabama blocks, 5,024,279 residents and 3,917,166 adults.

## CD-5 county portions

Two complementary national files share stable historical `unit_id` values:

- [Source crosswalk](../../national-election-night-2026/data/crosswalks/AL-2024-cd120.json): `units[]` with `status`, `current_district_id`, `candidate_district_ids`, `county_id`, and `baseline`. Select `current_district_id == 'AL-H-05'` for wholly assigned source precincts.
- [Census split model](../../national-election-night-2026/data/census-split-estimates/AL.json): `races['AL-H-05'].units[]` with `county_id`, `unit_id`, `weight`, `fragment_vap`, `source_vap`, and fractional `baseline` candidate aggregates. It supplements withheld source units; do not duplicate a source unit already assigned by the first file. There are 48 modeled CD-5 fragments. The model also preserves zero-mass exclusions separately.

The current reference boundary is the retained [Census 2026 CD120 Alabama archive](https://www2.census.gov/geo/tiger/TIGER2026/CD/tl_2026_01_cd120.zip), not a congressional seat name assigned from a 2024 congressional contest. Historical 2024 House candidate totals are not presidential benchmarks.

For the four fully included non-Madison counties, the combined whole-plus-modeled CD-5 source units reconcile exactly to the complete historical county presidential D/R totals:

| County | FIPS | Harris | Trump | All listed candidates |
| --- | --- | ---: | ---: | ---: |
| Jackson | 01071 | 3,276 | 20,073 | 23,556 |
| Lawrence | 01079 | 2,983 | 13,024 | 16,122 |
| Limestone | 01083 | 14,581 | 37,887 | 53,332 |
| Morgan | 01103 | 12,392 | 40,449 | 53,543 |
| Lauderdale CD-5 portion | 01077 | ≈1,324.270458 | ≈7,995.951129 | ≈9,426.254451 |

Lauderdale’s included whole precincts are `ELGIN SENIOR CTR_` and `ROGERSVILLE 1ST BAPT_`. Its four modeled fragments are `ANDERSON GYMNASIUM` (weight 1), `ATLAS CH OF CHRIST` (31/5573), `CTR_ STAR 1ST BAPT_` (3440/3919), and `LEXINGTON SENIOR CTR_` (2107/2111). The retained CD-5 model excludes `KILLEN CH OF CHRIST` because the intersecting fragment has no uniquely located adult block population. That is a zero-weight boundary caveat, not evidence that the fragment had zero 2024 voters. Tiny district touches in nearby counties similarly have zero Census mass and must not create county vote contributions.

The reproducible non-Madison CD-5 benchmark is the four complete county rows plus the six Lauderdale rows above: Harris **34,556.27045807275**, Trump **119,428.95112857418**, other **1,994.032864311546**, all listed candidates **155,979.25445095845**. Democratic two-party margin is approximately **−55.117420 percentage points**. Preserve fractional values through calculation and export; round only display. Do not include Madison in this external-county aggregate, because Madison’s current-seat precinct fragments are a separate contribution.

## Limestone HD-25 and SD-2

The newer governor-model crosswalks are usable historical allocation inputs, independent of the governor scenario’s partisan assumptions:

- [House adult-population crosswalk](../../Election-Maps/alabama/data/governor-poll-scenario-2026/precinct-to-house-district.csv), 2,197 rows.
- [Senate adult-population crosswalk](../../Election-Maps/alabama/data/governor-poll-scenario-2026/precinct-to-senate-district.csv), 2,040 rows.

Both have `UNIQUE_ID,DISTRICT,VAP20,allocation_weight,allocation_method`. Every saved row has method `2020 Census block voting-age population`; the saved [demographic audit](../../Election-Maps/alabama/data/governor-poll-scenario-2026/demographics-audit.json) records zero precinct nearest joins, zero district nearest joins, zero area-fallback precincts, and unit weight sums exactly one. The builder `Election-Maps/scripts/acquire-alabama-governor-scenario-demographics.py` contains a nearest and area fallback path, so inspect these recorded outcomes rather than assuming the algorithm can never fall back.

Join each crosswalk `UNIQUE_ID` one-to-one to historical `source_unit_id`, select `County=Limestone` by the historical record’s county ID, and multiply **each presidential candidate’s source count** by `allocation_weight`. Recompute D/R shares and margins from additive counts. Do not average margins or scale the county total by land area.

| Seat | Historical Limestone source precinct | Adult mass | Weight |
| --- | --- | ---: | ---: |
| HD-25 | CAPSHAW BAPTIST CH | 2,493 | 0.4254992319508449 |
| SD-2 | BETHEL CH OF CHRIST | 834 | 0.16554188169908693 |
| SD-2 | CAPSHAW BAPTIST CH | 5,859 | 1 |
| SD-2 | COPELAND PRESBYTERIAN CH | 3,866 | 0.7811679127096383 |
| SD-2 | JAMES CLEMENS HIGH SCHOOL | 1,718 | 0.23970978094042136 |

The resulting Limestone HD-25 benchmark is Harris **896.5268817204302**, Trump **1,049.7066052227344**, all listed candidates **1,990.0599078341015** (D two-party margin −7.870573 pp). Limestone SD-2 is Harris **3,230.404177266447**, Trump **4,903.3359053623835**, all listed candidates **8,325.637055520408** (D two-party margin −20.567804 pp). These are modeled 2024 presidential county portions, not 2024 HD-25 or SD-2 race results.

## Current-plan verification and caution

The saved governor-model crosswalk used older 2021 House/Senate plan files. The project now uses Census 2025 SLDL and the [2025 remedial Senate plan](https://thearp.org/documents/19121/AL_SD_Remedial2025.zip). I directly compared each selected seat’s old and current unsimplified polygons against **all positive-adult-population block points** in `Election-Maps/alabama/data/demographics/census-2020-blocks/alabama-2020-block-vap.gpkg`. The read-only check used `pyogrio.read_arrow`, Shapely `within`, and PyProj transformations into EPSG:5070. The Senate remedial shapefile and DBF have different filename case; copying their components to matching temporary basenames was needed for its district attributes to be read. No retained source was edited.

| Seat | Old positive blocks | Current positive blocks | Old adult mass | Current adult mass | Changed positive blocks |
| --- | ---: | ---: | ---: | ---: | ---: |
| HD-10 | 580 | 580 | 39,003 | 39,003 | 0 |
| HD-20 | 793 | 792 | 39,977 | 39,976 | 1 (one adult) |
| HD-21 | 958 | 958 | 40,137 | 40,137 | 0 |
| HD-25 | 603 | 603 | 39,137 | 39,137 | 0 |
| SD-2 | 1,740 | 1,740 | 117,745 | 117,745 | 0 |
| SD-7 | 2,701 | 2,702 | 117,929 | 117,931 | 1 (two adults) |

Thus the **Limestone HD-25 and SD-2 weights are unchanged under the current adult-internal-point model**; HD-10 and HD-21 also have identical memberships. For HD-20 and SD-7, regenerate the historical allocation from current reference polygons rather than silently carrying forward the old weights. Boundary geometries are not exactly identical even when every populated point has unchanged membership. This numerical check does not certify a legal boundary or prove a live precinct crosswalk.

The older `Election-Maps/outputs/alabama-2024-presidential-legislative-district-results/precinct-to-{house,senate}-district-crosswalk.csv` files and their `create-alabama-2024-presidential-legislative-district-maps.py` builder use **land-area weights**. They must not be mislabeled Census-population allocations or silently substituted for the newer files above.

## Reproducible integration contract

Preserve raw historical observations and generated current-seat allocations separately. Give each benchmark its current seat ID, county ID, election `2024 presidential`, source hashes, current-plan hashes, method, raw parent IDs, fractional counts, weight and unresolved/zero-mass caveats. For Madison-only races the geographic benchmark is the county’s historical sum. For the seven tracked seats, add current-seat Madison fragments plus the external county portions above; select each county once and never add county totals on top of their precinct children. A county-return source does not justify drawing precinct live-return allocations.

For each contribution verify unique IDs, candidate identity, nonnegative finite weights/counts, parent conservation including unresolved mass, source hash consistency and geographic completeness. When computing reported-so-far comparison, use the benchmark of exactly the counties/precinct aggregates that supplied the live results; missing counties retain unknown status.

Read-only verification in this subtask recomputed the tabulated estimates and compared old/current plan membership. It did not run application tests or create a live-to-historical precinct match.

Input SHA-256 values at inspection:

| Artifact | SHA-256 |
| --- | --- |
| National AL historical JSON | `8320e51499a801c970a15042dabf656c146b51302d9649153bbdca6c9fb24421` |
| National AL CD120 source crosswalk | `12982eca6458000421d25ee74b568b3085ea7ee9ffb82af7f05a5827f1bfe62e` |
| National AL Census split estimates | `087a3caabcc849fc632111ce9fb3b298d0dd3883c98bbe76819e5935f596b581` |
| Governor-model House VAP crosswalk | `aa7bea33bcf261cb708fccc11cd900040174ea6f2182af6d30118d9e5b6eecf9` |
| Governor-model Senate VAP crosswalk | `4a5fcf0680531035c21d5fd41b47dfbeb6060664bde671a81d31a90bc97f70bb` |
