# Seat-clipped display geography

`data/seat-geography.json` provides reference map pieces for CD-5, House districts 10, 20, 21 and 25, and Senate districts 2 and 7. Madison uses the current official election map's precinct polygons; areas outside Madison use county portions. This artifact contains **no vote counts, estimated vote allocations or historical crosswalks**.

## Reproduce and verify

Preparation uses the existing Shapely/PyProj environment, without a dashboard runtime GIS dependency:

```bash
/home/stolas/Workspace/Projects/Election-Maps/.venv/bin/python scripts/prepare-seat-geography.py
/home/stolas/Workspace/Projects/Election-Maps/.venv/bin/python scripts/prepare-seat-geography.py --verify
```

The script's default root comes from its own location. It reads only checked-in `data/current-geography.json` and `data/geography.json`, embeds their SHA-256 hashes, and produces deterministic output. It does not edit source projects. `--verify` reconstructs the artifact, validates geometry and identities, and confirms byte-for-byte equality with the stored file.

## Source boundaries

The seven seat outlines are the prepared display boundaries already documented in [geography.md](geography.md): Census 2026 CD120 for CD-5, Census 2025 House districts, and Alabama's 2025 remedial Senate plan. These carry the existing 50-metre congressional and 30-metre legislative display simplification. They are retained verbatim in `metadata.seats[seat_id].geometry`, with source IDs and vintages. The artifact does not claim independent certification of current election applicability.

