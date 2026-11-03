# Dashboard geography and historical context

Prepared 2026-11-03 from existing local source projects. Those projects were read only; no new web retrieval was needed for these cached source geometries. Parent dashboard source research must independently validate election applicability before treating any reference boundary as an authoritative current reporting boundary.

## Artifacts and schema

`data/geography.json` is a WGS84 GeoJSON FeatureCollection with 92 polygon features:

| kind | scope | count | provenance / vintage |
|---|---|---:|---|
| precinct | madison | 79 | RDH 2024 general-election all-precinct layer via national dashboard |
| county | cd5 | 6 | Census 2025 cartographic counties, 1:500,000 |
| district | cd5 | 1 | Census 2026 TIGER CD120, district 05 |
| district | hd10, hd20, hd21, hd25 | 4 | Census 2025 TIGER lower state-legislative districts |
| district | sd2, sd7 | 2 | Alabama 2025 remedial Senate plan, locally catalogued for 2026–2030 |

Every feature has `kind`, `scope`, `id`, `name`, `vintage`, `source_id`, and `live_boundary_match`. Existing precinct `unit_id`, `source_unit_id`, `unit_name`, county identifiers and source result-allocation labels remain intact. District IDs use dashboard scope keys (`cd5`, `hd10`, etc.). County IDs are five-character Census FIPS. Source filenames, original identities, SHA-256 hashes and available public URLs are embedded in the collection metadata. Source properties for the new legislative imports remain on each feature.

`data/baseline.json` holds 79 Madison 2024 presidential units. Their `unit_id` values match the historical polygon IDs exactly; all 79 joins are one-to-one, without name normalization, nearest-geography joins or vote reallocation. The source sums are Harris **87,824**, Trump **105,430**, all listed candidates **197,617**. These figures include RDH's candidate-specific allocation of centrally reported absentee/other ballots to election-day precincts. They are historical context, not live returns, actual turnout or a forecast baseline. The source's complete limitations remain in the file. No 2024 legislative district baseline was constructed.

## Source chain and important limits

