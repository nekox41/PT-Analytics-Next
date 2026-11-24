/* SPDX-License-Identifier: GPL-3.0-or-later */
"use strict";
const Report = (() => {
 const metrics=[
 ["total_time","总用时","#69adff"],["total_flight_time","飞行时间","#ffad50"],
 ["total_shield_time","破盾时间","#8588ea"],["total_leg_time","破腿时间","#50cbd3"],
 ["total_body_time","本体击杀","#e45860"],["total_pylon_time","信标耗时","#e482d9"]
 ];
 const names=["第一阶段","第二阶段","第三阶段","第四阶段"];
 const definitions={
 shield:{key:"total_shield_time",cn:"护盾",en:"Shield change",list:"shield_changes",value:"shield_time",name:"status_effect"},
 leg:{key:"total_leg_time",cn:"破腿",en:"Leg break",list:"leg_breaks",value:"leg_break_time",name:"leg_position"},
 body:{key:"total_body_kill_time",cn:"本体",en:"Body killed"},
 pylon:{key:"total_pylon_time",cn:"信标",en:"Pylons"}
 };
 const types={1:["shield","leg","body","pylon"],2:["leg","body"],3:["shield","leg","body","pylon"],4:["shield","leg","body"]};
 const legNames={FrontLeft:"FL",FrontRight:"FR",BackLeft:"BL",BackRight:"BR"};
 const damageNames={Impact:"Impact",Puncture:"Puncture",Slash:"Slash",Heat:"Heat",Cold:"Cold",Electric:"Electric",Toxin:"Toxin",Blast:"Blast",Radiation:"Radiation",Gas:"Gas",Magnetic:"Magnetic",Viral:"Viral",Corrosive:"Corrosive",NoShield:"None"};
 const damageFiles=["Impact","Puncture","Slash","Heat","Cold","Electric","Toxin","Blast","Radiation","Gas","Magnetic","Viral","Corrosive"];
 const damageIcon=name=>damageFiles.includes(name)?"damage-icons/"+name+".png":null;
 const iconCache=new Map();
 async function loadIcons(r){
  const names=[...new Set(phases(r).flatMap(p=>p.rows.flatMap(row=>row.splits)).filter(s=>s.icon).map(s=>s.name))];
  await Promise.all(names.map(async name=>{
   if(!iconCache.has(name)){
    const image=new Image();image.src=damageIcon(name);
    await image.decode();iconCache.set(name,image);
   }
  }));
  return iconCache;
 }
 const fmt=n=>Number.isFinite(n)?n.toFixed(3):"—";
 const sec=n=>Number.isFinite(n)?fmt(n)+"s":"—";
 const elapsed=r=>r.elapsed_seconds??(!r.is_aborted_run&&!r.is_bugged_run&&(r.lifecycle==="completed"||!r.lifecycle&&r.phases?.length===4)?r.total_times.total_time:null);
 const life=r=>r.lifecycle||(r.is_aborted_run?"aborted":r.is_bugged_run||r.phases?.length!==4?"invalid":"completed");
 const eligible=r=>life(r)==="completed"&&!r.is_bugged_run&&Number.isFinite(elapsed(r))&&elapsed(r)>0&&r.phases?.length===4&&r.phases.every(p=>p.complete!==false);
 function rows(p){
  return (types[p.phase_number]||[]).map(type=>{
   const d=definitions[type],raw=p[d.key],value=Number.isFinite(raw)&&!(p.complete===false&&raw===0)?raw:null;
   const splits=(d.list?p[d.list]||[]:[]).map(x=>({name:type==="shield"?damageNames[x[d.name]]||x[d.name]:legNames[x[d.name]]||x[d.name],symbol:type==="shield"?damageNames[x[d.name]]||String(x[d.name]??"Unknown"):legNames[x[d.name]]||"?",icon:type==="shield"?damageIcon(x[d.name]):null,time:x[d.value],type}));
   return {type,cn:d.cn,en:d.en,value,splits};
  });
 }
 function phases(r){
  let cumulative=Number.isFinite(r.total_times.total_flight_time)?r.total_times.total_flight_time:0;
  return names.map((name,i)=>{
   const p=r.phases.find(p=>p.phase_number===i+1);
   if(!p)return {name,number:i+1,missing:true,rows:[]};
   cumulative+=p.total_time;
   return {name,number:i+1,duration:p.total_time,cumulative,complete:p.complete!==false,rows:rows(p)};
  });
 }
 function duration(t){
  if(!Number.isFinite(t))return "unknown time";
  const ms=Math.round(Math.max(0,t)*1000),m=Math.floor(ms/60000),s=Math.floor(ms/1000)%60;
  return m+"m "+s+"s "+(ms%1000)+"ms";
 }
 function metricValues(r,previous){
  const compare=eligible(r)&&previous&&eligible(previous);
  return metrics.map(([key,label,color],i)=>{
   const value=i===0?elapsed(r):r.total_times[key];
   const other=compare?(i===0?elapsed(previous):previous.total_times[key]):null;
   return {key,label,color,value,delta:Number.isFinite(value)&&Number.isFinite(other)?value-other:null,lower:i===0&&life(r)==="interrupted"};
  });
 }
 function deltaText(d){return Number.isFinite(d)?(d>0?"+":d<0?"-":"")+fmt(Math.abs(d))+"s":"";}
 function plain(s){return String(s??"").replace(/[\r\n\t]+/g," ").trim();}
 function text(r){
  const bar="-".repeat(72),total=elapsed(r),status=life(r),player=r.self_verified&&r.player_name?plain(r.player_name):"Unknown player";
  const name=plain(r.run_name)||("Run #"+r.run_id);
  const headline=status==="completed"?"cleared in "+duration(total):({aborted:"aborted",interrupted:"interrupted",invalid:"invalid"}[status]||status)+(Number.isFinite(total)?" after "+(status==="interrupted"?"at least ":"")+duration(total):" (elapsed unknown)");
  const lines=["",bar,"Profit-Taker "+name+" by "+player+" "+headline];
  const peers=[...new Set([!r.self_verified?r.player_name:"",...(r.squad_members||[]).map(p=>p.member_name)].filter(Boolean).map(plain))];
  if(peers.length)lines.push("Participants: "+[...(r.self_verified?[player+" (self)"]:[]),...peers].join(" | "));
  lines.push("");
  if(Number.isFinite(total)&&Number.isFinite(r.total_times.total_flight_time)&&r.phases.length)lines.push("From elevator to Profit-Taker took "+sec(r.total_times.total_flight_time)+". Fight duration: "+duration(Math.max(0,total-r.total_times.total_flight_time))+".");
  else lines.push("Flight and fight duration: incomplete evidence.");
  const rowText=(label,value)=>" "+(label+":").padEnd(17)+sec(value);
  for(const phase of phases(r)){
   if(phase.missing)continue;
   lines.push("","> Phase "+phase.number+" ["+fmt(phase.duration)+"]"+(phase.complete?"":" (incomplete)"));
   for(const row of phase.rows)lines.push(rowText(row.en,row.value)+(row.splits.length?" - "+row.splits.map(s=>(row.type==="shield"?s.name+" ":"")+sec(s.time)).join(" | "):""));
  }
  const totals=r.total_times,values=[totals.total_shield_time,totals.total_leg_time,totals.total_body_time,totals.total_pylon_time];
  lines.push("","> Sum of parts ["+fmt(values.every(Number.isFinite)?values.reduce((a,b)=>a+b,0):null)+"]");
  ["Shield change","Leg Break","Body Killed","Pylons"].forEach((label,i)=>lines.push(" "+(label+":").padEnd(16)+sec(values[i])));
  lines.push(bar,"");return lines.join("\n");
 }
 // Dedicated export layout; no DOM rasterization, remote resources or opaque page background.
 function canvas(r,{light=true,pb=false,previous=null,single=false,icons=new Map()}={}){
  const W=1338,gap=12,mh=145,pw=single?W:(W-gap)/2;
  const model=phases(r),values=metricValues(r,previous),muted=light?"#929292":"#aaa",fg=light?"#121212":"#eee",panel=light?"#fff":"#1e1e1e";
  const c=document.createElement("canvas"),ctx=c.getContext("2d");
  const font=(size,weight=400,mono=false)=>ctx.font=weight+" "+size+"px "+(mono?'Consolas, monospace':'"Segoe UI", "Microsoft YaHei UI", sans-serif');
  const width=(s,size=12,weight=400,mono=false)=>{font(size,weight,mono);return ctx.measureText(s).width;};
  // Wrap splits within the corresponding summary row, never clip extra damage changes.
  function layout(row){
   const start=200,limit=pw-16,wrapped=[[]];let x=start;
   for(const split of row.splits){
    const w=(split.icon&&icons.has(split.name)?15:width(split.symbol,12,600))+8+width(fmt(split.time),12,400,true)+18;
    if(x+w>limit&&wrapped[wrapped.length-1].length){wrapped.push([]);x=start;}
    wrapped[wrapped.length-1].push({split,x});x+=w;
   }
   return {wrapped,height:Math.max(23,wrapped.length*22)};
  }
  const laid=model.map(p=>({p,rows:p.rows.map(row=>({row,...layout(row)}))}));
  const heights=[];for(let i=0;i<laid.length;i+=single?1:2)heights.push(Math.max(160,...laid.slice(i,i+(single?1:2)).map(o=>55+o.rows.reduce((a,row)=>a+row.height,0))));
  const H=mh+gap+heights.reduce((a,b)=>a+b,0)+gap*(heights.length-1);
  const scale=2;c.width=W*scale;c.height=H*scale;ctx.scale(scale,scale);ctx.textBaseline="middle";
  function round(x,y,w,h,radius,fill){ctx.beginPath();ctx.roundRect(x,y,w,h,radius);ctx.fillStyle=fill;ctx.fill();}
  function txt(s,x,y,color,size=15,weight=400,align="left",mono=false){font(size,weight,mono);ctx.textAlign=align;ctx.fillStyle=color;ctx.fillText(s,x,y);}
  function clock(x,y){ctx.strokeStyle="#fff";ctx.lineWidth=2.5;ctx.beginPath();ctx.arc(x,y,9,0,Math.PI*2);ctx.moveTo(x,y-5);ctx.lineTo(x,y);ctx.lineTo(x+4,y+3);ctx.stroke();}
  const mw=(W-5*gap)/6;
  values.forEach((m,i)=>{
   const x=i*(mw+gap);round(x,0,mw,mh,11,panel);round(x+20,15,40,40,10,m.color);clock(x+40,35);
   txt(m.label,x+70,35,muted,13,500);
   const value=(m.lower&&Number.isFinite(m.value)?"≥ ":"")+fmt(m.value),color=i===0&&pb?(light?"#68bd39":"#c6d93a"):fg;
   txt(value,x+25,91,color,32,700);
   if(Number.isFinite(m.value))txt("s",x+25+width(value,32,700)+1,96,color,16,400);
   if(m.delta!==null)txt(deltaText(m.delta),x+mw-10,123,m.delta<0?"#35a866":m.delta>0?"#e45850":muted,14,500,"right");
  });
  let y=mh+gap;
  heights.forEach((h,index)=>{
   const count=single?1:2;
   laid.slice(index*count,index*count+count).forEach(({p,rows},i)=>{
    const x=i*(pw+gap);round(x,y,pw,h,11,panel);
    txt(p.name+(p.missing?" · 无数据":p.complete?"":" · 未完成"),x+20,y+26,muted,16);
    if(p.missing)return;
    const cum=sec(p.cumulative),tail=width(cum,20,700);
    txt(cum,x+pw-15,y+26,fg,20,700,"right");
    txt(sec(p.duration)+" /",x+pw-22-tail,y+26,muted,15,500,"right");
    let yy=y+51;const rowHeight=rows.reduce((a,r)=>a+r.height,0);
    ctx.strokeStyle=muted;ctx.lineWidth=1;ctx.beginPath();ctx.moveTo(x+185,yy);ctx.lineTo(x+185,yy+rowHeight);ctx.stroke();
    rows.forEach(({row,wrapped,height})=>{
     txt(row.cn,x+20,yy+11,fg,15);txt(fmt(row.value),x+169,yy+11,muted,15,400,"right",true);
     wrapped.forEach((line,j)=>line.forEach(({split,x:sx})=>{
      const dy=yy+11+j*22;
      const image=split.icon&&icons.get(split.name);
      let symbolWidth=width(split.symbol,12,600);
      if(image){
       // Tint the original alpha mask for the selected theme, preserving its silhouette.
       const mask=document.createElement("canvas");mask.width=32;mask.height=32;
       const m=mask.getContext("2d");const ratio=Math.min(32/image.naturalWidth,32/image.naturalHeight);
       const iw=image.naturalWidth*ratio,ih=image.naturalHeight*ratio;
       m.drawImage(image,(32-iw)/2,(32-ih)/2,iw,ih);
       m.globalCompositeOperation="source-in";m.fillStyle=fg;m.fillRect(0,0,32,32);
       ctx.drawImage(mask,x+sx,dy-7.5,15,15);symbolWidth=15;
      } else {txt(split.symbol,x+sx,dy,fg,12,600);}
      txt(fmt(split.time),x+sx+symbolWidth+8,dy,muted,12,400,"left",true);
     }));
     yy+=height;
    });
   });
   y+=h+gap;
  });
  return c;
 }
 return {metrics,names,rows,phases,text,canvas,damageIcon,loadIcons,metricValues,deltaText,eligible,elapsed,life,duration,fmt,sec};
})();
if(typeof module!=="undefined"&&module.exports)module.exports=Report;
