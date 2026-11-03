"""Statewide county views and Madison drilldown; positive fixtures are browser-only."""
import copy,csv,io,json,math
from pathlib import Path
from urllib.request import urlopen
from playwright.sync_api import sync_playwright
BASE='http://127.0.0.1:8052';OUT=Path('runtime/qa');OUT.mkdir(parents=True,exist_ok=True)
data=json.load(urlopen(BASE+'/api/state'))
statewide=[r for r in data['races']if r.get('statewide')]
assert len(statewide)==20
expected={f'01{i:03}'for i in range(1,134,2)}
errors=[]
with sync_playwright()as p:
 b=p.chromium.launch(headless=True);page=b.new_page(viewport={'width':1512,'height':1050});page.on('pageerror',lambda e:errors.append(str(e)))
 page.goto(BASE+'/#race/governor',wait_until='networkidle')
 page.select_option('#race-group','statewide');assert page.locator('.race-choice').count()==20;page.select_option('#race-group','all')
 for r in statewide:
  page.goto(BASE+'/#race/'+r['id'],wait_until='networkidle')
  assert page.locator('#race-coverage').input_value()=='alabama'
  assert page.locator('#map .area').count()==67,r['id']
  assert page.locator('#map .area[data-kind="county"]').count()==67
  assert not page.get_by_role('heading',name='Precinct returns',exact=True).is_visible()
  assert page.evaluate('benchmarkRecords().length')==67
  assert math.isclose(page.evaluate('calibration().calibratedDemShare'),50,abs_tol=1e-7)
  assert page.locator('#county-table tbody tr').count()==len(r['county_results'])
  if r['statewide_feed_available']:
   assert {c['county_id']for c in r['county_results']}==expected,(r['id'],'feed not yet cached')
   assert next(c for c in r['county_results']if c['county_id']=='01089')['source_id']=='madison'
  else:
   assert 'Statewide county feed unavailable' in page.locator('.notice').first.inner_text()
   assert len(r['county_results'])==1
 page.goto(BASE+'/#race/governor',wait_until='networkidle')
 page.select_option('#map-layer','historical');assert page.locator('#map .area:not([fill="#deded8"])').count()==67
 page.locator('#map .area[data-feature-id="alabama:county:01089"]').focus();page.locator('#map .area[data-feature-id="alabama:county:01089"]').press('Enter')
 assert '2024 presidential reference' in page.locator('#selection').inner_text()
 page.locator('#county-search').fill('Limestone');assert page.locator('#county-table tbody tr').count()==1
 page.locator('#target').fill('55');page.locator('#target').press('Tab');assert math.isclose(page.evaluate('calibration().calibratedDemShare'),55,abs_tol=1e-7)
 page.locator('#county-search').fill('Morgan');assert page.locator('#county-table tbody tr').count()==1
 page.locator('#county-search').fill('');assert page.locator('#county-table tbody tr').count()==67
 page.select_option('#target-party','rep');assert math.isclose(page.evaluate('calibration().calibratedDemShare'),45,abs_tol=1e-7)
 with page.expect_download()as dl:page.locator('#export-counties').click()
 dl.value.save_as(str(OUT/'statewide-counties.csv'));rows=list(csv.DictReader(io.StringIO((OUT/'statewide-counties.csv').read_text())))
 assert {r['county_fips']for r in rows}==expected
 page.locator('#benchmark-disclosure summary').click()
 with page.expect_download()as dl:page.locator('#export-benchmarks').click()
 dl.value.save_as(str(OUT/'statewide-benchmarks.csv'));rows=list(csv.DictReader(io.StringIO((OUT/'statewide-benchmarks.csv').read_text())))
 assert len(rows)==67 and sum(float(r['democratic_2024'])for r in rows)==772412
 page.evaluate('window.scrollTo(0,0)');page.screenshot(path=str(OUT/'statewide-desktop.png'),full_page=False)
 page.select_option('#race-coverage','madison');assert page.locator('#map .area').count()==79
 assert page.get_by_role('heading',name='Precinct returns',exact=True).is_visible()
 assert page.locator('#county-table tbody tr').count()==1 and page.evaluate('benchmarkScope()')=='madison'
 assert len(page.evaluate('race().units'))==82
 page.select_option('#race-coverage','alabama');assert page.locator('#map .area').count()==67
 page.goto(BASE+'/#race/amendment-1',wait_until='networkidle');assert 'D/R target gap is not defined' in page.locator('#target-result').inner_text()
 page.select_option('#map-layer','live');assert 'Yes +30' in page.locator('#legend').inner_text()
 assert 'Write-ins not supplied' not in page.locator('#county-table').inner_text()
 page.set_viewport_size({'width':390,'height':844});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 page.locator('#map').scroll_into_view_if_needed();page.screenshot(path=str(OUT/'statewide-mobile.png'),full_page=False)
 # Positive county and administrative observations, intercepted for this page only.
 fixture=copy.deepcopy(data);r=next(r for r in fixture['races']if r['id']=='governor')
 for name,d,rep in [('Autauga',60,40),('Madison',80,70)]:
  c=next(c for c in r['county_results']if c['county']==name);c.update(total=d+rep,candidates=[{'name':'Doug Jones','party':'DEM','votes':d},{'name':'Tommy Tuberville','party':'REP','votes':rep}])
 r['contributions']=copy.deepcopy(r['county_results']);r.update(total=250,candidates=[{'name':'Doug Jones','party':'DEM','votes':140},{'name':'Tommy Tuberville','party':'REP','votes':110}])
 unit=next(u for u in r['units']if not u['administrative']and any(x['parent_id']==u['name']and x['seat_id']=='madison'and x['available']for x in json.load(open('data/seat-benchmarks.json'))['units'].values()))
 unit.update(total=100,candidates=[{'name':'Doug Jones','party':'DEM','votes':50},{'name':'Tommy Tuberville','party':'REP','votes':50}])
 admin=next(u for u in r['units']if u['administrative']);admin.update(total=50,candidates=[{'name':'Doug Jones','party':'DEM','votes':30},{'name':'Tommy Tuberville','party':'REP','votes':20}]);r['units']=[unit,admin]
 positive=b.new_page();positive.on('pageerror',lambda e:errors.append(str(e)));positive.route('**/api/state',lambda route:route.fulfill(json=fixture))
 positive.goto(BASE+'/#race/governor',wait_until='networkidle')
 assert '2 reported areas' in positive.locator('#target-result').inner_text()
 assert page.locator('#error').is_hidden()
 assert positive.evaluate('race().total')==250
 assert positive.locator('#map .area:not([fill="#deded8"])').count()==2
 expected_gap=positive.evaluate("(()=>{const cs=race().county_results.filter(c=>c.total>0);return ElectionBenchmarks.compare(cs.map(c=>({d:twoParty(c).d,r:twoParty(c).r,targetDem:targetFor(benchmarkForCounty(c))}))).gap})()")
 assert f'{expected_gap:+.1f} pp' in positive.locator('#target-result').inner_text()
 positive.select_option('#race-coverage','madison');assert positive.evaluate('race().total')==150
 assert '1 reported areas' in positive.locator('#target-result').inner_text()
 positive.select_option('#race-coverage','alabama');assert '2 reported areas' in positive.locator('#target-result').inner_text()
 positive.close()
 amendment=copy.deepcopy(data);r=next(r for r in amendment['races']if r['id']=='amendment-1');c=next(c for c in r['county_results']if c['county']=='Autauga');c.update(total=100,candidates=[{'name':'Yes','party':'YES','votes':70},{'name':'No','party':'NO','votes':30}])
 yesno=b.new_page();yesno.on('pageerror',lambda e:errors.append(str(e)));yesno.route('**/api/state',lambda route:route.fulfill(json=amendment));yesno.goto(BASE+'/#race/amendment-1',wait_until='networkidle')
 assert yesno.locator('#map .area[data-feature-id="alabama:county:01001"]').get_attribute('fill')=='#247357'
 yesno.select_option('#map-layer','target');assert yesno.locator('#map .area:not([fill="#deded8"])').count()==0
 yesno.close();assert not errors,errors;b.close()
print('PASS: 20 statewide views, 12 complete 67-county feeds, 8 explicit gaps, historical maps/calibration, county search/exports, Madison drilldown, positive county/admin comparisons, desktop/mobile; no browser errors')
