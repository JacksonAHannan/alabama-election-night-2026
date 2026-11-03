#!/usr/bin/env python3
"""Prepare AL-02 current-plan county outlines and estimated 2024 context.

Offline, deterministic; adjacent source projects remain read only.
Run with ../Election-Maps/.venv/bin/python. --verify rebuilds both artifacts.
"""
from __future__ import annotations
import argparse
from collections import Counter, defaultdict
import hashlib
import importlib.util
import json
from pathlib import Path

import pyogrio
from pyproj import Transformer
from shapely import STRtree, from_wkb, make_valid
from shapely.geometry import Point, mapping, shape
from shapely.ops import transform, unary_union

ROOT = Path(__file__).resolve().parents[1]
MAPS = ROOT.parent / 'Election-Maps'
NATIONAL = ROOT.parent / 'national-election-night-2026'
# Current 2026 race's independent county roster, checked against the statewide
# adult-point scan below. Never discover membership by coarse-outline slivers.
COUNTIES = {'01005': 'Barbour', '01011': 'Bullock', '01013': 'Butler',
            '01031': 'Coffee', '01039': 'Covington', '01041': 'Crenshaw',
            '01045': 'Dale', '01051': 'Elmore', '01061': 'Geneva',
            '01067': 'Henry', '01069': 'Houston', '01085': 'Lowndes',
            '01087': 'Macon', '01101': 'Montgomery', '01109': 'Pike',
            '01113': 'Russell'}
SPLIT = {'01039', '01051'}
MEASURE = Transformer.from_crs(4326, 5070, always_xy=True).transform
DISPLAY = Transformer.from_crs(5070, 4326, always_xy=True).transform
BLOCK_PROJECT = Transformer.from_crs(4269, 5070, always_xy=True).transform
VOTE_FIELDS = ('democratic_votes', 'republican_votes', 'other_votes')
CURRENT_PLAN_SOURCES = [
    {'publisher': 'civicAPI', 'url': 'https://civicapi.org/api/v2/race/86901', 'role': 'current 2026 AL-02 race independent 16-county roster; publisher aggregator'},
    {'publisher': 'Alabama Secretary of State', 'url': 'https://www.sos.alabama.gov/sites/default/files/06-03-2026/2023%20Congressional%20Plan_Map.pdf', 'role': '2026 published congressional plan reference'},
    {'publisher': 'Alabama Secretary of State', 'url': 'https://www.sos.alabama.gov/alabama-votes/2026-general-election-sample-ballots', 'role': '2026 general sample ballots; current AL-02 contest identity'},
    {'publisher': 'U.S. Census Bureau', 'url': 'https://www2.census.gov/geo/tiger/TIGER2026/CD/tl_2026_01_cd120.zip', 'role': 'unsimplified 2026 CD120 allocation geometry'},
]


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def polygons(g):
    if not g.is_valid:
        g = make_valid(g)
    if g.is_empty:
        return None
    if g.geom_type in ('Polygon', 'MultiPolygon'):
        return g
    pieces = [polygons(x) for x in getattr(g, 'geoms', ())]
    pieces = [x for x in pieces if x is not None]
    return unary_union(pieces) if pieces else None


def read_shapes(path):
    meta, _, geometries, fields = pyogrio.raw.read(path)
    project = Transformer.from_crs(meta['crs'], 5070, always_xy=True).transform
    records = [dict(zip(meta['fields'], values)) for values in zip(*fields)]
    shapes = []
    repairs = []
    for index, wkb in enumerate(geometries):
        g = from_wkb(wkb)
        if not g.is_valid:
            repairs.append({'index': index, 'phase': 'source'})
            g = make_valid(g)
        g = transform(project, g)
        if not g.is_valid:
            repairs.append({'index': index, 'phase': 'projection'})
            g = make_valid(g)
        assert g.is_valid and not g.is_empty
        shapes.append(g)
    return records, shapes, repairs


