# Alabama statewide county reference assets

The statewide Senate, Governor, and other statewide views use all 67 full Alabama
counties. This is a separate reference from the seven seat-specific clipped maps.
No existing seat or Madison precinct assets were changed.

## Geometry

`data/statewide-geography.json` copies all polygons unchanged from the read-only
national dashboard input `../national-election-night-2026/data/geometry/states/AL.geojson`.
Stable IDs are `alabama:county:<five-digit FIPS>`, with matching `county_id`,
`FIPS`, and `parent_id` properties. Every county has `seat_id: "alabama"` and
`kind: "county"`. The collection and metadata include the full-state bounds.

The national manifest `../national-election-night-2026/data/state_geometry.json`
documents the [Census 2025 cartographic county boundaries at 1:500,000](https://www2.census.gov/geo/tiger/GENZ2025/shp/cb_2025_us_county_500k.zip).
It records original archive SHA-256
`aa976c00b181939755d0da757f4c7c2dc0103c3b3b4530fb2a91c2bb62fc777c`,
reprojection from EPSG:4269 to EPSG:4326, no simplification or rounding, and zero
invalid, empty, or duplicate county geometries. This preparation does not further
transform them. The AL geometry input SHA-256 is
`b364f9fbfef20a1b178eab9a7e13369f1fb42042e3be6608fe03d80ade208f40`.

## Presidential benchmarks

`data/statewide-benchmarks.json` copies the 67 existing integer county aggregates
in `../national-election-night-2026/data/historical/AL-2024.json`, SHA-256
`8320e51499a801c970a15042dabf656c146b51302d9649153bbdca6c9fb24421`.
It does not allocate precinct votes into counties, interpolate votes over a map,
or use demographic weights. Original candidate-level counts, whole-county
aggregate counts, source provenance, and source qualification are retained.

The national source identifies Redistricting Data Hub's local Alabama 2024
general-election all-precinct layer. RDH allocates centrally reported votes
within counties, and the inherited results basis is `estimated_source_allocation`.
The county references therefore conservatively retain `is_estimate: true`, with
method `source_whole_county_integer_aggregate` and an explicit whole-county
qualification. No `adult_mass` field is manufactured. Source provenance includes
a legacy RDH URL whose path says 2022; that inherited URL is preserved as data
and does not override the 2024 archive paths, election metadata, candidate
fields, and independent statewide comparison.

Statewide sums are Harris **772,412**, Trump **1,462,616**, other listed candidates
**30,062**, total **2,265,090**, with **2,235,028** two-party votes. The source's
independently preserved [FEC 2024 presidential result workbook](https://www.fec.gov/documents/5645/2024presgeresults.xlsx)
reconciliation matches both major-party and all-candidate statewide counts.
This confirms statewide totals, not the correctness of each source precinct
allocation or legal boundaries. These references are historical context for all
2026 statewide races, not projections of their final results.

## Reproduction and acceptance

Run from the Alabama dashboard repository:

```sh
python scripts/prepare-statewide-reference.py
python scripts/prepare-statewide-reference.py --verify
../Election-Maps/.venv/bin/python tests/statewide_reference.py
```

The generator uses Python's standard library and only reads sibling-project
inputs. Both outputs store input SHA-256 hashes; the benchmarks also preserve
the generator hash. The independent test verifies all 67 expected Alabama FIPS
codes, valid unchanged whole-county geometry, stable IDs, original county and
candidate counts, statewide conservation, source provenance, and the absence of
invented adult-population weights. All commands passed on November 3, 2026.
