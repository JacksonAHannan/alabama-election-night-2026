#!/usr/bin/env python3
"""Copy full Alabama county outlines and whole-county presidential references.

Preparation only; the dashboard runtime needs neither Python nor GIS packages.
Inputs in the national dashboard are read only. No spatial vote allocation occurs.
"""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NATIONAL = ROOT.parent / "national-election-night-2026"
GEOMETRY = NATIONAL / "data/geometry/states/AL.geojson"
GEOMETRY_MANIFEST = NATIONAL / "data/state_geometry.json"
HISTORICAL = NATIONAL / "data/historical/AL-2024.json"
EXPECTED_FIPS = {f"01{i:03}" for i in range(1, 134, 2)}
COUNT_FIELDS = (
    "democratic_votes", "republican_votes", "other_votes",
    "two_party_votes", "total_votes",
)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source(path):
    return {"path": str(path.relative_to(ROOT.parent)), "sha256": digest(path)}


def coordinates(geometry):
    def walk(value):
        if value and isinstance(value[0], (int, float)):
            yield value
        else:
            for child in value:
                yield from walk(child)
    return walk(geometry["coordinates"])


def encode(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), allow_nan=False) + "\n"


def build():
    raw_geometry = json.loads(GEOMETRY.read_text())
    manifest = json.loads(GEOMETRY_MANIFEST.read_text())["AL"]
    historical = json.loads(HISTORICAL.read_text())
    assert digest(GEOMETRY) == manifest["sha256"], "County geometry manifest hash changed"
    geometry_by_id = {f["properties"]["GEOID"]: f for f in raw_geometry["features"]}
    counties = {c["county_id"]: c for c in historical["counties"]}
    assert len(raw_geometry["features"]) == len(historical["counties"]) == 67
    assert set(geometry_by_id) == set(counties) == EXPECTED_FIPS
    sources = [source(GEOMETRY), source(GEOMETRY_MANIFEST), source(HISTORICAL), source(Path(__file__))]
    features, records = [], {}
    caveat = (
        "Whole-county integer totals copied from the national dashboard's 2024 presidential source. "
        "RDH allocates central ballots within counties; that source qualification is preserved. "
        "No new spatial allocation, precinct interpolation, or district split is applied. "
        "Historical context, not a forecast or current race returns."
    )
    for county_id in sorted(EXPECTED_FIPS):
        original = geometry_by_id[county_id]
        county = counties[county_id]
        assert county["county_name"] == original["properties"]["NAME"]
        assert county["results_complete"] and county["major_parties_present"]
        assert all(isinstance(county[k], int) and county[k] >= 0 for k in COUNT_FIELDS)
        assert county["two_party_votes"] == county["democratic_votes"] + county["republican_votes"]
        assert county["total_votes"] == county["two_party_votes"] + county["other_votes"]
        feature_id = f"alabama:county:{county_id}"
        props = {
            "id": feature_id, "kind": "county", "name": county["county_name"],
            "county_name": county["county_name"], "county_id": county_id,
            "FIPS": county_id, "parent_id": county_id, "seat_id": "alabama",
        }
        features.append({"type": "Feature", "id": feature_id, "properties": props, "geometry": original["geometry"]})
        records[feature_id] = {
            **props, "year": 2024, "contest": "president", "available": True,
            "method": "source_whole_county_integer_aggregate", "is_estimate": True,
            **{k: county[k] for k in COUNT_FIELDS},
            "margin_dem_pp": 100 * (county["democratic_votes"] - county["republican_votes"]) / county["two_party_votes"],
            "candidate_votes": county["candidate_votes"],
            "results_basis": county["results_basis"],
            "source_results_basis": historical["results_basis"],
            "source_contributions": [{
                "source_id": county_id, "source_county_name": county["county_name"], "weight": 1,
                **{f"source_{k}": county[k] for k in COUNT_FIELDS},
            }], "caveat": caveat,
        }
    points = [p for f in features for p in coordinates(f["geometry"])]
    bounds = [min(p[0] for p in points), min(p[1] for p in points), max(p[0] for p in points), max(p[1] for p in points)]
    totals = {k: sum(c[k] for c in counties.values()) for k in COUNT_FIELDS}
    official = historical["official_state_reconciliation"]
    assert totals["democratic_votes"] == official["official_democratic_votes"]
    assert totals["republican_votes"] == official["official_republican_votes"]
    assert totals["total_votes"] == official["official_total_votes"]
    geometry = {
        "type": "FeatureCollection", "bbox": bounds,
        "metadata": {"schema_version": 1, "scope": "alabama", "county_count": 67,
                     "bounds": bounds, "sources": sources[:2], "source_geometry": manifest,
                     "operation": "Copy all 67 whole county polygons unchanged; stable FIPS IDs. No clipping or simplification."},
        "features": features,
    }
    benchmarks = {
        "schema_version": 1, "year": 2024, "contest": "president",
        "label": "2024 presidential county reference", "margin_denominator": "two_party_votes",
        "method": "Copy the existing national dashboard's 67 whole-county integer aggregates without spatial vote allocation.",
        "limitations": [caveat, *historical["limitations"]], "sources": sources,
        "source_provenance": historical["provenance"],
        "audit": {"county_count": 67, "totals": totals,
                  "official_state_reconciliation": official, "source_results_basis": historical["results_basis"]},
        "units": records,
    }
    return {"statewide-geography.json": geometry, "statewide-benchmarks.json": benchmarks}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify", action="store_true", help="Require checked-in outputs to reproduce exactly")
    args = parser.parse_args()
    for filename, value in build().items():
        path = ROOT / "data" / filename
        output = encode(value)
        if args.verify:
            assert path.read_text() == output, f"Stale output: {path}"
        else:
            path.write_text(output)
    print("PASS: 67 full counties and whole-county 2024 presidential references; statewide source/FEC totals reconcile")


if __name__ == "__main__":
    main()