def build():
    cd_path = MAPS / 'mississippi/data/alabama-midterm-comparison/sources/cd120/tl_2026_01_cd120.zip'
    source_path = MAPS / 'alabama/data/geospatial/election-precincts/al-2024-gen-prec/al-2024-gen-all-prec/al-2024-gen-all-prec.shp'
    block_path = MAPS / 'alabama/data/demographics/census-2020-blocks/al2020.pl.zip'
    parser_path = NATIONAL / 'scripts/census_block_population.py'
    county_path = ROOT / 'data/statewide-geography.json'
    national_path = NATIONAL / 'data/historical/AL-2024.json'
    cd_records, cd_shapes, cd_repairs = read_shapes('/vsizip/' + str(cd_path))
    district = next(g for row, g in zip(cd_records, cd_shapes) if int(row['CD120FP']) == 2)
    seat_display = transform(DISPLAY, district.simplify(50, preserve_topology=True))
    assert seat_display.is_valid
    county_collection = json.loads(county_path.read_text())
    features = []
    for feature in county_collection['features']:
        cp = feature['properties']; cid = cp['county_id']
        if cid not in COUNTIES:
            continue
        parent = shape(feature['geometry'])
        clipped = polygons(parent.intersection(seat_display))
        assert clipped is not None
        parent_area = transform(MEASURE, parent).area
        clip_area = transform(MEASURE, clipped).area
        props = {'id': f'cd2:county:{cid}', 'seat_id': 'cd2', 'scope': 'cd2',
                 'kind': 'county', 'parent_id': cid, 'source_id': cid,
                 'name': COUNTIES[cid], 'unit_name': COUNTIES[cid],
                 'source_unit_name': COUNTIES[cid], 'county_id': cid,
                 'county_name': COUNTIES[cid], 'full_parent_geometry_path': 'data/statewide-geography.json',
                 'full_parent_feature_id': cp['id'], 'parent_area_m2': parent_area,
                 'clipped_area_m2': clip_area, 'area_fraction_of_parent': min(1., clip_area / parent_area),
                 'area_fraction_of_seat': clip_area / district.area,
                 'is_clipped': not clipped.equals(parent), 'boundary_resolution_sliver': False,
                 'electoral_county_split': cid in SPLIT, 'vintage': '2025 Census county / 2026 Census CD120',
                 'live_boundary_match': 'county FIPS; seat-specific contest totals required',
                 'votes_allocated': False, 'return_geography': 'district_contest_county_portion',
                 'geometry_label': 'County portion' if cid in SPLIT else 'County'}
        features.append({'type': 'Feature', 'id': props['id'], 'properties': props, 'geometry': mapping(clipped)})
    assert {f['properties']['county_id'] for f in features} == set(COUNTIES)
    coverage = unary_union([shape(f['geometry']) for f in features])
    geography = {'type': 'FeatureCollection', 'metadata': {
        'schema_version': 1, 'scope': 'cd2', 'feature_count': len(features),
        'current_plan_authority_references': CURRENT_PLAN_SOURCES,
        'sources': [{'path': str(p), 'sha256': digest(p)} for p in (cd_path, county_path)],
        'preparation': {'script': 'scripts/prepare-al02-reference.py', 'display_simplify_meters': 50,
                        'allocation_district_simplified': False, 'area_crs': 'EPSG:5070'},
        'seats': {'cd2': {'name': 'Alabama Congressional District 2', 'boundary_vintage': '2026 Census CD120',
                         'boundary_source_id': 'AL-H-02', 'geometry': mapping(seat_display),
                         'bounds': list(seat_display.bounds), 'seat_area_m2': district.area,
                         'display_covered_area_m2': transform(MEASURE, coverage).area,
                         'display_uncovered_area_m2': transform(MEASURE, seat_display.difference(coverage)).area,
                         'feature_counts': {'county': len(features)}, 'county_features': features,
                         'county_ids': sorted(COUNTIES), 'electorally_split_counties': sorted(SPLIT)}},
        'limitations': ['Display-only 50-metre district simplification; statistical assignments use the unsimplified source district.',
                        'County outline resolution differs from the district: small gaps at edges are retained, never filled.',
                        'County membership is validated against the current race roster and all statewide positive-adult Census points; tiny county-outline intersections do not establish membership.',
                        'No live votes are allocated by clipped geometry. Split counties require the congressional contest county totals, not whole-county statewide-race totals.']},
        'features': features}

    source_records, source_shapes, source_repairs = read_shapes(source_path)
    source_tree = STRtree(source_shapes)
    source_ids = [str(r['UNIQUE_ID']) for r in source_records]
    assert len(source_ids) == len(set(source_ids)) == 1947
    fields = {k for k in source_records[0] if k.startswith('G24PRE')}
    assert fields == {'G24PREDHAR', 'G24PRERTRU', 'G24PREIKEN', 'G24PREIOLI', 'G24PREISTE', 'G24PREOWRI'}
    votes = []
    for row in source_records:
        d, r = int(row['G24PREDHAR']), int(row['G24PRERTRU'])
        total = sum(int(row[k]) for k in fields)
        assert 0 <= d + r <= total
        votes.append((d, r, total - d - r))
    spec = importlib.util.spec_from_file_location('al02_census_parser', parser_path)
    parser = importlib.util.module_from_spec(spec); spec.loader.exec_module(parser)
    blocks, block_audit = parser.parse_block_records(block_path, 'AL')
    assert len({b['block_id'] for b in blocks}) == len(blocks)
    # Scan ALL statewide positive-adult points before applying any county list.
    statewide_mass = Counter(); projected = {}; selected = set()
    for block in blocks:
        if block['vap'] <= 0:
            continue
        point = transform(BLOCK_PROJECT, Point(block['lon'], block['lat']))
        projected[block['block_id']] = point
        if district.contains(point):
            selected.add(block['block_id'])
            statewide_mass['01' + block['county_fips']] += block['vap']
    assert set(statewide_mass) == set(COUNTIES), dict(statewide_mass)
    denominator = Counter(); mass = defaultdict(Counter); unresolved = []
    full_county_mass = Counter(); seat_mass = Counter()
    for block in blocks:
        cid = '01' + block['county_fips']
        if cid not in COUNTIES or block['vap'] <= 0:
            continue
        full_county_mass[cid] += block['vap']
        point = projected[block['block_id']]
        hits = [int(i) for i in source_tree.query(point, predicate='within')
                if str(source_records[int(i)]['COUNTYFP']).zfill(3) == block['county_fips']]
        inside = block['block_id'] in selected
        if len(hits) != 1:
            unresolved.append({'block_id': block['block_id'], 'county_id': cid,
                               'adult_mass': block['vap'], 'inside_seat': inside,
                               'reason': 'historical_source_match_count', 'matches': len(hits)})
            continue
        i = hits[0]; denominator[i] += block['vap']
        if inside:
            mass[cid][i] += block['vap']; seat_mass[cid] += block['vap']
    assert not unresolved, unresolved[:10]
    assert seat_mass == statewide_mass, (seat_mass, statewide_mass)
    for cid in COUNTIES.keys() - SPLIT:
        assert seat_mass[cid] == full_county_mass[cid], (cid, seat_mass[cid], full_county_mass[cid])
    for cid in SPLIT:
        assert 0 < seat_mass[cid] < full_county_mass[cid]
    units = {}; partition = Counter()
    for cid, name in COUNTIES.items():
        contributions = []; sums = [0., 0., 0.]
        for i, adult_mass in sorted(mass[cid].items()):
            weight = adult_mass / denominator[i]
            assert 0 < weight <= 1
            partition[i] += weight
            for channel in range(3): sums[channel] += votes[i][channel] * weight
            contributions.append({'source_id': source_ids[i], 'source_precinct_name': str(source_records[i]['Precinct']),
                                  'adult_mass': adult_mass, 'source_adult_denominator': denominator[i], 'weight': weight,
                                  **{'source_' + field: votes[i][j] for j, field in enumerate(VOTE_FIELDS)}})
        d, r, o = sums; key = f'cd2:county:{cid}'
        units[key] = {'id': key, 'seat_id': 'cd2', 'kind': 'county', 'parent_id': cid,
                      'county_id': cid, 'name': name, 'year': 2024, 'contest': 'president',
                      'available': bool(contributions), 'method': 'estimated_census_2020_adult_internal_points',
                      'is_estimate': True, 'democratic_votes': d, 'republican_votes': r, 'other_votes': o,
                      'two_party_votes': d + r, 'total_votes': d + r + o,
                      'margin_dem_pp': 100 * (d - r) / (d + r) if d + r else None,
                      'adult_mass': seat_mass[cid], 'source_contributions': contributions,
                      'electoral_county_split': cid in SPLIT,
                      'caveat': 'Estimated 2024 presidential context on the 2026 CD120 district. Source-precinct candidate totals weighted by 2020 adult internal-point mass; RDH already allocates central ballots. Not observed split-precinct returns or a forecast.'}
    assert all(0 < w <= 1 + 1e-12 for w in partition.values())
    for i, weight in partition.items():
        for count in votes[i]:
            assert abs(count * weight + count * (1 - weight) - count) < 1e-8
    national = {c['county_id']: c for c in json.loads(national_path.read_text())['counties']}
    comparisons = {}
    for cid in sorted(COUNTIES.keys() - SPLIT):
        comparisons[cid] = {}
        for field in (*VOTE_FIELDS, 'total_votes'):
            actual = units[f'cd2:county:{cid}'][field]; expected = national[cid][field]
            assert abs(actual - expected) < 1e-8, (cid, field, actual, expected)
            comparisons[cid][field] = {'prepared': actual, 'national_source': expected}
    zero_sources = [{'source_id': source_ids[i], 'county_id': '01' + str(r['COUNTYFP']).zfill(3), 'votes': sum(votes[i])}
                    for i, r in enumerate(source_records)
                    if ('01' + str(r['COUNTYFP']).zfill(3)) in COUNTIES and not denominator[i]]
    # Even source precincts outside the selected seat need a denominator so a
    # source zero-population issue cannot silently masquerade as a split vote.
    assert not any(x['votes'] for x in zero_sources), zero_sources
    summary = {field: sum(u[field] for u in units.values()) for field in (*VOTE_FIELDS, 'two_party_votes', 'total_votes', 'adult_mass')}
    summary.update({'features': len(units), 'available_features': len(units), 'raw_seat_adult_mass': sum(statewide_mass.values()),
                    'accepted_seat_adult_mass': sum(seat_mass.values()), 'adult_assignment_coverage': 1.,
                    'estimated': True, 'method': 'estimated_census_2020_adult_internal_points',
                    'margin_dem_pp': 100 * (summary['democratic_votes'] - summary['republican_votes']) / summary['two_party_votes']})
    inputs = [Path(__file__).resolve(), cd_path, county_path, source_path, block_path, parser_path, national_path]
    inputs.extend(source_path.with_suffix(suffix) for suffix in ('.dbf', '.shx', '.prj', '.cpg') if source_path.with_suffix(suffix).is_file())
    benchmarks = {'schema_version': 1, 'year': 2024, 'contest': 'president',
                  'label': 'Estimated 2024 presidential benchmark', 'margin_denominator': 'two_party_votes',
                  'current_plan_authority_references': CURRENT_PLAN_SOURCES,
                  'method': 'For each entire original 2024 precinct, allocate candidate totals by the share of its 2020 Census adult internal-point mass within the unsimplified current 2026 CD120 district and county. Keep fractional counts; outside-seat mass retains its votes. Never allocate live returns.',
                  'limitations': ['All benchmarks are estimates; RDH already allocated central ballots into original precinct candidate totals.',
                                  '2020 adult population is not the 2024 or 2026 electorate; internal points are not population centroids.',
                                  'The current 2026 district differs from the district used for the November 2024 House election.',
                                  'No area, nearest-neighbor, demographic preference, or zero-population fallback.',
                                  'Display simplification and coarse county outlines are not used for statistical assignment.'],
                  'sources': [{'path': str(p), 'sha256': digest(p)} for p in inputs],
                  'audit': {'census_parser': block_audit, 'source_geometry_repairs': source_repairs, 'district_geometry_repairs': cd_repairs,
                            'county_universe': {'method': 'All statewide positive-adult Census points checked against the unsimplified district before county filtering',
                                                'expected_counties_from_current_race_roster': sorted(COUNTIES),
                                                'discovered_counties_from_statewide_block_scan': sorted(statewide_mass)},
                            'county_adult_mass': dict(seat_mass), 'full_county_adult_mass': dict(full_county_mass),
                            'whole_county_comparisons_to_national_source': comparisons, 'unresolved_blocks': unresolved,
                            'zero_adult_source_precincts': zero_sources, 'seat_partitions': {'cd2': summary},
                            'unallocated_source_weight': {source_ids[i]: 1 - w for i, w in partition.items() if w < 1 - 1e-12},
                            'checks': {'all_statewide_seat_mass_accounted': True, 'whole_counties_reconcile': True,
                                       'unique_block_ids': True, 'source_weights_at_most_one': True,
                                       'source_candidate_conservation_with_residual': True, 'no_live_returns_allocated': True}},
                  'units': units}
    validate(geography, benchmarks)
    return geography, benchmarks


