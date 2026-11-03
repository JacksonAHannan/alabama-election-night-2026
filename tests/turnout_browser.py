"""Completion-gated 2024 vote-volume comparisons; all positive data is browser-local."""
import copy,csv,io,json
from pathlib import Path
from urllib.request import urlopen
from playwright.sync_api import sync_playwright
BASE='http://127.0.0.1:8052';OUT=Path('runtime/qa');OUT.mkdir(parents=True,exist_ok=True)
data=json.load(urlopen(BASE+'/api/state'));seat=json.load(open('data/seat-benchmarks.json'));statebench=json.load(open('data/statewide-benchmarks.json'));cd2bench=json.load(open('data/al02-benchmarks.json'))
def reference(record,total=1000):
 record.update(available=True,is_estimate=True,democratic_votes=total*.4,republican_votes=total*.5,other_votes=total*.1,two_party_votes=total*.9,total_votes=total)
def county(r,name,total,complete,basis):
 c=next(c for c in r['county_results']if c['county']==name);c.update(total=total,reporting_complete=complete,reporting_basis=basis,candidates=[{'name':r['candidates'][0]['name'],'party':'DEM','votes':total//2},{'name':r['candidates'][-1]['name'],'party':'REP','votes':total-total//2}]);return c
fixture=copy.deepcopy(data);gov=next(r for r in fixture['races']if r['id']=='governor')
county(gov,'Autauga',800,True,'civicAPI estimated reporting').update(write_ins_supplied=False,estimated_reporting=100)
county(gov,'Baldwin',600,False,'civicAPI estimated reporting').update(estimated_reporting=99.9)
county(gov,'Barbour',800,None,None).update(estimated_reporting=100)
county(gov,'Madison',1200,True,'Official geographic precinct statuses; administrative totals may change').update(write_ins_supplied=True)
for cid in['01001','01089']:reference(statebench['units']['alabama:county:'+cid])
gov['contributions']=copy.deepcopy(gov['county_results'])
hd=next(r for r in fixture['races']if r['id']=='hd10');originals=[u for u in hd['units']if not u['administrative']and seat['units'].get('hd10:precinct:'+u['name'],{}).get('available')]
for u,total,complete in zip(originals[:3],[750,600,800],[True,False,None]):
 u.update(total=total,reporting_complete=complete,reporting_basis='Official Clarity precinct status '+('4'if complete else'2'),candidates=[{'name':'Marilyn Lands','party':'DEM','votes':total//2},{'name':'Aaron Thomas','party':'REP','votes':total-total//2}]);reference(seat['units']['hd10:precinct:'+u['name']])
admin=next(u for u in hd['units']if u['administrative']);admin.update(reporting_complete=True,total=200)
missing=copy.deepcopy(originals[0]);missing.update(id='fixture:unknown',name='Unknown historical area')
hd['units']=[originals[0],originals[1],originals[2],admin,missing];hd['total']=sum(u['total']for u in hd['units'])
al=next(r for r in fixture['races']if r['id']=='cd2');county(al,'Elmore',1000,True,'civicAPI estimated reporting').update(estimated_reporting=100,write_ins_supplied=False);reference(cd2bench['units']['cd2:county:01051'],2500);al['contributions']=copy.deepcopy(al['county_results'])
errors=[]
with sync_playwright()as p:
 b=p.chromium.launch(headless=True);page=b.new_page(viewport={'width':1512,'height':1050});page.on('pageerror',lambda e:errors.append(str(e)))
 page.route('**/api/state',lambda route:route.fulfill(json=fixture))
 def reply(payload):return lambda route:route.fulfill(json=payload)
 for path,payload in[('seat-benchmarks.json',seat),('statewide-benchmarks.json',statebench),('al02-benchmarks.json',cd2bench)]:page.route('**/data/'+path,reply(payload))
 page.goto(BASE+'/#race/governor',wait_until='networkidle')
 def crow(name):return page.locator('#county-table tbody tr').filter(has=page.locator('td[data-label="County"]',has_text=name)).first
 assert crow('Autauga').locator('[data-label="Turnout vs 2024"]').inner_text().startswith('≥ 80.0%')
 assert 'civicAPI estimated reporting' in crow('Autauga').inner_text()
 assert crow('Madison').locator('[data-label="Turnout vs 2024"]').inner_text().startswith('≈ 120.0%')
 assert crow('Baldwin').locator('[data-label="Turnout vs 2024"]').inner_text()=='—'
 assert crow('Barbour').locator('[data-label="Turnout vs 2024"]').inner_text()=='—'
 page.locator('#map .area[data-feature-id="alabama:county:01001"]').focus();page.locator('#map .area[data-feature-id="alabama:county:01001"]').press('Enter');assert 'Turnout vs 2024: ≥ 80.0%' in page.locator('#selection').inner_text()
 page.select_option('#target-party','rep');assert crow('Autauga').locator('[data-label="Turnout vs 2024"]').inner_text().startswith('≥ 80.0%')
 with page.expect_download()as dl:page.locator('#export-counties').click()
 dl.value.save_as(str(OUT/'turnout-county-export.csv'));export=list(csv.DictReader(io.StringIO((OUT/'turnout-county-export.csv').read_text())))
 assert all(float(r['turnout_pct_of_2024'])==80 and r['turnout_lower_bound']=='true'and float(r['presidential_votes_2024'])==1000 for r in export if r['county']=='Autauga')
 assert all(r['turnout_pct_of_2024']==''for r in export if r['county']in['Baldwin','Barbour'])
 assert all(r['historical_estimate']=='true'for r in export if r['county']in['Baldwin','Barbour'])
 page.locator('#county-search').fill('Autauga');page.locator('#county-section').scroll_into_view_if_needed();page.screenshot(path=str(OUT/'turnout-county.png'),full_page=False)
 page.goto(BASE+'/#race/hd10',wait_until='networkidle')
 def urow(name):return page.locator('#unit-table tbody tr').filter(has=page.get_by_role('button',name=name,exact=True)).first
 assert urow(originals[0]['name']).locator('[data-label="Turnout vs 2024"]').inner_text().startswith('≈ 75.0%')
 for u in[originals[1],originals[2],admin,missing]:assert urow(u['name']).locator('[data-label="Turnout vs 2024"]').inner_text()=='—'
 page.get_by_role('button',name=originals[0]['name'],exact=True).click();assert 'Turnout vs 2024: ≈ 75.0%' in page.locator('#selection').inner_text()
 with page.expect_download()as dl:page.locator('#export-race').click()
 dl.value.save_as(str(OUT/'turnout-precinct-export.csv'));export=list(csv.DictReader(io.StringIO((OUT/'turnout-precinct-export.csv').read_text())))
 assert all(float(r['turnout_pct_of_2024'])==75 and r['reporting_complete']=='true'for r in export if r['unit']==originals[0]['name'])
 assert all(r['turnout_pct_of_2024']==''for r in export if r['unit']!=originals[0]['name'])
 page.set_viewport_size({'width':390,'height':844});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 page.locator('#unit-table').scroll_into_view_if_needed();page.screenshot(path=str(OUT/'turnout-precinct-mobile.png'),full_page=False)
 page.goto(BASE+'/#race/cd2',wait_until='networkidle');assert crow('Elmore').locator('[data-label="Turnout vs 2024"]').inner_text().startswith('≥ 40.0%')
 assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 # Current real setup data must not be labeled completed merely from zeros or positive fixtures.
 real=b.new_page();real.on('pageerror',lambda e:errors.append(str(e)));real.goto(BASE+'/#race/hd10',wait_until='networkidle')
 if not any(u.get('reporting_complete')for u in next(r for r in data['races']if r['id']=='hd10')['units']):assert all(x=='—'for x in real.locator('#unit-table [data-label="Turnout vs 2024"]').all_inner_texts())
 assert not errors,errors;b.close()
print('PASS: completed county/precinct turnout uses total2024votes; incomplete/unknown/admin/missing withheld; values above100%, write-in lower bounds, splitcounty references, selection/details,exports and mobile; no browser errors')
