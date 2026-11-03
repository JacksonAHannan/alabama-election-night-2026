/* Pure calculations over explicitly modeled 2024 reference counts. */
(function(root,factory){const api=factory();if(typeof module==='object'&&module.exports)module.exports=api;else root.ElectionBenchmarks=api;})(typeof globalThis!=='undefined'?globalThis:this,()=>{
 'use strict';
 const usable=u=>u&&u.available!==false&&Number.isFinite(u.democratic_votes)&&Number.isFinite(u.republican_votes)&&u.democratic_votes>=0&&u.republican_votes>=0&&u.democratic_votes+u.republican_votes>0;
 const weight=u=>u.democratic_votes+u.republican_votes;
 const clamp=x=>Math.max(0,Math.min(1,x));
 function calibrate(records,target=50,party='dem'){
  if(!Number.isFinite(target)||target<=0||target>=100)throw Error('Target must be between 0 and 100');
  const units=records.filter(usable),total=units.reduce((n,u)=>n+weight(u),0);
  if(!total)return {targets:{},referenceDemShare:null,calibratedDemShare:null,weight:0,shift:null};
  const desired=(party==='rep'?100-target:target)/100;
  const share=shift=>units.reduce((n,u)=>n+weight(u)*clamp(u.democratic_votes/weight(u)+shift),0)/total;
  let lo=-1,hi=1;for(let i=0;i<80;i++){const mid=(lo+hi)/2;if(share(mid)<desired)lo=mid;else hi=mid;}
  const shift=(lo+hi)/2,targets={};for(const u of units)targets[u.id]=100*clamp(u.democratic_votes/weight(u)+shift);
  return {targets,shift,weight:total,referenceDemShare:100*units.reduce((n,u)=>n+u.democratic_votes,0)/total,calibratedDemShare:100*share(shift)};
 }
 function aggregate(records,id){const units=records.filter(usable);if(!units.length)return null;return {id,available:true,is_estimate:true,children:units,democratic_votes:units.reduce((n,u)=>n+u.democratic_votes,0),republican_votes:units.reduce((n,u)=>n+u.republican_votes,0),other_votes:units.reduce((n,u)=>n+(u.other_votes||0),0),two_party_votes:units.reduce((n,u)=>n+weight(u),0),total_votes:units.every(u=>Number.isFinite(u.total_votes)&&u.total_votes>=0)?units.reduce((n,u)=>n+u.total_votes,0):null};}
 function targetFor(record,calibration){if(!usable(record))return null;if(record.children){const total=record.children.reduce((n,u)=>n+weight(u),0);return record.children.reduce((n,u)=>n+weight(u)*calibration.targets[u.id],0)/total;}return calibration.targets[record.id]??null;}
 function compare(observations,party='dem'){
  let actual=0,expected=0,total=0,count=0;
  for(const o of observations){if(!Number.isFinite(o.d)||!Number.isFinite(o.r)||o.d<0||o.r<0||o.d+o.r<=0||!Number.isFinite(o.targetDem)||o.targetDem<0||o.targetDem>100)continue;const n=o.d+o.r;total+=n;actual+=party==='rep'?o.r:o.d;expected+=n*(party==='rep'?100-o.targetDem:o.targetDem)/100;count++;}
  return {count,total,actualShare:total?100*actual/total:null,targetShare:total?100*expected/total:null,gap:total?100*(actual-expected)/total:null};
 }
 // A reporting flag is required; positive votes and race calls do not establish completion.
 function turnout(observed,reference){
  if(observed?.reporting_complete!==true||observed.administrative||reference?.available===false||!Number.isFinite(reference?.total_votes)||reference.total_votes<=0||!Number.isFinite(observed.total)||observed.total<0)return null;
  return {percent:100*observed.total/reference.total_votes,currentVotes:observed.total,referenceVotes:reference.total_votes,estimated:reference.is_estimate===true,lowerBound:observed.write_ins_supplied===false&&observed.write_ins_applicable!==false,completionBasis:observed.reporting_basis||'Source reports complete'};
 }
 return {usable,calibrate,aggregate,targetFor,compare,turnout};
});
