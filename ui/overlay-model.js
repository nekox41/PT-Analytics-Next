/* SPDX-License-Identifier: MIT */
"use strict";
const OverlayModel=(()=>{
 const names=["飞行","第一阶段","第二阶段","第三阶段","第四阶段"];
 const stageLabels={pending:"等待离开电梯",flight:"飞行中",shield:"破盾中",leg:"破腿中",body:"本体击杀",pylon:"摧毁信标",transition:"阶段转换",finished:"任务完成"};
 const labels={shield:"破盾",leg:"破腿",body:"本体",pylon:"信标"};
 const phaseMetrics={1:["shield","leg","body","pylon"],2:["leg","body"],3:["shield","leg","body","pylon"],4:["shield","leg","body"]};
 const format=t=>Number.isFinite(t)?t.toFixed(3):"—";
 const demo={key:"preview",elapsed:48.326,phase:3,phase_elapsed:4.745,flight:11.328,completed:[{phase_number:1,total_time:25.748},{phase_number:2,total_time:6.505}],active_metric:"shield",shield:"Toxin",shield_time:4.745,leg_time:null,body_time:null,pylon_time:null,legs:0,uncertain:false};
 const num=n=>Number.isFinite(n)?n:null;
 const plus=(n,d)=>num(n)===null?null:n+d;
 function derive(frame,secondsSinceReceive=0){
  const cfg=frame?.settings?.config||{},preview=!!frame?.settings?.editing;
  const v=preview?demo:frame?.live;
  if(!v)return {preview,waiting:true,stages:names.map(name=>({name,state:"waiting",value:null})),metrics:[],label:"等待任务",total:null,delta:null,shield:null,uncertain:false};
  const age=Math.max(0,(frame.age_ms||0)/1000+secondsSinceReceive);
  // Short local extrapolation only. Never advance through a lost backend connection.
  const delta=preview||v.active_metric==="finished"||v.active_metric==="pending"?0:Math.min(age,0.75);
  const elapsed=plus(v.elapsed,delta),finished=v.active_metric==="finished",p=v.phase;
  const stages=names.map((name,i)=>{
   if(i===0){
    if(Number.isFinite(v.flight))return {name,state:"done",value:v.flight};
    return {name,state:v.active_metric==="flight"?"active":"waiting",value:v.active_metric==="flight"?elapsed:null};
   }
   const ended=v.completed.find(x=>x.phase_number===i);
   if(ended)return {name,state:"done",value:ended.total_time};
   return {name,state:!finished&&p===i?"active":"waiting",value:!finished&&p===i?plus(v.phase_elapsed,delta):null};
  });
  let comparison=null,compareLabel="";
  const pb=preview?{flight:12.0,cumulative:[38,46,76,82]}:frame.pb;
  if(cfg.compare&&pb&&!v.uncertain&&Number.isFinite(v.flight)){
   const count=v.completed.length;
   const previous=count?pb.cumulative?.[count-1]:pb.flight;
   const current=v.flight+v.completed.reduce((sum,x)=>sum+x.total_time,0);
   if(Number.isFinite(previous)){comparison=current-previous;compareLabel=count?"P"+count+" 累计 vs PB":"飞行 vs PB";}
  }
  const metrics=cfg.details&&!finished?(phaseMetrics[p]||[]).map(key=>({
   key,label:labels[key],active:v.active_metric===key,value:plus(v[key+"_time"],v.active_metric===key?delta:0)
  })):[];
  return {preview,waiting:false,total:elapsed,stages,metrics,label:stageLabels[v.active_metric]||"等待日志",
   delta:comparison,compareLabel,shield:v.active_metric==="shield"?v.shield:null,legs:v.legs,phase:p,finished,uncertain:v.uncertain,stale:!preview&&!finished&&age>1.5};
 }
 return {derive,format};
})();
if(typeof module!=="undefined"&&module.exports)module.exports=OverlayModel;
