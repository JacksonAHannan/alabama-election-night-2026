#!/usr/bin/env python3
"""Prepare seat-clipped reference geography, with no vote allocation.

Preparation dependencies: Shapely and PyProj. Run with Election-Maps/.venv/bin/python.
--verify compares a fresh deterministic build with the checked-in artifact.
"""
from __future__ import annotations
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import sys
import pyproj
import shapely
from pyproj import Transformer
from shapely import make_valid
from shapely.geometry import shape, mapping
from shapely.ops import transform, unary_union

ROOT = Path(__file__).resolve().parents[1]
SEATS = ('cd5', 'hd10', 'hd20', 'hd21', 'hd25', 'sd2', 'sd7')
MEASURE = Transformer.from_crs(4326, 5070, always_xy=True).transform

def polygons(geom):
    """Keep area geometry; intersections may also contain boundary-only lines."""
    if geom.is_empty:
        return None
    if not geom.is_valid:
        geom = make_valid(geom)
    if geom.geom_type in ('Polygon', 'MultiPolygon'):
        return geom
    pieces = [polygons(g) for g in getattr(geom, 'geoms', ())]
    pieces = [g for g in pieces if g is not None]
    return unary_union(pieces) if pieces else None

def area(geom):
    return transform(MEASURE, geom).area

def load(relative):
    raw = (ROOT / relative).read_bytes()
    return json.loads(raw), {'path': relative, 'sha256': hashlib.sha256(raw).hexdigest()}

