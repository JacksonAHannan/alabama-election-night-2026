# Current Madison election map: exact provider-name joins

The user supplied the [Madison County November 3, 2026 Clarity election](https://results.enr.clarityelections.com/AL/Madison/127483/web.345435/#/summary). Bounded research of that page's own public resources found its current precinct polygon map. No unrelated election IDs were guessed, login or challenge bypass used, or historical precinct name matches inferred.

## Discovery and source

The site's [published JavaScript](https://results.enr.clarityelections.com/AL/Madison/127483/web.345435/main.2ea4cb41df122bbb.js) implements `getKmlMap` using the election settings `kmlmap` filename and `getUri` pattern `/{state}/{county}/{election}/{version}/json/{filename}`. Saved `madison-settings.json` names `78711aa6-2e06-4898-914d-beee88029fcc.json`; election/version are the already verified 127483/382270. The resulting exact [map URL](https://results.enr.clarityelections.com/AL/Madison/127483/382270/json/78711aa6-2e06-4898-914d-beee88029fcc.json) returned HTTP 200 and **98,719 decoded bytes**.

`madison-live-map.raw.json` retains the decoded original response. `madison-live-map.discovery.json` records URLs, retrieval time, original-content SHA-256 hashes and the published JS excerpts that establish URL construction and coordinate decoding. Ordinary Python urllib requests worked for these exact resources; no Firecrawl request was made because its service had already returned HTTP 402 in the parent investigation.

## Schema and join validation

The official map is an array of 85 objects with `Name`, `Cords`, `GCords`, `NV`, `Data`, `Color`. `GCords` uses Google's encoded-polyline scheme with 1e-5-degree coordinates. The site's `OldDecode` collapses double backslashes and its `OldDecodeLine` decodes latitude before longitude. The prepared GeoJSON uses longitude before latitude, closes the rings and preserves coordinates without simplification.

The 85 map entries have **79 distinct exact Name strings**. Every one matches a name in the same election's `details.Contests[0].P` case-sensitively. The only three details names without polygons are **ABSENTEE**, **Provisional**, **PROVISIONAL ABSENTEE**. Thus the feed has 79 geographic precinct reporting names plus three administrative names, 82 total for countywide contests. The source does not provide a separate numeric precinct identifier in these map objects; the exact provider `Name` is the stable identity available for the same-election join. It is preserved verbatim as `id`, `source_id`, `name` and `unit_name` on the prepared features. No historical identity relationship is claimed.

Duplicate names represent multiple map entries: Optimist Park has four, Bobo Volunteer Fire Department two, and The Church of Jesus Christ of Latter-day Saints three. All polygon components for each exact provider Name are unioned into one reporting-name feature. Three entries have no area: Bobo entry 24 decodes to a point; LDS entries 67 and 68 decode to lines. These redundant non-area entries are documented and excluded from polygon output; each name still has a real polygon from another official entry. The decoded source has 22 invalid/degenerate entries, mostly self-intersections or repeated edges. `make_valid` repairs them for GeoJSON display, retaining polygonal components; non-area artifacts are listed in metadata. Nothing allocates votes or changes source return values.

## Deliverable and limits

`data/current-geography.json` contains **79 valid Polygon/MultiPolygon features** and records `live_boundary_match: exact_provider_map_name`. Its metadata includes the original map URL/hash, election/version, excluded administrative units and all repairs. This replaces the need to attach live returns to historical precinct polygons: join exact current provider names from this map to exact current provider return names. Keep administrative votes in their own rows and out of polygon precinct totals.

The map is a current-election provider artifact. The provider does not state its underlying survey/boundary vintage or legal precision, so this is an exact reporting-name relationship to the provider's own displayed map, not independent geographic certification. Historical swing and prior-election vote comparisons still require a separately verified crosswalk. Source map changes should be revalidated if its `kmlmap` UUID or content hash changes.

Fresh offline verification checked all 79 output geometries as valid/nonempty polygons, exact identities against retained raw map and details, the precise three excluded administrative names, unique IDs, and discovery hashes. All checks passed. The file is 587,045 bytes.

## Reproduce from retained original response

Run with `Election-Maps/.venv/bin/python`. No GIS dependency is required in the dashboard runtime.

```python
import json,hashlib,collections
from pathlib import Path
from shapely.geometry import Polygon,mapping
from shapely import make_valid
from shapely.ops import unary_union
p=Path('/home/stolas/Workspace/Projects/alabama-election-night-2026');r=p/'research';raw=(r/'madison-live-map.raw.json').read_bytes();entries=json.loads(raw);details=json.loads((r/'madison-details.json').read_text());names=set(details['Contests'][0]['P'])
def decode(s):
 s=s.replace('\\\\','\\');i=0;lat=lon=0;coords=[]
 while i<len(s):
  vals=[]
  for _ in range(2):
   shift=result=0
   while True:
    v=ord(s[i])-63;i+=1;result|=(v&31)<<shift;shift+=5
    if v<32:break
   vals.append(~(result>>1) if result&1 else result>>1)
  lat+=vals[0];lon+=vals[1];coords.append([lon/100000,lat/100000])
 if coords[0]!=coords[-1]:coords.append(coords[0])
 return coords
groups=collections.defaultdict(list);repair=[];nonarea=[]
for i,e in enumerate(entries):
 assert e['Name'] in names
 g=Polygon(decode(e['GCords']))
 if not g.is_valid:repair.append({'entry_index':i,'name':e['Name']});g=make_valid(g)
 if g.geom_type=='GeometryCollection':
  g=unary_union([part for part in g.geoms if part.geom_type in ['Polygon','MultiPolygon']])
 if g.is_empty or g.geom_type not in ['Polygon','MultiPolygon']:
  nonarea.append({'entry_index':i,'name':e['Name'],'geometry_type':g.geom_type});continue
 assert g.is_valid
 groups[e['Name']].append((i,g))
features=[]
for name,parts in groups.items():
 g=unary_union([g for i,g in parts]);assert g.is_valid and not g.is_empty
 features.append({'type':'Feature','geometry':mapping(g),'properties':{'kind':'precinct','scope':'madison','id':name,'name':name,'source_id':name,'source_unit_name':name,'unit_name':name,'county_id':'01089','county_name':'Madison','vintage':'2026-11-03 provider election map','live_boundary_match':'exact_provider_map_name','source_entry_indexes':[i for i,g in parts],'source_election_id':'127483'}})
missing=sorted(names-set(groups));assert missing==['ABSENTEE','PROVISIONAL ABSENTEE','Provisional']
meta={'schema_version':1,'source_url':'https://results.enr.clarityelections.com/AL/Madison/127483/382270/json/78711aa6-2e06-4898-914d-beee88029fcc.json','source_sha256':hashlib.sha256(raw).hexdigest(),'source_election_id':'127483','source_version':'382270','provider':'Madison County Clarity official election website','join':'Exact case-sensitive map.Name to same-election details.Contests[].P; no historical joins','source_map_entries':len(entries),'precinct_names':len(groups),'administrative_units_without_polygons':missing,'decoding':'Published Clarity OldDecode/OldDecodeLine: Google encoded polyline at 1e-5 degrees, latitude then longitude; swapped to GeoJSON longitude/latitude. Double backslashes collapsed as provider does. Closed rings. Duplicate Name entries unioned into one feature per reporting name.','geometry_repairs':repair,'non_area_map_entries':nonarea,'limitations':['Provider map is current election-specific and names match details exactly; underlying survey boundary vintage and legal precision are not provided.','Administrative ABSENTEE, Provisional, PROVISIONAL ABSENTEE totals have no precinct polygon; retain separately.','No historical precinct crosswalk, swing or vote redistribution is established.']}
out={'type':'FeatureCollection','metadata':meta,'features':features};(p/'data/current-geography.json').write_text(json.dumps(out,separators=(',',':'))+'\n');print(json.dumps({'entries':len(entries),'unique':len(features),'missing':missing,'repairs':repair,'nonarea':nonarea,'bytes':(p/'data/current-geography.json').stat().st_size},indent=2))

```
