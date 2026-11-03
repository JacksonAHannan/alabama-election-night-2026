"""Exercise exported Pages snapshots at a project subpath, or a published URL."""
import os
from playwright.sync_api import sync_playwright
BASE=os.environ.get('PAGES_TEST_URL','http://127.0.0.1:8053/site/')
errors=[];failed=[]
with sync_playwright()as p:
 b=p.chromium.launch(headless=True);page=b.new_page(viewport={'width':1512,'height':1050});page.on('pageerror',lambda e:errors.append(str(e)));page.on('response',lambda r:failed.append((r.status,r.url))if r.status>=400 else None)
 page.goto(BASE+'#race/cd2',wait_until='networkidle');page.get_by_role('heading',name='U.S. House District 2',exact=True).wait_for()
 assert page.evaluate('snapshotMode')is True
 assert len(page.evaluate('state.data.races'))==50
 for rid,areas in[('cd2',16),('cd5',6),('governor',67),('us-senate',67)]:
  page.goto(BASE+'#race/'+rid,wait_until='networkidle')
  if rid=='cd5':page.select_option('#map-geography','counties')
  assert page.locator('#map .area').count()==areas,rid
  assert page.locator('#map text').count()==0
  for _ in range(3):page.get_by_role('button',name='Zoom in',exact=True).click()
  assert page.locator('#map text').count()==0,rid
  page.locator('#map .area').first.focus();page.locator('#map .area').first.press('Enter');assert page.locator('#selection h3').inner_text()
  page.select_option('#map-layer','historical');assert page.locator('#map text').count()==0
  page.get_by_role('button',name='Reset',exact=True).click()
 assert page.evaluate("document.fonts.check('12px Caslon') && document.fonts.check('12px Franklin')")
 page.select_option('#race-coverage','madison');assert page.locator('#map text').count()==0
 page.locator('#target').fill('55');page.locator('#target').press('Tab');assert abs(page.evaluate('calibration().calibratedDemShare')-55)<1e-7
 page.get_by_role('button',name='Refresh view',exact=True).click();page.wait_for_function("!document.getElementById('refresh').disabled");assert page.locator('#error').is_hidden()
 page.locator('nav a[href="#sources"]').click();page.get_by_role('heading',name='Sources & coverage',exact=True).wait_for()
 page.goto(BASE+'#race/cd2',wait_until='networkidle');page.set_viewport_size({'width':390,'height':844});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 assert page.locator('#map text').count()==0
 # A stopped publisher must age into a stale warning even when its JSON is unchanged.
 page.evaluate("state.data.sources.filter(s=>s.fetched_at).forEach(s=>{s.fetched_at='2026-01-01T00:00:00Z';s.stale=false});ageSources();renderStatus()")
 assert 'source issue or stale data' in page.locator('#source-status').inner_text()
 assert not errors,errors;assert not failed,failed;b.close()
print('PASS: Pages subpath/assets/fonts/snapshots, county maps without name labels at every zoom, accessible inspection, target controls, refresh/sources/mobile and stale-publication warning')
