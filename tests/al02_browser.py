"""Current AL-02 geography, county observations and precinct research; fixtures stay in browser."""
import copy,csv,io,json,math
from pathlib import Path
from urllib.request import urlopen
from playwright.sync_api import sync_playwright
BASE='http://127.0.0.1:8052';OUT=Path('runtime/qa');OUT.mkdir(parents=True,exist_ok=True)
data=json.load(urlopen(BASE+'/api/state'));r=next(r for r in data['races']if r['id']=='cd2')
expected={'Barbour','Bullock','Butler','Coffee','Covington','Crenshaw','Dale','Elmore','Geneva','Henry','Houston','Lowndes','Macon','Montgomery','Pike','Russell'}
assert len(data['races'])==50 and {c['county']for c in r['county_results']}==expected
assert not r['units'] and r['expected_county_count']==16
assert {c['name']for c in r['candidates']}=={'Shomari C. Figures','Rhett Marques'}
geo=json.load(open('data/al02-geography.json'));bench=json.load(open('data/al02-benchmarks.json'));errors=[]
with sync_playwright()as p:
 b=p.chromium.launch(headless=True);page=b.new_page(viewport={'width':1512,'height':1050});page.on('pageerror',lambda e:errors.append(str(e)))
 page.goto(BASE+'/#race/cd2',wait_until='networkidle');page.get_by_role('heading',name='U.S. House District 2',exact=True).wait_for()
 assert page.locator('#map .area').count()==16 and page.locator('#seat-clip').count()==1
 assert {f['properties']['id']for f in geo['features']}==set(page.locator('#map .area').evaluate_all('(els)=>els.map(e=>e.dataset.featureId)'))
 assert page.locator('#map-geography').input_value()=='counties'
 assert 'current AL-02' in page.locator('#map-note').inner_text()
 assert len(page.evaluate('benchmarkRecords()'))==16 and page.evaluate('benchmarkScope()')=='cd2'
 assert math.isclose(page.evaluate('calibration().calibratedDemShare'),50,abs_tol=1e-7)
 assert page.locator('#county-table tbody tr').count()==16
 page.select_option('#race-group','Federal');assert page.locator('.race-choice').count()==3;page.select_option('#race-group','all')
 page.locator('#race-search').fill('Shomari');assert page.locator('.race-choice').count()==1;page.locator('#race-search').fill('')
 page.select_option('#map-layer','historical');assert page.locator('#map .area:not([fill="#deded8"])').count()==16
 elmore=page.locator('#map .area[data-feature-id="cd2:county:01051"]');elmore.focus();elmore.press('Enter');assert '2024 presidential reference' in page.locator('#selection').inner_text()
 page.locator('#target').fill('55');page.locator('#target').press('Tab');assert math.isclose(page.evaluate('calibration().calibratedDemShare'),55,abs_tol=1e-7)
 page.select_option('#target-party','rep');assert math.isclose(page.evaluate('calibration().calibratedDemShare'),45,abs_tol=1e-7)
 page.locator('#county-search').fill('Elmore');assert page.locator('#county-table tbody tr').count()==1
 with page.expect_download()as dl:page.locator('#export-counties').click()
 dl.value.save_as(str(OUT/'al02-county-export.csv'));rows=list(csv.DictReader(io.StringIO((OUT/'al02-county-export.csv').read_text())))
 assert {r['county']for r in rows}==expected and len(rows)==32
 assert any(r['candidate']=='Shomari C. Figures'and r['source_candidate_name']=='Shomari Figures'for r in rows)
 page.locator('#county-search').fill('');page.locator('#benchmark-disclosure summary').click()
 with page.expect_download()as dl:page.locator('#export-benchmarks').click()
 dl.value.save_as(str(OUT/'al02-benchmark-export.csv'));rows=list(csv.DictReader(io.StringIO((OUT/'al02-benchmark-export.csv').read_text())))
 assert len(rows)==16 and all(r['scope']=='cd2'and r['available']=='true'and r['estimated']=='true'for r in rows)
 assert math.isclose(sum(float(r['democratic_2024'])for r in rows),sum(r['democratic_votes']for r in bench['units'].values()),abs_tol=1e-6)
 checks=data['al02_research']['counties'];assert {c['county']for c in checks}==expected and len(checks)==16
 heading=page.get_by_role('heading',name='AL-02 precinct feed coverage',exact=True);assert heading.count()==1
 assert heading.locator('..').locator('.county').count()==16
 page.evaluate('window.scrollTo(0,0)');page.screenshot(path=str(OUT/'al02-desktop.png'),full_page=False)
 page.set_viewport_size({'width':390,'height':844});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 page.locator('#map').scroll_into_view_if_needed();page.screenshot(path=str(OUT/'al02-mobile.png'),full_page=False)
 # A county that later supplies precinct detail still enters county target comparisons once.
 fixture=copy.deepcopy(data);r=next(r for r in fixture['races']if r['id']=='cd2');c=next(c for c in r['county_results']if c['county']=='Elmore');c.update(total=100,precinct_detail=True,granularity='precinct',candidates=[{'name':'Shomari C. Figures','party':'DEM','votes':60},{'name':'Rhett Marques','party':'REP','votes':40}]);r['contributions']=copy.deepcopy(r['county_results']);r.update(total=100,candidates=c['candidates'],units=[{'id':'fixture:unit:elmore','name':'Example Elmore reporting unit','county':'Elmore','county_id':'01051','administrative':False,'total':100,'candidates':c['candidates']}])
 positive=b.new_page();positive.on('pageerror',lambda e:errors.append(str(e)));positive.route('**/api/state',lambda route:route.fulfill(json=fixture));positive.goto(BASE+'/#race/cd2',wait_until='networkidle')
 assert '1 reported areas' in positive.locator('#target-result').inner_text()
 assert 'Example Elmore reporting unit' in positive.locator('#unit-table').inner_text()
 assert positive.locator('#map .area:not([fill="#deded8"])').count()==1
 gap=positive.evaluate('60-targetFor(benchmarkForCounty(race().county_results.find(c=>c.county==="Elmore")))')
 assert f'{gap:+.1f} pp' in positive.locator('#target-result').inner_text()
 positive.select_option('#map-layer','target');assert positive.locator('#map .area:not([fill="#deded8"])').count()==1
 positive.close();assert not errors,errors;b.close()
print('PASS: AL-02 16 counties/current boundary, source alias, modeled2024/targets, search/filter, county and benchmark exports, 16 official-source checks, positive county/precinct deduplication, desktop/mobile; no browser errors')
