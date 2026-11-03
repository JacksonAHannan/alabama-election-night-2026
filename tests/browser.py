"""Real cached returns plus isolated synthetic browser fixtures; never writes live results."""
import csv,io,json,math
from pathlib import Path
from urllib.request import urlopen
from playwright.sync_api import sync_playwright
BASE='http://127.0.0.1:8052';OUT=Path('runtime/qa');OUT.mkdir(parents=True,exist_ok=True)
data=json.load(urlopen(BASE+'/api/state'));geo=json.load(open('data/seat-geography.json'));bench=json.load(open('data/seat-benchmarks.json'))
ids=['cd5','hd10','hd20','hd21','hd25','sd2','sd7']
assert len(data['races'])==len({r['id']for r in data['races']})==50 and data['current_map_compatible']
for rid,expected in [('cd5',6),('sd2',2),('hd25',2)]:
 r=next(r for r in data['races']if r['id']==rid);assert len(r['county_results'])==expected
 assert next(c for c in r['county_results']if c['county']=='Madison')['source_id']=='madison'
 assert next(c for c in r['county_results']if c['county']=='Limestone')['source_id'].startswith('civic-')
 assert all(u['county']=='Madison'for u in r['units'])
errors=[]
with sync_playwright()as p:
 b=p.chromium.launch(headless=True);page=b.new_page(viewport={'width':1512,'height':1050});page.on('pageerror',lambda e:errors.append(str(e)))
 page.goto(BASE+'/#race/hd10',wait_until='networkidle');page.locator('#target-result').wait_for()
 for rid in ids:
  page.goto(BASE+'/#race/'+rid,wait_until='networkidle')
  expected=[f for f in geo['features']if f['properties']['seat_id']==rid and not f['properties']['boundary_resolution_sliver']]
  actual=set(page.locator('#map .area').evaluate_all('(els)=>els.map(e=>e.dataset.featureId)'))
  assert actual=={f['properties']['id']for f in expected},rid
  assert page.locator('#seat-clip').count()==1
  assert 'clipped to the selected seat' in page.locator('#map-note').inner_text()
  assert page.locator('#target-result').inner_text().startswith('2024 D reference:')
  assert math.isclose(page.evaluate('calibration().calibratedDemShare'),50,abs_tol=1e-7)
 page.goto(BASE+'/#race/hd25',wait_until='networkidle')
 limestone=page.locator('#county-table tbody tr').filter(has=page.get_by_text('Limestone',exact=True)).first
 assert 'County totals only' in limestone.inner_text()
 assert 'Estimated geography' in limestone.inner_text()
 assert page.locator('#map .area[data-kind="county"]').count()==1
 page.select_option('#map-layer','historical')
 available=sum(bench['units'][f['properties']['id']]['available']for f in geo['features']if f['properties']['seat_id']=='hd25'and not f['properties']['boundary_resolution_sliver'])
 assert page.locator('#map .area:not([fill="#deded8"])').count()==available
 page.locator('#map .area').first.focus();page.locator('#map .area').first.press('Enter');assert '2024' in page.locator('#selection').inner_text();page.get_by_role('button',name='Clear',exact=True).click()
 page.locator('#target').fill('55');page.locator('#target').press('Tab')
 assert math.isclose(page.evaluate('calibration().calibratedDemShare'),55,abs_tol=1e-7)
 page.select_option('#target-party','rep')
 assert math.isclose(page.evaluate('calibration().calibratedDemShare'),45,abs_tol=1e-7)
 page.locator('#target').fill('50');page.locator('#target').press('Tab');page.select_option('#target-party','dem')
 page.locator('#benchmark-disclosure summary').click()
 with page.expect_download()as dl:page.locator('#export-benchmarks').click()
 dl.value.save_as(str(OUT/'benchmark-export.csv'))
 exported=list(csv.DictReader(io.StringIO((OUT/'benchmark-export.csv').read_text())))
 assert len(exported)==21 and all(r['estimated']=='true'if r['available']=='true'else r['two_party_2024']==''for r in exported)
 with page.expect_download()as dl:page.locator('#export-counties').click()
 dl.value.save_as(str(OUT/'county-export.csv'))
 exported=list(csv.DictReader(io.StringIO((OUT/'county-export.csv').read_text())))
 assert {r['county']for r in exported}=={'Limestone','Madison'}
 page.locator('#race-search').fill('Lands');assert page.locator('.race-choice').count()==1;page.locator('#race-search').fill('')
 page.locator('[data-star="hd10"]').click();page.select_option('#race-group','watched');assert page.locator('.race-choice').count()==1
 page.reload(wait_until='networkidle');page.select_option('#race-group','watched');assert page.locator('.race-choice').count()==1;page.select_option('#race-group','all')
 page.goto(BASE+'/#race/cd5',wait_until='networkidle');page.select_option('#map-geography','counties');assert page.locator('#map .area').count()==6
 page.screenshot(path=str(OUT/'county-fallback-desktop.png'),full_page=True)
 page.locator('nav a[href="#sources"]').click();page.get_by_role('heading',name='Sources & coverage',exact=True).wait_for();assert page.locator('.source').count()==len(data['sources']);assert 'civicAPI' in page.locator('#main').inner_text()
 page.locator('nav a[href="#methodology"]').click();page.get_by_role('heading',name='Reading the returns',exact=True).wait_for();assert 'Census' in page.locator('#main').inner_text()
 page.goto(BASE+'/#race/sd7',wait_until='networkidle');page.screenshot(path=str(OUT/'seat-benchmarks-desktop.png'),full_page=True)
 page.set_viewport_size({'width':390,'height':844});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 page.screenshot(path=str(OUT/'seat-benchmarks-mobile.png'),full_page=True)
 page.get_by_role('button',name='Refresh view').click();page.wait_for_function("!document.getElementById('refresh').disabled");assert page.locator('#error').is_hidden()
 # Synthetic positive precinct returns, intercepted only for this isolated page.
 fixture=json.loads(json.dumps(data));r=next(r for r in fixture['races']if r['id']=='hd10');r.update(total=3000,status='Preliminary results',candidates=[{'id':'a','name':'Marilyn Lands','party':'DEM','votes':1800},{'id':'b','name':'Aaron Thomas','party':'REP','votes':1200}])
 r['units']=[{'id':f'fixture:{i}','name':f'Example precinct {i:02d}','county':'Madison','county_id':'01089','administrative':False,'total':120,'candidates':[{'id':'a','name':'Marilyn Lands','party':'DEM','votes':72},{'id':'b','name':'Aaron Thomas','party':'REP','votes':48}]}for i in range(25)]
 actual=next(u for u in next(x for x in data['races']if x['id']=='hd10')['units']if not u['administrative']and bench['units'].get('hd10:precinct:'+u['name'],{}).get('available'))
 r['units'][0].update(id=actual['id'],name=actual['name']);r['county_results'][0].update(total=3000,candidates=r['candidates']);r['contributions'][0]['total']=3000
 test=b.new_page(viewport={'width':1280,'height':900});test.on('pageerror',lambda e:errors.append(str(e)));test.route('**/api/state',lambda route:route.fulfill(json=fixture));test.goto(BASE+'/#race/hd10',wait_until='networkidle')
 assert test.locator('#map .area:not([fill="#deded8"])').count()==1
 assert 'against target in 1 reported areas' in test.locator('#target-result').inner_text()
 assert test.locator('#unit-table tbody tr').count()==20;test.get_by_role('button',name='Next',exact=True).click();assert test.locator('#unit-table tbody tr').count()==5
 test.locator('#unit-search').fill('precinct 03');assert test.locator('#unit-table tbody tr').count()==1;test.locator('.unit-button').click();assert 'Example precinct 03' in test.locator('#selection').inner_text()
 with test.expect_download()as dl:test.locator('#export-race').click()
 dl.value.save_as(str(OUT/'fixture-export.csv'));assert len((OUT/'fixture-export.csv').read_text().splitlines())==51
 test.set_viewport_size({'width':390,'height':844});assert test.evaluate('document.documentElement.scrollWidth<=innerWidth')
 test.unroute('**/api/state');test.route('**/api/state',lambda route:route.abort());test.get_by_role('button',name='Refresh view').click();test.locator('#error').wait_for(state='visible');assert 'Example precinct 03' in test.locator('#unit-table').inner_text()
 # Positive county-only return joins one clipped county portion, never precincts.
 fixture=json.loads(json.dumps(data));r=next(r for r in fixture['races']if r['id']=='cd5');county=next(c for c in r['county_results']if c['county']=='Limestone');county.update(total=1000,candidates=[{'id':'d','name':'Andrew Sneed','party':'DEM','votes':600},{'id':'r','name':'Dale W. Strong','party':'REP','votes':400}]);r['total']=1000
 county_page=b.new_page();county_page.on('pageerror',lambda e:errors.append(str(e)));county_page.route('**/api/state',lambda route:route.fulfill(json=fixture));county_page.goto(BASE+'/#race/cd5',wait_until='networkidle')
 assert county_page.locator('#map .area[data-feature-id="cd5:county:01083"]').get_attribute('fill')!='#deded8'
 assert 'against target in 1 reported areas' in county_page.locator('#target-result').inner_text()
 assert all(u['county']=='Madison'for u in r['units'])
 assert not errors,errors;b.close()
print('PASS: seven clipped/mixed maps, county fallback and source preference,2024benchmarks/calibration,threeCSVexports,search/stars,pagination,isolatedpositiveprecinct/countycomparisons,desktop/mobileandnetworkfailure; no browser errors')