The parent national dashboard imported `Election-Maps/alabama/data/geospatial/election-precincts/al-2024-gen-prec.zip` (SHA-256 `eb635ed3f4359be638fe2b8130ce9aa4d8e513849e2502813cd73ff2e69fb90e`). The archive README identifies **Alabama 2024 General Election Precinct Boundaries and Election Results**, RDH retrieval 2025-04-21, with election data from the [Alabama SOS election-data page](https://www.sos.alabama.gov/alabama-votes/voter/election-data). The README specifically identifies Madison's geometry as a shapefile downloaded from the county website, with no exact county URL or boundary release date supplied. It also describes use of older precinct sources and December 2024 voter-file checks in the statewide product. Its link to the RDH **2022** dataset describes a historical boundary input; that link must not be misrepresented as the 2024 download URL.

The national project's relevant provenance is `research/local-import-provenance.json`, `research/congressional-import-provenance.json`, and `data/state_geometry.json`. Its 2024 precinct display polygons were simplified in EPSG:5070 at 50 metres with validity repairs, and carry `live_boundary_match: unverified`. No new live reporting-unit relationship has been established here. A matching polling-place name alone does not establish 2026 geography.

The CD5 source is [Census TIGER/Line 2026, CD120](https://www2.census.gov/geo/tiger/TIGER2026/CD/tl_2026_01_cd120.zip). County outlines come from [Census cartographic counties 2025](https://www2.census.gov/geo/tiger/GENZ2025/shp/cb_2025_us_county_500k.zip). Display intersections identify Jackson, Lauderdale, Lawrence, Limestone, Madison and Morgan. **Lauderdale is partial in this CD5 geometry** (about 28.3% of its land/water display area). Countywide Lauderdale returns therefore cannot substitute for district-specific CD5 county returns. Area fractions are geometric diagnostics, not population shares, vote shares or legal boundary certification. The other five county intersections exceed 99.9%; minor differences arise from distinct source resolutions and simplification. County outlines remain whole counties and are not clipped into district fragments.

House districts use [Census TIGER 2025 SLDL](https://www2.census.gov/geo/tiger/TIGER2025/SLDL/tl_2025_01_sldl.zip), from `Election-Maps/alabama/data/geospatial/legislative-districts/tl-2025-01-sldl/`. This is a 2025 reference vintage, not independently verified election-authority certification of 2026 applicability.

Senate districts use the [American Redistricting Project's AL_SD_Remedial2025.zip](https://thearp.org/documents/19121/AL_SD_Remedial2025.zip), preserved in `Election-Maps/alabama/data/geospatial/legislative-districts/al-sldu-remedial-2025/`. Its local README says the court-ordered 2025 remedial plan replaces the previous Senate map for 2026–2030. The shape DBF itself says `ELECT_YEAR: Remedial 2025`. The archive mixes uppercase SHP filenames and a lowercase DBF basename. Importing the SHP directly on Linux silently omits district attributes; the preparation below copies the components to a temporary common basename before reading them. Original source files were untouched. Older Senate layers were deliberately not substituted.

The separate local `outputs/alabama-2026-custom-precincts` layer was inspected and excluded: its own README describes an inferred analytical derivative of 2024 polygons, constructed with historical name crosswalks and absorption of old polygons. It is not an official 2026 precinct-boundary release and cannot establish exact live map joins.

## Transformations and verification

The existing national Madison precinct, county and CD5 polygons were selected without new spatial vote assignments. House/Senate shapes were transformed to EPSG:5070, repaired only if invalid, simplified at 30 metres with topology preservation, and converted to EPSG:4326. Output coordinates were rounded to six decimals only where validity survived; otherwise original precision was kept. These are display operations and never allocate votes or infer district precinct membership.

Fresh validation checked all 92 geometries as nonempty, valid Polygon/MultiPolygon; unique feature IDs; the exact expected four HD and two SD scope keys; all six county FIPS; one-to-one equality of 79 baseline IDs and 79 precinct IDs; equality of preserved baseline values with the national source; preservation of all copied source-file hashes. All checks passed. The resulting geography is approximately 208 KB and baseline approximately 44 KB.

## Reproduction

Run the following with `Election-Maps/.venv/bin/python`; it uses installed Pyogrio, Shapely and PyProj directly, no runtime GIS dependency. Source project paths are explicit. The input source hashes are stored in the resulting GeoJSON and baseline metadata.

```python
from pathlib import Path
import json,hashlib,tempfile,shutil
import pyogrio
from shapely import from_wkb,make_valid
from shapely.geometry import shape,mapping
from shapely.ops import transform
from pyproj import Transformer
root=Path('/home/stolas/Workspace/Projects'); national=root/'national-election-night-2026'; maps=root/'Election-Maps'; out=root/'alabama-election-night-2026'
sources={}
def load(label,p):
 sources[label]={'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
 return json.loads(p.read_text())
features=[]
def add(geom,props):
 g=shape(geom)
 if not g.is_valid:g=make_valid(g)
 assert not g.is_empty and g.is_valid and g.geom_type in ['Polygon','MultiPolygon']
 features.append({'type':'Feature','geometry':mapping(g),'properties':props})
prec=load('precincts',national/'data/geometry/local/AL-precincts-2024.geojson')
for f in prec['features']:
 p=f['properties']
 if p['county_id']=='01089':add(f['geometry'],{**p,'kind':'precinct','scope':'madison','id':p['unit_id'],'name':p['unit_name'],'source_id':p['source_unit_id'],'geography_label':'2024 historical precinct reference'})
districts=load('congressional',national/'data/geometry/local/AL-districts-2026.geojson')
cd=next(f for f in districts['features'] if f['properties']['district']=='05')
add(cd['geometry'],{**cd['properties'],'kind':'district','scope':'cd5','id':'cd5','name':'Alabama CD-5','source_id':cd['properties']['source_geoid'],'live_boundary_match':'unverified','geography_label':'2026 Census CD120 reference'})
county_data=load('counties',national/'data/geometry/states/AL.geojson')
project=Transformer.from_crs(4326,5070,always_xy=True).transform
cd_projected=transform(project,shape(cd['geometry']))
for f in county_data['features']:
 p=f['properties'];g=transform(project,shape(f['geometry']));fraction=g.intersection(cd_projected).area/g.area
 if fraction>.001:
  add(f['geometry'],{**p,'kind':'county','scope':'cd5','id':p['GEOID'],'name':p['NAME'],'source_id':p['GEOID'],'county_id':p['GEOID'],'county_name':p['NAME'],'vintage':2025,'cd5_coverage':'partial' if fraction<.99 else 'whole','cd5_display_area_fraction':round(fraction,6),'live_boundary_match':'unverified','geography_label':'County reference outline'})

def shapefile(label,p,codefield,numbers,prefix,vintage):
 meta,_,geoms,fields=pyogrio.raw.read(p)
 sources[label]={'files':[{'path':str(c),'sha256':hashlib.sha256(c.read_bytes()).hexdigest()} for c in sorted(p.parent.glob(p.stem+'.*')) if c.suffix.lower() in ['.shp','.shx','.dbf','.prj','.cpg']]}
 project_in=Transformer.from_crs(meta['crs'],5070,always_xy=True).transform
 project_out=Transformer.from_crs(5070,4326,always_xy=True).transform
 for i,wkb in enumerate(geoms):
  props={str(k):str(v[i]) for k,v in zip(meta['fields'],fields)}
  n=int(props[codefield])
  if n not in numbers:continue
  g=transform(project_in,from_wkb(wkb));g=make_valid(g) if not g.is_valid else g
  g=g.simplify(30,preserve_topology=True);g=transform(project_out,g)
  add(mapping(g),{'kind':'district','scope':prefix+str(n),'id':prefix+str(n),'name':'Alabama '+prefix.upper()+'-'+str(n),'district':n,'source_id':props.get('GEOID',props[codefield]),'vintage':vintage,'live_boundary_match':'unverified','source_properties':props,'geography_label':str(vintage)+' boundary reference'})
leg=maps/'alabama/data/geospatial/legislative-districts'
shapefile('house',leg/'tl-2025-01-sldl/tl-2025-01-sldl.shp','SLDLST',{10,20,21,25},'hd',2025)
senate=leg/'al-sldu-remedial-2025'
with tempfile.TemporaryDirectory() as temp:
 p=Path(temp)/'senate.shp'
 for suffix in ['shp','shx','prj','cpg']:shutil.copy(senate/f'AL_SD_Remedial2025.{suffix}',p.with_suffix('.'+suffix))
 shutil.copy(senate/'al_sd_remedial2025.dbf',p.with_suffix('.dbf'))
 shapefile('senate',p,'DISTRICT',{2,7},'sd','2025 remedial')
 sources['senate']={'files':[{'path':str(c),'sha256':hashlib.sha256(c.read_bytes()).hexdigest()} for c in sorted(senate.iterdir()) if c.suffix.lower() in ['.shp','.shx','.dbf','.prj','.cpg']]}
sources['counties']['source_url']='https://www2.census.gov/geo/tiger/GENZ2025/shp/cb_2025_us_county_500k.zip'
sources['congressional']['source_url']='https://www2.census.gov/geo/tiger/TIGER2026/CD/tl_2026_01_cd120.zip'
sources['house']['source_url']='https://www2.census.gov/geo/tiger/TIGER2025/SLDL/tl_2025_01_sldl.zip'
sources['senate']['source_url']='https://thearp.org/documents/19121/AL_SD_Remedial2025.zip'
sources['precincts']['source_archive_sha256']='eb635ed3f4359be638fe2b8130ce9aa4d8e513849e2502813cd73ff2e69fb90e'
sources['precincts']['provider']='Redistricting Data Hub, Alabama 2024 general-election all-precinct layer; local Election-Maps copy'
# Decimal rounding reduces delivery bytes; no vote or spatial joins use rounded display polygons.
def round_tree(x):
 if isinstance(x,float):return round(x,6)
 if isinstance(x,(list,tuple)):return [round_tree(v) for v in x]
 if isinstance(x,dict):return {k:round_tree(v) for k,v in x.items()}
 return x
for f in features:
 rounded=round_tree(f['geometry'])
 if shape(rounded).is_valid:f['geometry']=rounded
 assert shape(f['geometry']).is_valid
geo={'type':'FeatureCollection','metadata':{'schema_version':1,'generated_on':'2026-11-03','use':'Reference polygons; historical precinct boundaries are not verified 2026 live reporting boundaries','sources':sources,'simplification':'National app geometry retained; House/Senate topology-preserving 30 metre display simplification in EPSG:5070; WGS84 coordinates rounded to six decimals when validity is preserved, otherwise original precision retained'},'features':features}
(out/'data/geography.json').write_text(json.dumps(geo,separators=(',',':'))+'\n')
history=load('historical_returns',national/'data/historical/AL-2024.json');units=[u for u in history['units'] if u['county_id']=='01089']
assert {u['unit_id'] for u in units}=={f['properties']['unit_id'] for f in features if f['properties']['kind']=='precinct'}
keys=['unit_id','unit_name','source_unit_id','county_id','county_name','unit_vintage','democratic_votes','republican_votes','other_votes','total_votes','two_party_votes','democratic_margin','candidate_votes','results_basis','results_complete']
base={'schema_version':1,'year':2024,'contest':'president','label':'2024 presidential historical context','source':sources['historical_returns'],'results_basis':history['results_basis'],'margin_unit':history['margin_unit'],'margin_denominator':history['margin_denominator'],'limitations':history['limitations'],'units':[{k:u[k] for k in keys} for u in units]}
(out/'data/baseline.json').write_text(json.dumps(base,separators=(',',':'))+'\n')
print(json.dumps({'features':len(features),'kinds':{kind:sum(f['properties']['kind']==kind for f in features) for kind in ['precinct','county','district']},'baseline_units':len(units),'geography_bytes':(out/'data/geography.json').stat().st_size,'baseline_bytes':(out/'data/baseline.json').stat().st_size,'baseline_D':sum(u['democratic_votes'] for u in units),'baseline_R':sum(u['republican_votes'] for u in units),'baseline_total':sum(u['total_votes'] for u in units)},indent=2))

```
