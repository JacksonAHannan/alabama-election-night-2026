# AL-02 current-plan geography and 2024 presidential reference

The AL-02 view uses the **2026 current congressional district**, rather than the district used for the November 2024 House race. The Alabama Secretary of State publishes the [2023 Congressional Plan map](https://www.sos.alabama.gov/sites/default/files/06-03-2026/2023%20Congressional%20Plan_Map.pdf) in its current 2026 election resources. Current [general-election sample ballots](https://www.sos.alabama.gov/alabama-votes/2026-general-election-sample-ballots) independently identify Shomari Figures and Rhett Marques as the AL-02 candidates in Coffee, Covington, Elmore, Houston and Lowndes. The current [civicAPI AL-02 race](https://civicapi.org/api/v2/race/86901) supplies an independent 16-county roster. These online findings are detailed in [al02-sources.json](al02-sources.json).

The cached Census [2026 CD120 source](https://www2.census.gov/geo/tiger/TIGER2026/CD/tl_2026_01_cd120.zip) has the same 16-county extent. It includes Barbour, Bullock, Butler, Coffee, Crenshaw, Dale, Geneva, Henry, Houston, Lowndes, Macon, Montgomery, Pike and Russell in full, and portions of Covington and Elmore. It excludes the Mobile-area counties present in the 2024 election's different AL-02 district. An all-state Census adult-point scan independently confirms precisely these 16 counties; it runs **before** any county filtering.

## Assets and consumption

- `data/al02-geography.json` is a 16-feature GeoJSON collection. IDs are `cd2:county:FIPS`; properties include `seat_id: cd2`, `kind: county`, the county FIPS as `parent_id`, and `electoral_county_split`. `metadata.seats.cd2` provides the district outline, bounds and `county_features`, compatible with the existing seat reference interface.
- `data/al02-benchmarks.json` uses the existing `seat-benchmarks.json` schema. Its 16 `units` have identical feature IDs and fractional 2024 Democratic, Republican and other presidential counts. Every benchmark is labeled estimated.
- `scripts/prepare-al02-reference.py` is an offline reproducible builder. It reads adjacent Election-Maps and national-dashboard source files without editing them. Both assets preserve input SHA-256 hashes and current-plan source links.

Display polygons intersect the existing 2025 Census county outlines with a 50-metre simplified display outline of the Census 2026 CD120 district. Area fractions describe geometry only and are never vote weights. The statistical allocation instead uses the **unsimplified original district** in EPSG:5070. Different county/district source resolutions leave small explicit gaps and tiny neighboring-county intersections; the independent electoral roster prevents those artifacts from creating false county membership.

## Estimated 2024 context

The calculation reuses the dashboard's established approach: assign each statewide Census 2020 positive-adult block internal point to the unsimplified current district and its original 2024 RDH presidential precinct. For each entire original precinct, divide the adult mass inside the selected district/county portion by that precinct's **entire** adult mass, including adults outside the seat. Apply that fraction separately to Democratic, Republican and other candidate totals. Do not renormalize a split precinct to absorb its outside-seat votes.

The first pass examines every positive-adult block statewide, discovers the seat's county universe, and checks it against the independent current-race roster. The second pass processes every positive-adult block in those complete counties, providing complete source denominators. All **559,067** adults selected in the statewide scan are assigned exactly once to the intended source precinct and one county reference. There are no unresolved or ambiguous positive-adult blocks and no positive-vote source precincts with zero adult denominator.

| Scope | Democratic | Republican | Other | Total | Adult mass |
|---|---:|---:|---:|---:|---:|
| Current AL-02 | 123,968.282272 | 165,892.388801 | 3,326.053585 | 293,186.724658 | 559,067 |
| Covington portion | 152.470081 | 1,260.933264 | 8.563373 | 1,421.966718 | 2,467 |
| Elmore portion | 4,657.812191 | 13,175.455537 | 232.490212 | 18,065.757940 | 33,492 |

The overall estimated Democratic minus Republican two-party margin is **−14.463537 percentage points**. Fourteen whole-county totals reconcile exactly to the national dashboard's original 2024 RDH county aggregates. Covington and Elmore are modeled current-seat portions, not their entire county presidential results. Every source precinct's assigned and complementary outside-seat candidate counts conserve its original counts; no source weight exceeds one.

These are historical context estimates, not observed split-precinct returns, demographic voting estimates, election forecasts or live vote allocations. RDH already allocated central ballots into the original 2024 precincts. Census 2020 adults are not the 2024 or 2026 electorate, and a block internal point is not a population centroid. No area, nearest-neighbor or zero-population fallback is used. The current county feed must supply **AL-02 contest totals for the county portion**; statewide-race whole-county totals are not a substitute in split counties.

## Reproduce and verify

```bash
../Election-Maps/.venv/bin/python scripts/prepare-al02-reference.py
../Election-Maps/.venv/bin/python scripts/prepare-al02-reference.py --verify
../Election-Maps/.venv/bin/python tests/al02_reference.py
```

`--verify` performs a fresh full-state block scan and source allocation, validates all output geometry and weights, then requires byte-for-byte agreement with both saved artifacts. The focused independent test checks all 16 stable identities, valid clipped polygons and containment, absence of positive-area county overlaps, source candidate values against original DBF data, weights and totals, all 14 whole-county reconciliations, both fractional split-county references, full adult-mass coverage and every recorded input hash.
