"""Independent county completeness, geometry, provenance, and source-vote checks."""
import hashlib
import json
from pathlib import Path

from shapely.geometry import shape

ROOT = Path(__file__).resolve().parents[1]
NATIONAL = ROOT.parent / "national-election-night-2026"
geo = json.loads((ROOT / "data/statewide-geography.json").read_text())
bench = json.loads((ROOT / "data/statewide-benchmarks.json").read_text())
source_geo = json.loads((NATIONAL / "data/geometry/states/AL.geojson").read_text())
historical = json.loads((NATIONAL / "data/historical/AL-2024.json").read_text())
expected = {f"01{i:03}" for i in range(1, 134, 2)}
features = {f["properties"]["county_id"]: f for f in geo["features"]}
source_features = {f["properties"]["GEOID"]: f for f in source_geo["features"]}
counties = {c["county_id"]: c for c in historical["counties"]}
assert len(geo["features"]) == len(bench["units"]) == 67
assert set(features) == set(counties) == expected
fields = ["democratic_votes", "republican_votes", "other_votes", "two_party_votes", "total_votes"]
for county_id in expected:
    feature = features[county_id]
    unit_id = f"alabama:county:{county_id}"
    props, geometry = feature["properties"], shape(feature["geometry"])
    assert feature["id"] == props["id"] == unit_id
    assert props["FIPS"] == props["parent_id"] == county_id
    assert props["kind"] == "county" and props["seat_id"] == "alabama"
    assert geometry.is_valid and not geometry.is_empty and geometry.geom_type in {"Polygon", "MultiPolygon"}
    assert feature["geometry"] == source_features[county_id]["geometry"], "Polygons must remain whole and unchanged"
    record = bench["units"][unit_id]
    assert record["available"] and record["is_estimate"]
    assert "adult_mass" not in record, "No invented demographic weight"
    assert record["method"] == "source_whole_county_integer_aggregate"
    assert record["candidate_votes"] == counties[county_id]["candidate_votes"]
    for field in fields:
        assert isinstance(record[field], int) and record[field] == counties[county_id][field], (county_id, field)
    assert record["two_party_votes"] == record["democratic_votes"] + record["republican_votes"]
    assert record["total_votes"] == record["two_party_votes"] + record["other_votes"]
for field in fields:
    total = sum(r[field] for r in bench["units"].values())
    assert total == sum(c[field] for c in counties.values()) == bench["audit"]["totals"][field]
assert bench["audit"]["totals"]["democratic_votes"] == 772412
assert bench["audit"]["totals"]["republican_votes"] == 1462616
assert bench["audit"]["totals"]["total_votes"] == 2265090
for source in bench["sources"]:
    path = ROOT.parent / source["path"]
    assert hashlib.sha256(path.read_bytes()).hexdigest() == source["sha256"], path
assert bench["source_provenance"] == historical["provenance"]
print("PASS: 67 unchanged full county polygons, stable FIPS, exact source county candidate counts, statewide conservation and provenance")
