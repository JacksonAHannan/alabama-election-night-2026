const test=require('node:test');
const assert=require('node:assert/strict');
const {changes}=require('../web/result-alerts.js');
const race=(total=100)=>({id:'cd5',name:'U.S. House District 5',total,candidates:[{name:'Example A',votes:60},{name:'Example B',votes:total-60}],units:[],county_results:[]});
const snapshot=r=>({prepared_at:'2026-11-04T01:00:00Z',races:[r]});
test('statewide publisher updates alert without changing underlying county observations',()=>{
 const before=snapshot({...race(),total:0,candidates:[]});
 const after=structuredClone(before);after.races[0].statewide_report={total:10506,candidates:[{name:'Barry Moore',votes:5539},{name:'Everett Wess',votes:4967}]};
 assert.equal(changes(before,after)[0].after,10506);
 const next=structuredClone(after);next.races[0].statewide_report.fetched_at='2026-11-04T01:02:00Z';assert.deepEqual(changes(after,next),[]);
 next.races[0].statewide_report.total+=10;next.races[0].statewide_report.candidates[0].votes+=10;
 assert.equal(changes(after,next)[0].delta,10);assert.equal(after.races[0].total,0);
});
test('initial load, repeated snapshots, timestamps, ordering and setup zeros are quiet',()=>{
 const r=race();const before=snapshot(r);const after=structuredClone(before);
 after.prepared_at='2026-11-04T01:02:00Z';after.races[0].fetched_at=after.prepared_at;after.races[0].candidates.reverse();
 assert.deepEqual(changes(null,before),[]);assert.deepEqual(changes(before,before),[]);assert.deepEqual(changes(before,after),[]);
 const zero={...race(0),candidates:[]};
 assert.deepEqual(changes(snapshot(zero),snapshot({...zero,candidates:[{name:'Example A',votes:0}]})),[]);
 assert.deepEqual(changes(snapshot(race()),{races:[]}),[]);
});
test('new results, increases and corrections include meaningful deltas',()=>{
 const before=snapshot(race()),after=snapshot(race(125));
 assert.deepEqual(changes(before,after),[{id:'cd5',name:'U.S. House District 5',before:100,after:125,delta:25,kind:'votes'}]);
 assert.equal(changes(after,before)[0].kind,'correction');assert.equal(changes(after,before)[0].delta,-25);
 const revised=race();revised.candidates[0].votes=55;revised.candidates[1].votes=45;
 assert.equal(changes(before,snapshot(revised))[0].kind,'correction');assert.equal(changes(before,snapshot(revised))[0].delta,0);
 assert.equal(changes({races:[]},after)[0].kind,'new');
 assert.equal(changes(snapshot({...race(),total:null,candidates:[]}),after)[0].kind,'new');
 assert.deepEqual(changes(before,snapshot({...race(),total:null,candidates:[]})),[]);
 assert.deepEqual(before,snapshot(race()),'Comparison must not mutate cached data');
});
test('area vote changes alert even when race totals are unchanged; reporting metadata does not',()=>{
 const r=race();r.units=[{name:'Example precinct',county:'Madison',total:10,candidates:[{name:'Example A',votes:10}],reporting_complete:false}];
 const before=snapshot(r),after=structuredClone(before);after.races[0].units[0].reporting_complete=true;
 assert.deepEqual(changes(before,after),[]);
 after.races[0].units[0].total=20;after.races[0].units[0].candidates[0].votes=20;
 assert.equal(changes(before,after)[0].kind,'detail');
 const countyBefore=structuredClone(before),countyAfter=structuredClone(before);
 countyBefore.races[0].county_results=[{county_id:'01089',total:20,candidates:[{name:'Example A',votes:20}]}];
 countyAfter.races[0].county_results=[{county_id:'01089',total:15,candidates:[{name:'Example A',votes:15}]}];
 assert.equal(changes(countyBefore,countyAfter)[0].kind,'correction');
});
test('source replacement qualifies a change instead of inventing a vote correction',()=>{
 const before=race(),after=race(80);
 before.county_results=[{county_id:'01089',source_id:'madison',total:100}];
 after.county_results=[{county_id:'01089',source_id:'civic-cd5',total:80}];
 assert.equal(changes(snapshot(before),snapshot(after))[0].kind,'coverage');
});
