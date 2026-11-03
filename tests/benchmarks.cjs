const test=require('node:test');const assert=require('node:assert/strict');const m=require('../web/benchmarks.js');
const u=(id,d,r)=>({id,available:true,democratic_votes:d,republican_votes:r,two_party_votes:d+r,total_votes:d+r});
test('calibration reaches selected whole-seat share even when extreme areas clip',()=>{const units=[u('blue',99,1),u('red',10,90)];const c=m.calibrate(units,75);assert.ok(Math.abs(c.calibratedDemShare-75)<1e-10);assert.equal(c.targets.blue,100);assert.ok(Math.abs(c.targets.red-50)<1e-10);assert.ok(Math.abs(m.calibrate(units,60,'rep').calibratedDemShare-40)<1e-10);});
test('matched-area target uses same current vote weights as actual share',()=>{const c=m.compare([{d:80,r:20,targetDem:70},{d:20,r:180,targetDem:20}]);assert.equal(c.total,300);assert.ok(Math.abs(c.targetShare-110/3)<1e-10);assert.ok(Math.abs(c.gap+10/3)<1e-10);assert.ok(Math.abs(m.compare([{d:80,r:20,targetDem:70}], 'rep').gap+10)<1e-10);});
test('county target aggregates calibrated constituent targets, not an unweighted mean',()=>{const units=[u('a',90,10),u('b',10,190)];const c=m.calibrate(units,55);const county=m.aggregate(units,'county');assert.ok(Math.abs(m.targetFor(county,c)-55)<1e-10);assert.equal(county.democratic_votes,100);});
test('unavailable benchmarks and setup zeros do not create a gap',()=>{assert.equal(m.calibrate([{id:'missing',available:false,democratic_votes:null,republican_votes:null}]).weight,0);const c=m.compare([{d:0,r:0,targetDem:50},{d:10,r:10,targetDem:null}]);assert.equal(c.gap,null);assert.equal(c.total,0);});
test('turnout compares all reported contest votes with all 2024 presidential votes only when complete',()=>{
 const reference={...u('area',40,50),other_votes:10,total_votes:100,is_estimate:true};
 const observed={total:60,reporting_complete:true,reporting_basis:'Verified complete'};
 const result=m.turnout(observed,reference);assert.equal(result.percent,60);assert.equal(result.referenceVotes,100);assert.equal(result.estimated,true);assert.equal(result.completionBasis,'Verified complete');
 assert.equal(m.turnout({...observed,total:150},reference).percent,150);
 assert.equal(m.turnout({...observed,total:0},reference).percent,0);
 assert.equal(m.turnout({...observed,reporting_complete:false},reference),null);
 assert.equal(m.turnout({total:60,estimated_reporting:100},reference),null);
 assert.equal(m.turnout({...observed,reporting_complete:null},reference),null);
 assert.equal(m.turnout({...observed,administrative:true},reference),null);
 for(const total_votes of[0,null,undefined,NaN,Infinity,-1])assert.equal(m.turnout(observed,{...reference,total_votes}),null);
 assert.equal(m.turnout(observed,{...reference,available:false}),null);
 for(const total of[null,undefined,NaN,Infinity,-1])assert.equal(m.turnout({...observed,total},reference),null);
});
test('turnout qualifies missing write-ins and never substitutes two-party votes for a missing historical total',()=>{
 const reference={...u('area',40,50),total_votes:100};
 const observed={total:60,reporting_complete:true,write_ins_supplied:false};
 assert.equal(m.turnout(observed,reference).lowerBound,true);
 assert.equal(m.turnout({...observed,write_ins_applicable:false},reference).lowerBound,false);
 assert.equal(m.turnout({...observed,write_ins_supplied:true},reference).lowerBound,false);
 const aggregated=m.aggregate([{...reference,id:'a'},u('b',5,5)],'county');assert.equal(aggregated.total_votes,110);
 const missing=m.aggregate([{...reference,total_votes:null},u('b',5,5)],'county');assert.equal(missing.total_votes,null);assert.equal(m.turnout(observed,missing),null);
});
