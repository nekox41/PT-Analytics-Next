/* SPDX-License-Identifier: MIT */
"use strict";
const $=id=>document.getElementById(id);
const invoke=(cmd,args={})=>window.__TAURI__.core.invoke(cmd,args);
const esc=s=>String(s??"").replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
const damageNames={Impact:"冲击",Puncture:"穿刺",Slash:"切割",Heat:"火焰",Cold:"冰冻",Electric:"电击",Toxin:"毒素",Blast:"爆炸",Radiation:"辐射",Gas:"毒气",Magnetic:"磁力",Viral:"病毒",Corrosive:"腐蚀"};
let frame=null,received=0,polling=false,lastShape="",lastShield="";
function update(t){
 const cfg=frame?.settings?.config||{},editing=!!frame?.settings?.editing;
 const v=OverlayModel.derive(frame,Math.max(0,t-received)/1000),f=OverlayModel.format;
 document.documentElement.classList.toggle("vertical",!!cfg.vertical);
 document.documentElement.classList.toggle("editing",editing);
 document.documentElement.style.setProperty("--alpha",String((cfg.opacity??85)/100));
 $("finish").hidden=!editing;$("resize").hidden=!editing;
 $("live-status").textContent=editing?"预览 · 拖动顶部调整位置":v.stale?"实时数据暂停":v.uncertain?"数据异常 · "+v.label:v.label;
 $("total").textContent=f(v.total)+(Number.isFinite(v.total)?"s":"");
 $("compare-label").textContent=v.compareLabel||"";
 $("comparison").textContent=Number.isFinite(v.delta)?(v.delta>0?"+":"")+f(v.delta)+"s":"";
 $("comparison").className=v.delta<0?"faster":v.delta>0?"slower":"";
 if(v.shield!==lastShield){
  const src=Report.damageIcon(v.shield);
  $("shield").innerHTML=src?'<i class="damage" style="--image:url('+esc(src)+')"></i>'+esc(damageNames[v.shield]||v.shield):"";
  lastShield=v.shield;
 }
 const shape=JSON.stringify([v.stages.map(x=>[x.name,x.state]),v.metrics.map(x=>[x.key,x.active]),v.phase]);
 if(shape!==lastShape){
  $("stages").innerHTML=v.stages.map((s,i)=>'<article class="stage '+s.state+'"><span>'+esc(s.name)+'</span><b id="stage-'+i+'">—</b></article>').join("");
  $("details").innerHTML=v.metrics.map(m=>'<article class="detail '+(m.active?"active":"")+'"><span>'+esc(m.label)+(m.key==="leg"?'<small id="legs"></small>':"")+'</span><b id="metric-'+m.key+'">—</b></article>').join("");
  $("details").hidden=!v.metrics.length;$("details").style.setProperty("--cols",String(Math.max(1,v.metrics.length)));lastShape=shape;
 }
 v.stages.forEach((s,i)=>{$("stage-"+i).textContent=f(s.value)});
 v.metrics.forEach(m=>{$("metric-"+m.key).textContent=f(m.value)});
 if($("legs"))$("legs").textContent=" "+(v.legs||0)+"/4";
 requestAnimationFrame(update);
}
async function poll(){
 if(polling)return;polling=true;
 try{frame=await invoke("get_overlay_frame");received=performance.now();$("error").textContent=""}
 catch(e){$("error").textContent="Overlay 连接异常";}
 finally{polling=false;}
}
$("handle").addEventListener("pointerdown",e=>{if(frame?.settings?.editing&&e.button===0&&!e.target.closest("button"))invoke("drag_overlay",{resize:false}).catch(e=>$("error").textContent=String(e));});
$("resize").addEventListener("pointerdown",e=>{if(e.button===0&&frame?.settings?.editing)invoke("drag_overlay",{resize:true}).catch(e=>$("error").textContent=String(e));});
$("finish").onclick=async()=>{try{await invoke("edit_overlay",{editing:false});await poll()}catch(e){$("error").textContent=String(e)}};
document.addEventListener('visibilitychange',()=>{if(!document.hidden)poll()});
poll();setInterval(poll,250);requestAnimationFrame(update);
