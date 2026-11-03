#!/usr/bin/env python3
"""Prepare estimated 2024 presidential context; never allocate live returns.

Run with ../Election-Maps/.venv/bin/python. All GIS imports are preparation only.
Source files in adjacent projects are read only; no network requests are made.
"""
from __future__ import annotations

import collections
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile

import pyogrio
from pyproj import Transformer
from shapely import STRtree, from_wkb, make_valid
from shapely.geometry import Point, shape
from shapely.ops import transform

ROOT = Path(__file__).resolve().parents[1]
MAPS = ROOT.parent / 'Election-Maps'
NATIONAL = ROOT.parent / 'national-election-night-2026'
SEATS = ('cd5', 'hd10', 'hd20', 'hd21', 'hd25', 'sd2', 'sd7')
REPAIRS = []


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_shapes(path, where=None, provenance_path=None):
    meta, _, geometries, fields = pyogrio.raw.read(path, where=where)
    project = Transformer.from_crs(meta['crs'], 5070, always_xy=True).transform
    records = [dict(zip(meta['fields'], values)) for values in zip(*fields)]
    shapes = []
    for index, wkb in enumerate(geometries):
        g = from_wkb(wkb)
        if not g.is_valid:
            REPAIRS.append({'path': str(provenance_path or path), 'index': index, 'phase': 'source'})
            g = make_valid(g)
        g = transform(project, g)
        if not g.is_valid:
            REPAIRS.append({'path': str(provenance_path or path), 'index': index, 'phase': 'projection'})
            g = make_valid(g)
        shapes.append(g)
    assert all(g.is_valid and not g.is_empty for g in shapes)
    return records, shapes