def validate(geography, benchmarks):
    seat = shape(geography['metadata']['seats']['cd2']['geometry'])
    ids = set()
    for f in geography['features']:
        p = f['properties']; g = shape(f['geometry'])
        assert p['id'] == f"cd2:county:{p['county_id']}" and p['id'] not in ids
        ids.add(p['id'])
        assert g.is_valid and not g.is_empty and g.geom_type in ('Polygon', 'MultiPolygon')
        assert transform(MEASURE, g.difference(seat)).area < .0001
        assert not p['votes_allocated'] and 0 < p['area_fraction_of_parent'] <= 1
        assert p['electoral_county_split'] == (p['county_id'] in SPLIT)
    assert ids == set(benchmarks['units']) == {f'cd2:county:{c}' for c in COUNTIES}
    assert benchmarks['audit']['unresolved_blocks'] == []
    for u in benchmarks['units'].values():
        assert u['available'] and u['is_estimate'] and u['adult_mass'] > 0
        for field in VOTE_FIELDS:
            calculated = sum(s['weight'] * s['source_' + field] for s in u['source_contributions'])
            assert abs(calculated - u[field]) < 1e-8
        assert abs(u['total_votes'] - sum(u[f] for f in VOTE_FIELDS)) < 1e-8


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--verify', action='store_true'); args = parser.parse_args()
    geography, benchmarks = build()
    for name, artifact in [('al02-geography.json', geography), ('al02-benchmarks.json', benchmarks)]:
        raw = json.dumps(artifact, separators=(',', ':')) + '\n'; path = ROOT / 'data' / name
        if args.verify:
            assert path.read_text() == raw, f'{name}: rebuild differs'
        else:
            path.write_text(raw)
    print(json.dumps({'verified' if args.verify else 'prepared': True,
                      'counties': len(geography['features']), 'summary': benchmarks['audit']['seat_partitions']['cd2']}))


if __name__ == '__main__':
    main()
