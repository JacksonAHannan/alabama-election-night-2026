"""Independent integration audit of clipped geometry and historical model output."""
import hashlib,json,math
from pathlib import Path
from shapely.geometry import shape
ROOT=Path(__file__).resolve().parents[1]
geo=json.loads((ROOT/'data/seat-geography.json').read_text())
bench=json.loads((ROOT/'data/seat-benchmarks.json').read_text())
records=bench['units']
expected={'cd5':{'01071','01077','01079','01083','01103'},'hd10':set(),'hd20':set(),'hd21':set(),'hd25':{'01083'},'sd2':{'01083'},'sd7':set()}
assert len(geo['features'])==len({f['properties']['id']for f in geo['features']})==270
assert len(records)==349
for scope,county_ids in expected.items():
 seat=shape(geo['metadata']['seats'][scope]['geometry'])
 features=[f for f in geo['features']if f['properties']['seat_id']==scope]
 visible=[f for f in features if not f['properties']['boundary_resolution_sliver']]
 outside={f['properties']['county_id']for f in visible if f['properties']['kind']=='county'}
 assert outside==county_ids,(scope,outside,county_ids)
 for f in features:
  g=shape(f['geometry']);assert g.is_valid and not g.is_empty
  assert g.difference(seat.buffer(1e-10)).area<1e-12,(scope,f['properties']['id'])
  rec=records[f['properties']['id']]
  if rec['available']:
   assert rec['is_estimate'] and rec['democratic_votes']>=0 and rec['republican_votes']>=0
   assert math.isclose(rec['two_party_votes'],rec['democratic_votes']+rec['republican_votes'],abs_tol=1e-7)
   for key in ['democratic_votes','republican_votes','other_votes']:
    source_key='source_'+key
    expected_votes=sum(c[source_key]*c['weight']for c in rec['source_contributions'])
    assert math.isclose(expected_votes,rec[key],abs_tol=1e-7),(rec['id'],key)
  else:
   assert rec['two_party_votes']is None and rec['democratic_votes']is None
 for county in county_ids:
  rec=records[f'{scope}:county:{county}'];assert rec['available'],f'Missing expected county benchmark: {scope}/{county}'
 assert {f['properties']['county_id']for f in geo['metadata']['seats'][scope]['county_features']}==county_ids|{'01089'}
 assert bench['audit']['seat_partitions'][scope]['adult_assignment_coverage']==1
# Whole counties in CD5 must reproduce source 2024 sums, independently of the builder's county-selection loop.
source=json.loads((ROOT.parent/'national-election-night-2026/data/historical/AL-2024.json').read_text())
rows=source.get('units',source.get('rows',[]))
assert rows,'Historical source units unavailable'
for county in ['01071','01079','01083','01103']:
 relevant=[u for u in rows if u.get('county_id')==county]
 assert relevant,county
 for k in ['democratic_votes','republican_votes','other_votes']:
  expected_votes=sum(u[k]for u in relevant)
  actual=records[f'cd5:county:{county}'][k]
  assert math.isclose(actual,expected_votes,abs_tol=1e-7),(county,k,actual,expected_votes)
mad=[u for u in records.values()if u['seat_id']=='madison']
assert len(mad)==79
assert sum(u['democratic_votes']for u in mad)==87824
assert sum(u['republican_votes']for u in mad)==105430
assert sum(u['total_votes']for u in mad)==197617
print('PASS: all seven seat clips,270features,349references,expectedoutsidecounties,independentfullcountyvotechecks,Madisonconservation and nullunavailablebenchmarks')