def build():
    current, current_source = load('data/current-geography.json')
    reference, reference_source = load('data/geography.json')
    precincts = [(f['properties'], shape(f['geometry'])) for f in current['features']]
    counties = [(f['properties'], shape(f['geometry'])) for f in reference['features']
                if f['properties']['kind'] == 'county']
    seats = {f['properties']['scope']: f for f in reference['features']
             if f['properties']['kind'] == 'district'}
    assert set(seats) == set(SEATS)
    assert len(precincts) == 79
    madison = next(g for p, g in counties if p['county_id'] == '01089')
    precinct_union = unary_union([g for p, g in precincts])
    features = []
    seat_metadata = {}
    for seat_id in SEATS:
        seat_feature = seats[seat_id]
        seat = shape(seat_feature['geometry'])
        seat_area = area(seat)
        start = len(features)
        for kind, parents, source_path in (
            ('precinct', precincts, current_source['path']),
            ('county', counties, reference_source['path']),
        ):
            for parent, parent_geom in parents:
                if kind == 'county' and parent['county_id'] == '01089':
                    continue
                clipped = polygons(parent_geom.intersection(seat))
                if clipped is None:
                    continue
                # County fragments represent areas outside Madison only. The
                # provider precinct union also excludes cross-source overlaps.
                if kind == 'county':
                    clipped = polygons(clipped.difference(madison).difference(precinct_union))
                    if clipped is None:
                        continue
                clipped_area = area(clipped)
                if clipped_area <= 0:
                    continue
                parent_area = area(parent_geom)
                parent_id = parent['source_id'] if kind == 'precinct' else parent['county_id']
                fraction = min(1., clipped_area / parent_area)
                # Tiny county intersections arise where coarse county edges
                # disagree with detailed seat polygons. Retain and flag them;
                # they do not establish electoral county membership.
                sliver = kind == 'county' and clipped_area < 250_000 and clipped_area / seat_area < .001
                props = {
                    'id': f'{seat_id}:{kind}:{parent_id}', 'seat_id': seat_id,
                    'scope': seat_id, 'kind': kind, 'parent_id': parent_id,
                    'name': parent['name'], 'unit_name': parent['name'],
                    'source_id': parent_id, 'source_unit_name': parent['name'],
                    'county_id': parent['county_id'], 'county_name': parent['county_name'],
                    'full_parent_geometry_path': source_path,
                    'full_parent_feature_id': parent['id'],
                    'parent_area_m2': parent_area, 'clipped_area_m2': clipped_area,
                    'area_fraction_of_parent': fraction,
                    'area_fraction_of_seat': clipped_area / seat_area,
                    'is_clipped': not parent_geom.equals(clipped),
                    'boundary_resolution_sliver': sliver,
                    'vintage': parent['vintage'],
                    'live_boundary_match': parent['live_boundary_match'],
                    'votes_allocated': False,
                    'return_geography': 'whole_source_reporting_unit',
                    'geometry_label': 'County portion' if kind == 'county' else 'Precinct portion',
                }
                features.append({'type': 'Feature', 'properties': props, 'geometry': mapping(clipped)})
        pieces = features[start:]
        coverage = unary_union([shape(f['geometry']) for f in pieces])
        counts = Counter(f['properties']['kind'] for f in pieces)
        county_ids = [f['properties']['county_id'] for f in pieces
                      if f['properties']['kind'] == 'county' and not f['properties']['boundary_resolution_sliver']]
        # County-only mode uses seat intersections with complete county
        # outlines, including Madison. This separate metadata collection does
        # not change mixed-mode reporting features or their stable IDs.
        county_features = []
        meaningful_counties = {'01089', *county_ids}
        for parent, parent_geom in counties:
            county_id = parent['county_id']
            if county_id not in meaningful_counties:
                continue
            outline = polygons(parent_geom.intersection(seat))
            if outline is None:
                continue
            parent_area = area(parent_geom)
            clipped_area = area(outline)
            county_features.append({
                'type': 'Feature', 'geometry': mapping(outline),
                'properties': {
                    'id': f'{seat_id}:county:{county_id}',
                    'seat_id': seat_id, 'scope': seat_id, 'kind': 'county',
                    'parent_id': county_id, 'source_id': county_id,
                    'name': parent['name'], 'unit_name': parent['name'],
                    'source_unit_name': parent['name'],
                    'county_id': county_id, 'county_name': parent['county_name'],
                    'full_parent_geometry_path': reference_source['path'],
                    'full_parent_feature_id': parent['id'],
                    'parent_area_m2': parent_area, 'clipped_area_m2': clipped_area,
                    'area_fraction_of_parent': min(1., clipped_area / parent_area),
                    'area_fraction_of_seat': clipped_area / seat_area,
                    'is_clipped': not parent_geom.equals(outline),
                    'boundary_resolution_sliver': False,
                    'vintage': parent['vintage'],
                    'live_boundary_match': parent['live_boundary_match'],
                    'votes_allocated': False,
                    'return_geography': 'whole_source_reporting_unit',
                    'geometry_label': 'County portion',
                },
            })
        seat_metadata[seat_id] = {
            'name': seat_feature['properties']['name'],
            'boundary_vintage': seat_feature['properties']['vintage'],
            'boundary_source_id': seat_feature['properties']['source_id'],
            'geometry': seat_feature['geometry'], 'bounds': list(seat.bounds),
            'seat_area_m2': seat_area, 'display_covered_area_m2': area(coverage),
            'display_uncovered_area_m2': area(seat.difference(coverage)),
            'display_overlap_area_m2': max(0., sum(f['properties']['clipped_area_m2'] for f in pieces) - area(coverage)),
            'madison_reference_area_not_covered_by_provider_map_m2': area(seat.intersection(madison).difference(precinct_union)),
            'feature_counts': dict(counts),
            'county_features': county_features,
            'outside_madison_counties_excluding_boundary_slivers': county_ids,
            'boundary_resolution_sliver_feature_ids': [f['properties']['id'] for f in pieces if f['properties']['boundary_resolution_sliver']],
        }
    artifact = {
        'type': 'FeatureCollection',
        'metadata': {
            'schema_version': 1,
            'sources': [current_source, reference_source],
            'preparation': {'script': 'scripts/prepare-seat-geography.py',
                            'shapely': shapely.__version__, 'pyproj': pyproj.__version__,
                            'clipping_crs': 'EPSG:4326', 'area_measurement_crs': 'EPSG:5070',
                            'operation': 'Exact source polygon intersections in WGS84. No simplification, rounding, nearest joins or vote allocation.'},
            'seats': seat_metadata,
            'feature_count': len(features),
            'limitations': [
                'All fractions describe polygon area, not population, electorate, ballots or votes. They must never be used as vote allocation weights.',
                'Live returns belong to full source reporting units. A clipped display polygon does not establish a precinct/district vote split.',
                'Madison precinct identities match the current official provider map by exact name; no historical precinct crosswalk is established.',
                'County fragments outside Madison subtract the Madison reference outline and current provider precinct union to avoid geographic double display.',
                'County and district reference boundaries have different resolution/vintage. Tiny outside-county intersections are retained as boundary_resolution_sliver and do not prove electoral county membership.',
                'Different source resolutions leave small uncovered gaps. Coverage and gap areas are recorded per seat instead of filled with inferred boundaries.',
                'Administrative ABSENTEE, Provisional and PROVISIONAL ABSENTEE returns have no geographic precinct polygon.',
            ],
        },
        'features': features,
    }
    validate(artifact, seats)
    return artifact

