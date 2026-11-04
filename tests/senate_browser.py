"""Verify statewide fallback visibility without allocating its votes to county data."""
from copy import deepcopy
import json
import os
from urllib.request import urlopen
from playwright.sync_api import sync_playwright

BASE=os.environ.get('PAGES_TEST_URL','http://127.0.0.1:8053/site/')
snapshot=json.load(urlopen(BASE+'api/state.json'))
payload=deepcopy(snapshot)
senate=next(r for r in payload['races']if r['id']=='us-senate')
report=senate['statewide_report'];report['preferred']=True
county_total=senate['total'];county_rows=deepcopy(senate['county_results'])
with sync_playwright()as p:
 b=p.chromium.launch(headless=True);page=b.new_page(viewport={'width':1440,'height':1000});errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 page.route('**/api/state.json',lambda route:route.fulfill(json=payload))
 page.goto(BASE+'#race/us-senate',wait_until='networkidle')
 assert page.get_by_role('heading',name='Statewide Senate returns',exact=True).is_visible()
 assert 'no county breakdown'in page.locator('.statewide-report').inner_text()
 assert 'WAFF / Gray'in page.locator('.statewide-report').inner_text()
 assert page.evaluate('race().total')==county_total
 assert page.evaluate('race().county_results')==county_rows
 assert page.locator('#map .area').count()==67
 page.select_option('#race-coverage','madison');assert page.locator('.statewide-report').count()==0
 page.select_option('#race-coverage','alabama');assert page.locator('.statewide-report').count()==1
 page.screenshot(path='runtime/qa/senate-fallback-desktop.png')
 page.set_viewport_size({'width':390,'height':844});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 page.screenshot(path='runtime/qa/senate-fallback-mobile.png')
 report['total']+=1;report['candidates'][0]['votes']+=1
 page.get_by_role('button',name='Refresh view',exact=True).click();page.wait_for_function('!document.getElementById("refresh").disabled')
 assert '+1 reported votes'in page.locator('#updates-list').inner_text()
 report['preferred']=False
 page.get_by_role('button',name='Refresh view',exact=True).click();page.wait_for_function('!document.getElementById("refresh").disabled')
 assert page.locator('.statewide-report').count()==0
 assert page.evaluate('race().total')==county_total
 assert not errors,errors;b.close()
print('PASS: labeled Senate statewide fallback, no county allocation, Madison exclusion, report alerts, healthy-county preference and mobile layout')