def main():
    geography_path = ROOT / 'data/seat-geography.json'
    current_path = ROOT / 'data/current-geography.json'
    geography = json.loads(geography_path.read_text())
    current = json.loads(current_path.read_text())
    target = {f['properties']['id']: f['properties'] for f in geography['features']}
    assert len(target) == len(geography['features'])
    current_ids = [f['properties']['source_id'] for f in current['features']]
    assert len(current_ids) == len(set(current_ids)) == 79
    for name in current_ids:
        key = 'madison:precinct:' + name
        target[key] = {'id': key, 'seat_id': 'madison', 'kind': 'precinct',
                       'parent_id': name, 'county_id': '01089', 'name': name}
    positive_project = Transformer.from_crs(4326, 5070, always_xy=True).transform
    current_shapes = [transform(positive_project, shape(f['geometry'])) for f in current['features']]
    current_tree = STRtree(current_shapes)

    source_path = MAPS / ('alabama/data/geospatial/election-precincts/al-2024-gen-prec/'
                          'al-2024-gen-all-prec/al-2024-gen-all-prec.shp')
    source_records, source_shapes = read_shapes(source_path)
    source_tree = STRtree(source_shapes)
    source_ids = [str(r['UNIQUE_ID']) for r in source_records]
    assert len(source_ids) == len(set(source_ids)) == 1947
    presidential_fields = [k for k in source_records[0] if k.startswith('G24PRE')]
    assert set(presidential_fields) == {'G24PREDHAR', 'G24PRERTRU', 'G24PREIKEN',
                                      'G24PREIOLI', 'G24PREISTE', 'G24PREOWRI'}
    source_votes = []
    for row in source_records:
        dem, rep = int(row['G24PREDHAR']), int(row['G24PRERTRU'])
        total = sum(int(row[k]) for k in presidential_fields)
        assert 0 <= dem + rep <= total
        source_votes.append((dem, rep, total - dem - rep))

    leg = MAPS / 'alabama/data/geospatial/legislative-districts'
    house = leg / 'tl-2025-01-sldl/tl-2025-01-sldl.shp'
    senate = leg / 'al-sldu-remedial-2025'
    cd_archive = MAPS / 'mississippi/data/alabama-midterm-comparison/sources/cd120/tl_2026_01_cd120.zip'
    districts = {}
    records, shapes = read_shapes('/vsizip/' + str(cd_archive))
    districts['cd5'] = next(g for r, g in zip(records, shapes) if int(r['CD120FP']) == 5)
    records, shapes = read_shapes(house)
    for r, g in zip(records, shapes):
        if int(r['SLDLST']) in (10, 20, 21, 25):
            districts['hd' + str(int(r['SLDLST']))] = g
    with tempfile.TemporaryDirectory() as tmp:
        local = Path(tmp) / 'senate.shp'
        for suffix in ('shp', 'shx', 'prj', 'cpg'):
            shutil.copyfile(senate / ('AL_SD_Remedial2025.' + suffix), local.with_suffix('.' + suffix))
        shutil.copyfile(senate / 'al_sd_remedial2025.dbf', local.with_suffix('.dbf'))
        records, shapes = read_shapes(local, provenance_path=senate / 'AL_SD_Remedial2025.shp')
        for r, g in zip(records, shapes):
            if int(r['DISTRICT']) in (2, 7):
                districts['sd' + str(int(r['DISTRICT']))] = g
    assert set(districts) == set(SEATS)

    parser_path = NATIONAL / 'scripts/census_block_population.py'
    spec = importlib.util.spec_from_file_location('census_block_population', parser_path)
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    block_path = MAPS / 'alabama/data/demographics/census-2020-blocks/al2020.pl.zip'
    blocks, block_audit = parser.parse_block_records(block_path, 'AL')
    assert len({b['block_id'] for b in blocks}) == len(blocks)
    block_project = Transformer.from_crs(4269, 5070, always_xy=True).transform
    # Discover the input universe from ALL statewide Census points. Filtering
    # guessed county FIPS here can silently omit a county and then falsely
    # report 100% coverage of the reduced universe.
    district_tree = STRtree([districts[s] for s in SEATS])
    statewide_seat_blocks = {}
    statewide_seat_mass = collections.Counter()
    discovered_counties = collections.defaultdict(set)
    projected_blocks = {}
    for block in blocks:
        if block['vap'] <= 0:
            continue
        point = transform(block_project, Point(block['lon'], block['lat']))
        selected = [SEATS[int(i)] for i in district_tree.query(point, predicate='within')]
        if selected:
            statewide_seat_blocks[block['block_id']] = selected
            for seat in selected:
                statewide_seat_mass[seat] += block['vap']
                discovered_counties[seat].add('01' + block['county_fips'])
        projected_blocks[block['block_id']] = point
    expected_counties = {
        seat: {f['properties']['county_id']
               for f in geography['metadata']['seats'][seat]['county_features']
               if not f['properties'].get('boundary_resolution_sliver', False)}
        for seat in SEATS}
    assert expected_counties['cd5'] == {'01071', '01077', '01079', '01083', '01089', '01103'}
    for seat in SEATS:
        assert discovered_counties[seat] == expected_counties[seat], (seat, discovered_counties[seat], expected_counties[seat])
    relevant_counties = {fips[-3:] for counties in discovered_counties.values() for fips in counties}
    assert relevant_counties == {'071', '077', '079', '083', '089', '103'}
    denominator = collections.Counter()
    mass = collections.defaultdict(collections.Counter)
    unresolved = []
    seat_inside_mass = collections.Counter()
    seat_accepted_mass = collections.Counter()
    current_assignment = collections.Counter()
    display_disagreement = collections.Counter()
    display_shapes = {s: transform(positive_project, shape(geography['metadata']['seats'][s]['geometry']))
                      for s in SEATS}
    positive_blocks = 0
    for block in blocks:
        if block['county_fips'] not in relevant_counties or block['vap'] <= 0:
            continue
        positive_blocks += 1
        point = projected_blocks[block['block_id']]
        hits = [int(i) for i in source_tree.query(point, predicate='within')
                if str(source_records[int(i)]['COUNTYFP']).zfill(3) == block['county_fips']]
        seats = statewide_seat_blocks.get(block['block_id'], [])
        for s in SEATS:
            if districts[s].contains(point) != display_shapes[s].contains(point):
                display_disagreement[s] += block['vap']
        for s in seats:
            seat_inside_mass[s] += block['vap']
        if len(hits) != 1:
            unresolved.append({'block_id': block['block_id'], 'vap': block['vap'],
                               'reason': 'historical_source_match_count', 'matches': len(hits), 'seats': seats})
            continue
        i = hits[0]
        denominator[i] += block['vap']
        if block['county_fips'] == '089':
            current_hits = current_tree.query(point, predicate='within')
            current_assignment[str(len(current_hits))] += 1
            if len(current_hits) != 1:
                unresolved.append({'block_id': block['block_id'], 'vap': block['vap'],
                                   'reason': 'current_madison_match_count', 'matches': len(current_hits),
                                   'seats': seats, 'historical_id': source_ids[i]})
                continue
            name = current_ids[int(current_hits[0])]
            mass['madison:precinct:' + name][i] += block['vap']
            keys = [s + ':precinct:' + name for s in seats]
        else:
            keys = [s + ':county:01' + block['county_fips'] for s in seats]
        for key in keys:
            if key not in target:
                unresolved.append({'block_id': block['block_id'], 'vap': block['vap'],
                                   'reason': 'missing_display_target', 'target_id': key,
                                   'historical_id': source_ids[i]})
                continue
            mass[key][i] += block['vap']
            seat_accepted_mass[key.split(':', 1)[0]] += block['vap']

    # Weights are relative to each ENTIRE source precinct, including adults
    # outside a selected seat. Never renormalize a partial seat to absorb votes.
    partitions = {s: collections.Counter() for s in (*SEATS, 'madison')}
    records = {}
    for key, props in target.items():
        counts = mass[key]
        contributions = []
        dem = rep = other = 0.
        for i, vap in sorted(counts.items()):
            weight = vap / denominator[i]
            assert 0 < weight <= 1
            d, r, o = source_votes[i]
            dem += d * weight
            rep += r * weight
            other += o * weight
            partitions[props['seat_id']][i] += weight
            contributions.append({'source_id': source_ids[i], 'source_precinct_name': str(source_records[i]['Precinct']),
                                  'adult_mass': vap, 'source_adult_denominator': denominator[i],
                                  'weight': weight, 'source_democratic_votes': d,
                                  'source_republican_votes': r, 'source_other_votes': o})
        available = bool(counts)
        records[key] = {'id': key, 'seat_id': props['seat_id'], 'kind': props['kind'],
                        'parent_id': props['parent_id'], 'county_id': props['county_id'],
                        'name': props['name'], 'year': 2024, 'contest': 'president',
                        'available': available, 'method': 'estimated_census_2020_adult_internal_points' if available else 'unavailable',
                        'is_estimate': available, 'democratic_votes': dem if available else None,
                        'republican_votes': rep if available else None, 'other_votes': other if available else None,
                        'two_party_votes': dem + rep if available else None,
                        'total_votes': dem + rep + other if available else None,
                        'margin_dem_pp': 100 * (dem - rep) / (dem + rep) if dem + rep else None,
                        'adult_mass': sum(counts.values()), 'source_contributions': contributions,
                        'caveat': 'Estimated 2024 presidential context from 2020 adult-population points; RDH already allocates central ballots. Not observed split-precinct results or a forecast.' if available else 'No usable 2020 adult mass; historical benchmark withheld. No area or zero-population fallback.'}

    for seat, weights in partitions.items():
        assert all(0 <= w <= 1 + 1e-12 for w in weights.values()), seat
        # The complementary outside-seat bucket preserves every source
        # candidate total. It is deliberately not redistributed to the seat.
        for i, weight in weights.items():
            for count in source_votes[i]:
                assert abs(count * weight + count * (1 - weight) - count) < 1e-8
    for seat in SEATS:
        assert seat_inside_mass[seat] == statewide_seat_mass[seat]
        assert seat_accepted_mass[seat] == statewide_seat_mass[seat], seat
    for county in expected_counties['cd5'] - {'01089'}:
        assert records['cd5:county:' + county]['available'], county
    # Whole-county components must independently reconcile to the national
    # source import, especially Jackson (071), which a former guessed filter
    # accidentally replaced with DeKalb (049). Lauderdale is a partial county.
    national_path = NATIONAL / 'data/historical/AL-2024.json'
    national = json.loads(national_path.read_text())
    reference_counties = {c['county_id']: c for c in national['counties']}
    county_comparisons = {}
    for county in ('01071', '01079', '01083', '01103'):
        observed = records['cd5:county:' + county]
        expected = reference_counties[county]
        comparison = {}
        for field in ('democratic_votes', 'republican_votes', 'other_votes', 'total_votes'):
            assert abs(observed[field] - expected[field]) < 1e-8, (county, field)
            comparison[field] = {'prepared': observed[field], 'national_source': expected[field]}
        county_comparisons[county] = comparison
    madison_source = [i for i, r in enumerate(source_records) if str(r['COUNTYFP']).zfill(3) == '089']
    assert len(madison_source) == 79
    assert current_assignment == {'1': 5874}, current_assignment
    assert all(abs(partitions['madison'][i] - 1) < 1e-12 for i in madison_source)
    for channel, idx in [('democratic_votes', 0), ('republican_votes', 1), ('other_votes', 2)]:
        actual = sum(r[channel] for r in records.values() if r['seat_id'] == 'madison')
        expected = sum(source_votes[i][idx] for i in madison_source)
        assert abs(actual - expected) < 1e-8, (channel, actual, expected)
    assert all((v is None or v >= 0) for r in records.values()
               for k, v in r.items() if k in ('democratic_votes', 'republican_votes', 'other_votes'))
    zero_source = [{'source_id': source_ids[i], 'county_fips': str(row['COUNTYFP']),
                    'votes': sum(source_votes[i])} for i, row in enumerate(source_records)
                   if str(row['COUNTYFP']).zfill(3) in relevant_counties and denominator[i] == 0]
    seat_audit = {}
    for seat in (*SEATS, 'madison'):
        subset = [r for r in records.values() if r['seat_id'] == seat]
        seat_audit[seat] = {'features': len(subset), 'available_features': sum(r['available'] for r in subset),
                            'democratic_votes': sum(r['democratic_votes'] or 0 for r in subset),
                            'republican_votes': sum(r['republican_votes'] or 0 for r in subset),
                            'other_votes': sum(r['other_votes'] or 0 for r in subset),
                            'adult_mass': sum(r['adult_mass'] for r in subset),
                            'source_weight_maximum': max(partitions[seat].values()),
                            'unallocated_source_weight': {source_ids[i]: 1 - weight
                                                          for i, weight in partitions[seat].items()
                                                          if weight < 1 - 1e-12},
                            'raw_seat_adult_mass': seat_inside_mass[seat] if seat != 'madison' else 304143,
                            'accepted_seat_adult_mass': seat_accepted_mass[seat] if seat != 'madison' else 304143,
                            'display_vs_raw_district_disagreement_adult_mass': display_disagreement[seat]}
        summary = seat_audit[seat]
        summary['two_party_votes'] = summary['democratic_votes'] + summary['republican_votes']
        summary['total_votes'] = summary['two_party_votes'] + summary['other_votes']
        summary['margin_dem_pp'] = 100 * (summary['democratic_votes'] - summary['republican_votes']) / summary['two_party_votes']
        summary['adult_assignment_coverage'] = summary['accepted_seat_adult_mass'] / summary['raw_seat_adult_mass']
        summary['estimated'] = True
        summary['method'] = 'estimated_census_2020_adult_internal_points'
    inputs = [Path(__file__).resolve(), source_path, current_path, geography_path, block_path, parser_path, national_path, house,
              senate / 'AL_SD_Remedial2025.shp', senate / 'al_sd_remedial2025.dbf', cd_archive]
    for path in (source_path, house, senate / 'AL_SD_Remedial2025.shp'):
        inputs.extend(path.with_suffix(suffix) for suffix in ('.dbf', '.shx', '.prj', '.cpg')
                      if path.with_suffix(suffix).is_file())
    inputs = list(dict.fromkeys(inputs))
    output = {'schema_version': 1, 'year': 2024, 'contest': 'president',
              'label': 'Estimated 2024 presidential benchmark', 'margin_denominator': 'two_party_votes',
              'method': 'Apply source-precinct candidate totals uniformly to its 2020 Census adult internal-point mass by current provider reporting unit and unsimplified selected district. Fractional counts retained. Never allocate current returns.',
              'limitations': ['All benchmarks are estimated, including unchanged-looking precincts: raw source already allocates central votes.',
                             '2020 adult population is not the 2024 or 2026 electorate; block internal points are not population centroids.',
                             'Current provider maps are not independent certification of legal boundaries; district reference plan applicability remains subject to source caveats.',
                             'No area, nearest-neighbor, demographic preference, or zero-population fallback.',
                             'District shapes used for allocation are unsimplified; map display outlines are simplified and may disagree near edges.'],
              'sources': [{'path': str(p), 'sha256': digest(p)} for p in inputs],
              'audit': {'census_parser': block_audit, 'geometry_repairs': REPAIRS, 'relevant_positive_vap_blocks': positive_blocks,
                        'whole_cd5_county_comparisons_to_national_source': county_comparisons,
                        'county_universe': {'method': 'All statewide positive-adult Census points tested against all seven raw districts before county filtering',
                                            'expected_counties_from_meaningful_seat_map': {s: sorted(v) for s, v in expected_counties.items()},
                                            'discovered_counties_from_statewide_block_scan': {s: sorted(v) for s, v in discovered_counties.items()},
                                            'raw_seat_adult_mass_from_all_statewide_points': dict(statewide_seat_mass)},
                        'madison_current_match_counts': dict(current_assignment), 'unresolved_blocks': unresolved,
                        'zero_adult_source_precincts': zero_source, 'seat_partitions': seat_audit,
                        'checks': {'nonnegative': True, 'source_weights_at_most_one': True,
                                   'madison_candidate_conservation': True, 'source_candidate_conservation_with_residual': True,
                                   'unique_block_ids': True,
                                   'unique_current_ids': True, 'all_statewide_seat_mass_accounted': True,
                                   'cd5_all_five_outside_madison_counties_available': True,
                                   'whole_cd5_counties_reconcile_to_national_source': True}},
              'units': records}
    (ROOT / 'data/seat-benchmarks.json').write_text(json.dumps(output, separators=(',', ':')) + '\n')
    print(json.dumps({'units': len(records), 'unresolved_blocks': len(unresolved),
                      'zero_adult_sources': zero_source, 'seats': seat_audit}, indent=2))


if __name__ == '__main__':
    main()