def validate(artifact, seats):
    features = artifact['features']
    assert len({f['properties']['id'] for f in features}) == len(features)
    assert {f['properties']['seat_id'] for f in features} == set(SEATS)
    for feature in features:
        p = feature['properties']; geom = shape(feature['geometry'])
        seat = shape(seats[p['seat_id']]['geometry'])
        assert not geom.is_empty and geom.is_valid and geom.geom_type in ('Polygon', 'MultiPolygon'), p['id']
        # Overlay roundoff can produce boundary-only outside remnants. Reject
        # any positive-area escape beyond a one-square-centimetre tolerance.
        assert area(geom.difference(seat)) < .0001, p['id']
        assert 0 < p['area_fraction_of_parent'] <= 1
        assert p['votes_allocated'] is False
        assert p['kind'] == 'precinct' or p['county_id'] != '01089'
    # County-only outlines are a second view of the same prepared seats.
    # They intentionally include Madison rather than its precinct pieces.
    for seat_id, info in artifact['metadata']['seats'].items():
        outlines = info['county_features']
        expected_ids = {'01089', *info['outside_madison_counties_excluding_boundary_slivers']}
        assert {f['properties']['county_id'] for f in outlines} == expected_ids
        assert len({f['properties']['id'] for f in outlines}) == len(outlines)
        seat = shape(seats[seat_id]['geometry'])
        for feature in outlines:
            p = feature['properties']; geom = shape(feature['geometry'])
            assert geom.is_valid and not geom.is_empty and geom.geom_type in ('Polygon', 'MultiPolygon')
            assert area(geom.difference(seat)) < .0001
            assert p['id'] == f"{seat_id}:county:{p['county_id']}"
            assert p['parent_id'] == p['source_id'] == p['county_id']
            assert p['kind'] == 'county' and not p['boundary_resolution_sliver']
            assert not p['votes_allocated'] and 0 < p['area_fraction_of_parent'] <= 1
    return True

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify', action='store_true', help='Verify without changing files')
    args = parser.parse_args()
    artifact = build()
    encoded = json.dumps(artifact, separators=(',', ':'), ensure_ascii=False) + '\n'
    path = ROOT / 'data/seat-geography.json'
    if args.verify:
        assert path.read_text() == encoded, 'Prepared artifact differs; rerun without --verify'
    else:
        path.write_text(encoded)
    print(('Verified' if args.verify else 'Prepared') + f' {len(artifact["features"])} valid seat-clipped features; {len(encoded.encode())} bytes.')
    for seat_id, info in artifact['metadata']['seats'].items():
        print(f'{seat_id}: {info["feature_counts"]}; uncovered {info["display_uncovered_area_m2"] / 1e6:.6f} km²; boundary slivers {len(info["boundary_resolution_sliver_feature_ids"])}')

if __name__ == '__main__':
    main()
