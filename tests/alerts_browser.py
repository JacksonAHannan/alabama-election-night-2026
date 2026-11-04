"""Result alerts use browser-only fixtures; production observations remain untouched."""
from copy import deepcopy
import json
import os
from urllib.request import urlopen
from playwright.sync_api import sync_playwright

BASE = os.environ.get('PAGES_TEST_URL', 'http://127.0.0.1:8053/site/')
snapshot = json.load(urlopen(BASE + 'api/state.json'))
payload = deepcopy(snapshot)


def adjust(first, second=0):
    race = next(r for r in payload['races'] if r['id'] == 'cd5')
    county = next(c for c in race['county_results'] if c['county'] == 'Madison')
    unit = next(u for u in race['units'] if not u['administrative'])
    for row in [race, county, unit]:
        row['candidates'][0]['votes'] += first
        row['candidates'][1]['votes'] += second
        row['total'] += first + second


with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page(viewport={'width': 1440, 'height': 1000})
    errors = []
    page.on('pageerror', lambda error: errors.append(str(error)))
    page.route('**/api/state.json', lambda route: route.fulfill(json=payload))
    page.goto(BASE + '#race/cd5', wait_until='networkidle')
    assert page.locator('#refresh-interval').inner_text() == 'Auto-refresh: 30s'
    assert 'Watching for vote changes' in page.locator('#updates-status').inner_text()
    assert page.locator('#dismiss-updates').is_hidden()

    def refresh():
        page.get_by_role('button', name='Refresh view', exact=True).click()
        page.wait_for_function('!document.getElementById("refresh").disabled')

    payload['prepared_at'] = '2026-11-04T01:00:01Z'
    refresh()
    assert page.locator('#updates-details').is_hidden(), 'Timestamp changes must remain quiet'
    adjust(10)
    refresh()
    assert '1 race updated' in page.locator('#updates-status').inner_text()
    assert '+10 reported votes' in page.locator('#updates-list').inner_text()
    assert page.locator('#updates-status').get_attribute('aria-live') == 'polite'
    assert page.locator('#updates-details').get_attribute('open') is not None
    assert page.locator('#updates-list li').count() == 1
    refresh()
    assert page.locator('#updates-list li').count() == 1, 'Do not repeat the same alert'
    adjust(-1, 1)
    refresh()
    assert 'Vote correction' in page.locator('#updates-list li').first.inner_text()
    assert 'Race total unchanged' in page.locator('#updates-list li').first.inner_text()
    page.locator('#updates-list a').first.click()
    assert page.url.endswith('#race/cd5')
    page.evaluate('window.scrollTo(0,1800)')
    assert abs(page.locator('#result-updates').bounding_box()['y']) < 2
    page.evaluate('window.scrollTo(0,0)')
    page.screenshot(path='runtime/qa/results-alerts-desktop.png')
    page.set_viewport_size({'width': 390, 'height': 844})
    assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
    page.screenshot(path='runtime/qa/results-alerts-mobile.png')
    page.get_by_role('button', name='Dismiss alerts', exact=True).click()
    assert page.locator('#updates-details').is_hidden()
    refresh()
    assert page.locator('#updates-details').is_hidden(), 'Dismissed changes must not reappear'
    adjust(5)
    refresh()
    assert '+5 reported votes' in page.locator('#updates-list').inner_text()
    assert not errors, errors
    browser.close()
print('PASS: quiet first load/polls, changed votes, same-total corrections, linked races, dismiss/re-alert, accessible live display and mobile layout')
