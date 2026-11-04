/* Compare accepted vote observations; polling and source timestamps are not results. */
(function(root,factory){const api=factory();if(typeof module==='object'&&module.exports)module.exports=api;else root.ElectionAlerts=api;})(typeof globalThis!=='undefined'?globalThis:this,()=>{
 'use strict';
 const count=n=>Number.isFinite(n)&&n>=0?n:null;
 const identity=c=>String(c.name||'').toUpperCase().replace(/[^A-Z0-9]/g,'');
 const votes=x=>new Map((x?.candidates||[]).filter(c=>count(c.votes)!=null).map(c=>[identity(c),c.votes]));
 const positive=x=>count(x?.total)>0||[...votes(x).values()].some(n=>n>0);
 const changed=(a,b)=>a.size!==b.size||[...a].some(([k,v])=>b.get(k)!==v);
 const decreased=(a,b)=>[...a].some(([k,v])=>b.has(k)&&b.get(k)<v);
 const coverage=r=>new Map((r.county_results||[]).map(c=>[c.county_id||c.county,c.source_id||'']));
 function areas(r){
  return new Map([...(r.units||[]).map(x=>['precinct:'+x.county+':'+x.name,x]),...(r.county_results||[]).map(x=>['county:'+(x.county_id||x.county),x])]);
 }
 function changes(previous,next){
  if(!previous||!next)return [];
  const prior=new Map((previous.races||[]).map(r=>[r.id,r]));
  return(next.races||[]).flatMap(r=>{
   const old=prior.get(r.id),before=count(old?.total),after=count(r.total);
   if(!old||before==null){return positive(r)?[{id:r.id,name:r.name,before,after,delta:null,kind:'new'}]:[];}
   if(after==null)return []; // A lost feed is covered by source-health warnings.
   const oldVotes=votes(old),newVotes=votes(r);
   const totalsChanged=before!==after||changed(oldVotes,newVotes);
   let detailChanged=false,correction=after<before||decreased(oldVotes,newVotes);
   const oldAreas=areas(old);
   for(const[key,area]of areas(r)){
    const priorArea=oldAreas.get(key);
    if(priorArea){
     const a=votes(priorArea),b=votes(area);
     if(count(priorArea.total)!==count(area.total)||changed(a,b))detailChanged=true;
     if((count(area.total)!=null&&count(priorArea.total)!=null&&area.total<priorArea.total)||decreased(a,b))correction=true;
    }else if(positive(area))detailChanged=true;
   }
   if(!totalsChanged&&!detailChanged)return [];
   // A newly exposed zero-valued setup choice is still not an election return.
   if(!positive(old)&&!positive(r))return [];
   const sourceChange=positive(old)&&changed(coverage(old),coverage(r));
   return[{id:r.id,name:r.name,before,after,delta:after-before,kind:sourceChange?'coverage':correction?'correction':totalsChanged?'votes':'detail'}];
  });
 }
 return{changes};
});