Madison's 79 source parents are the exact provider-name polygons documented in [madison-live-map.md](madison-live-map.md), from the current election's [published precinct map](https://results.enr.clarityelections.com/AL/Madison/127483/382270/json/78711aa6-2e06-4898-914d-beee88029fcc.json). Original case-sensitive names identify parents, and remain available in `data/current-geography.json`. County parents use Census 2025 cartographic county reference boundaries, 1:500,000, preserved in `data/geography.json` with FIPS IDs.

This artifact is for display. Statistical assignment to seats must use unsimplified source seat boundaries and an explicitly documented allocation method; the independent benchmark preparation owns that calculation. A positive geometric intersection alone does not establish that the source reports a particular contest in a precinct.

## Feature identity and fields

The collection has 270 polygon features. Every feature is a nonempty, valid Polygon or MultiPolygon. Each feature's `id` is `seat_id:kind:parent_id`; `seat_id` and `scope` are one of `cd5`, `hd10`, `hd20`, `hd21`, `hd25`, `sd2`, `sd7`.

| Property | Meaning |
|---|---|
| `kind` | `precinct` inside Madison, `county` outside Madison |
| `parent_id` / `source_id` | Exact current provider name for precincts; five-character county FIPS for counties |
| `name` / `unit_name` / `source_unit_name` | Original current provider precinct name or original county name |
| `full_parent_geometry_path` / `full_parent_feature_id` | Location and stable ID of the whole original reporting-unit polygon |
| `parent_area_m2` / `clipped_area_m2` | Whole and displayed polygon areas measured in EPSG:5070 |
| `area_fraction_of_parent` | Displayed area divided by full parent area; **never a vote or population weight** |
| `area_fraction_of_seat` | Displayed area divided by full seat area; **never a vote or population weight** |
| `is_clipped` | Whether the displayed polygon differs from its whole source parent |
| `boundary_resolution_sliver` | Tiny county intersection that does not establish electoral membership |
| `votes_allocated` | Always `false` |
| `return_geography` | Always `whole_source_reporting_unit`; clipping the map does not split return values |

Source precinct names may contain punctuation, spaces or colons; consume `parent_id` directly instead of parsing the composite ID. The source paths and parent feature IDs preserve the full parent geometry separately, avoiding duplication in every seat piece.

## County-only map mode

`metadata.seats[seat_id].county_features` is a separate array of county GeoJSON features for county-only display. It contains Madison plus the seat's meaningful outside counties, with each polygon equal to the complete prepared **seat ∩ county** outline. The county-only view does not subtract Madison precinct coverage. The tiny county boundary-resolution artifacts described below are excluded.

County-only feature IDs use `seat_id:county:FIPS`; `parent_id`, `source_id` and `county_id` retain the five-character county FIPS, and `kind` is `county`. They include the same parent references, area fractions and no-vote-allocation labels as the mixed view. There are six county outlines for CD-5, one each for HD-10, HD-20, HD-21 and SD-7, and two each for HD-25 and SD-2. These metadata outlines do not alter the original 270 top-level mixed-view features or their IDs. County-only source boundaries still do not establish precise live voting geography.

## Overlay method and safeguards

Every parent is intersected with the seat in EPSG:4326, preserving the exact prepared seat outline and source coordinates. There is no further simplification, rounding or nearest-feature assignment. Boundary-only lines and points have no area and are excluded. Invalid overlay geometry is repaired with `make_valid`, retaining polygonal components. Areas are measured after transformation to EPSG:5070.

County fragments additionally subtract Madison's county reference outline and the current provider's combined precinct footprint. This prevents a county piece outside Madison from drawing over Madison's current provider precincts where sources disagree. Source differences therefore remain as explicit coverage gaps instead of receiving fabricated geography.

Each seat's metadata records its outline, bounds, area, counts, union-covered area, uncovered area and source overlap area. It also records the part of Madison's reference outline left uncovered by the current provider map. Overlaps within the provider's own polygons remain source limitations; county/precinct display overlap is removed by the mask above.

## Counts and source-resolution artifacts

| Seat | Madison precinct pieces | Outside county pieces | Meaningful outside county portions | Tiny county artifacts |
|---|---:|---:|---|---:|
| CD-5 | 79 | 5 | Jackson, Lauderdale, Lawrence, Limestone, Morgan | 0 |
| HD-10 | 20 | 2 | None established | 2 |
| HD-20 | 24 | 1 | None established | 1 |
| HD-21 | 24 | 0 | None | 0 |
| HD-25 | 20 | 1 | Limestone | 0 |
| SD-2 | 36 | 1 | Limestone | 0 |
| SD-7 | 55 | 2 | None established | 2 |

Coarse county outlines and more detailed seat boundaries disagree slightly along county edges. The county artifacts in HD-10 (Limestone/Morgan), HD-20 (Morgan), SD-7 (Jackson/Limestone) are each below 0.25 km² **and** 0.1% of seat area. They are retained for geometric honesty but flagged `boundary_resolution_sliver: true`, and excluded from `metadata.seats[seat_id].outside_madison_counties_excluding_boundary_slivers`. Consumers should withhold them from electoral county labels and vote aggregation unless independent raw-boundary/benchmark evidence establishes actual seat membership. This avoids converting geometric noise into invented voting coverage.

CD-5's Lauderdale outline is the clipped district portion, not the full county. HD-25 and SD-2 also show only their Limestone portion. Whole-county returns do not become seat-specific returns merely because their county polygon has been clipped.

Coverage gaps from differing source precision are approximately 3.172 km² in CD-5, 0.126 in HD-10, 0.097 in HD-20, 0.107 in HD-21, 0.023 in HD-25, 0.079 in SD-2 and 0.247 in SD-7. These small display gaps are recorded, not filled. Administrative ABSENTEE, Provisional and PROVISIONAL ABSENTEE units have no geographic polygon.

## Verification evidence

A fresh `--verify` run checks a deterministic rebuild, unique IDs across all 270 features, all seven expected seats, valid nonempty area geometry, positive fractions no greater than one, no Madison county feature mixed with Madison precincts, `votes_allocated: false`, and containment within the exact prepared seat outline with a 0.0001 m² numerical tolerance. The verifier also checks county-only outline identities, expected county sets including Madison, absence of flagged artifacts, and seat containment. Source hashes and full-parent references are embedded for downstream auditing. No performance or statistical-accuracy claim is made by this map preparation.
