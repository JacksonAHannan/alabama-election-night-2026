#!/usr/bin/env python3
"""Independent artifact checks; run with ../Election-Maps/.venv/bin/python."""
from collections import Counter
import hashlib
import json
from pathlib import Path

import pyogrio
from pyproj import Transformer
from shapely.geometry import shape
from shapely.ops import transform, unary_union

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = {'01005', '01011', '01013', '01031', '01039', '01041', '01045', '01051',
            '01061', '01067', '01069', '01085', '01087', '01101', '01109', '01113'}
SPLIT = {'01039', '01051'}
FIELDS = ('democratic_votes', 'republican_votes', 'other_votes')
geo = json.loads((ROOT / 'data/al02-geography.json').read_text())
bench = json.loads((ROOT / 'data/al02-benchmarks.json').read_text())
seat = shape(geo['metadata']['seats']['cd2']['geometry'])
measure = Transformer.from_crs(4326, 5070, always_xy=True).transform
assert seat.is_valid and not seat.is_empty
assert {f['properties']['county_id'] for f in geo['features']} == EXPECTED
assert len(geo['features']) == 16
assert set(bench['units']) == {f'cd2:county:{cid}' for cid in EXPECTED}
assert geo['metadata']['seats']['cd2']['county_features'] == geo['features']
areas = []
for feature in geo['features']:
    p = feature['properties']; g = shape(feature['geometry'])
    assert feature['id'] == p['id'] == f"cd2:county:{p['county_id']}"
    assert p['seat_id'] == 'cd2' and p['kind'] == 'county'
    assert p['parent_id'] == p['source_id'] == p['county_id']
    assert p['electoral_county_split'] == (p['county_id'] in SPLIT)
    assert not p['votes_allocated'] and not p['boundary_resolution_sliver']
    assert g.is_valid and not g.is_empty and g.geom_type in ('Polygon', 'MultiPolygon')
    assert transform(measure, g.difference(seat)).area < .0001
    assert 0 < p['area_fraction_of_parent'] <= 1
    areas.append(g)
# Neighboring county pieces cannot contribute positive-area overlap.
overlap = sum(transform(measure, g).area for g in areas) - transform(measure, unary_union(areas)).area
assert abs(overlap) < .1, overlap

source_path = ROOT.parent / 'Election-Maps/alabama/data/geospatial/election-precincts/al-2024-gen-prec/al-2024-gen-all-prec/al-2024-gen-all-prec.shp'
meta, _, _, fields = pyogrio.raw.read(source_path, read_geometry=False)
original = {str(row['UNIQUE_ID']): row for row in (dict(zip(meta['fields'], vals)) for vals in zip(*fields))}
assert len(original) == 1947
national = {c['county_id']: c for c in json.loads((ROOT.parent / 'national-election-night-2026/data/historical/AL-2024.json').read_text())['counties']}
partitions = Counter()
for key, record in bench['units'].items():
    cid = record['county_id']
    assert key == f'cd2:county:{cid}'
    assert record['available'] and record['is_estimate'] and record['adult_mass'] > 0
    assert record['method'] == 'estimated_census_2020_adult_internal_points'
    source_sums = Counter(); mass = 0
    for contribution in record['source_contributions']:
        source = original[contribution['source_id']]
        assert '01' + str(source['COUNTYFP']).zfill(3) == cid
        d, r = int(source['G24PREDHAR']), int(source['G24PRERTRU'])
        other = sum(int(v) for k, v in source.items() if k.startswith('G24PRE')) - d - r
        weight = contribution['weight']
        assert weight == contribution['adult_mass'] / contribution['source_adult_denominator']
        assert 0 < weight <= 1
        mass += contribution['adult_mass']; partitions[contribution['source_id']] += weight
        for field, votes in zip(FIELDS, (d, r, other)):
            assert contribution['source_' + field] == votes
            source_sums[field] += votes * weight
    assert mass == record['adult_mass']
    for field in FIELDS:
        assert abs(source_sums[field] - record[field]) < 1e-8
    assert abs(record['two_party_votes'] - record['democratic_votes'] - record['republican_votes']) < 1e-8
    assert abs(record['total_votes'] - sum(record[f] for f in FIELDS)) < 1e-8
    if cid not in SPLIT:
        for field in (*FIELDS, 'total_votes'):
            assert abs(record[field] - national[cid][field]) < 1e-8, (cid, field)
    else:
        assert all(0 < record[field] < national[cid][field] for field in (*FIELDS, 'total_votes'))
assert all(weight <= 1 + 1e-12 for weight in partitions.values())
audit = bench['audit']
assert set(audit['county_universe']['discovered_counties_from_statewide_block_scan']) == EXPECTED
assert set(audit['county_universe']['expected_counties_from_current_race_roster']) == EXPECTED
assert audit['unresolved_blocks'] == []
assert not any(s['votes'] for s in audit['zero_adult_source_precincts'])
assert sum(audit['county_adult_mass'].values()) == sum(u['adult_mass'] for u in bench['units'].values()) == 559067
for cid in EXPECTED:
    if cid not in SPLIT:
        assert audit['county_adult_mass'][cid] == audit['full_county_adult_mass'][cid]
    else:
        assert 0 < audit['county_adult_mass'][cid] < audit['full_county_adult_mass'][cid]
summary = audit['seat_partitions']['cd2']
assert summary['raw_seat_adult_mass'] == summary['accepted_seat_adult_mass'] == 559067
assert summary['adult_assignment_coverage'] == 1
for field in (*FIELDS, 'total_votes', 'two_party_votes', 'adult_mass'):
    assert abs(summary[field] - sum(u[field] for u in bench['units'].values())) < 1e-8
for source in (*geo['metadata']['sources'], *bench['sources']):
    assert hashlib.sha256(Path(source['path']).read_bytes()).hexdigest() == source['sha256'], source['path']
print('PASS: 16 AL-02 clipped counties, 14 whole-county reconciliations, two modeled split counties, 559067 adults fully accounted, source weights/counts/hashes verified.')
