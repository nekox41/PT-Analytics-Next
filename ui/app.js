"use strict";
const $=id=>document.getElementById(id);
const invoke=(cmd,args={})=>window.__TAURI__?.core?.invoke(cmd,args) ?? Promise.reject(new Error("请从 Tauri 桌面程序启动"));
const icons={
home:'<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1z"/>',
box:'<path d="m3 8 3-4h12l3 4-3 4H6zM5 12v8h14v-8M12 4v8"/>',
chart:'<rect x="3" y="3" width="18" height="18" rx="3"/><path d="M8 16v-5m4 5V7m4 9v-7"/>',
settings:'<path d="m12 2 3 2 4 1 1 4 2 3-2 3-1 4-4 1-3 2-3-2-4-1-1-4-2-3 2-3 1-4 4-1z"/><circle cx="12" cy="12" r="3"/>',
left:'<path d="m15 5-7 7 7 7M8 12h13"/>',right:'<path d="m9 5 7 7-7 7M16 12H3"/>',
layout:'<rect x="3" y="4" width="18" height="6" rx="1"/><rect x="3" y="14" width="18" height="6" rx="1"/>',
sun:'<circle cx="12" cy="12" r="4"/><path d="M12 1v3m0 16v3M1 12h3m16 0h3M4 4l2 2m12 12 2 2M4 20l2-2M18 6l2-2"/>',
moon:'<path d="M20 15A9 9 0 0 1 9 4 9 9 0 1 0 20 15z"/>',
menu:'<path d="M4 6h16M4 12h16M4 18h16"/>',refresh:'<path d="M20 7v5h-5M4 17v-5h5M4 12a8 8 0 0 1 14-5l2 5M20 12a8 8 0 0 1-14 5l-2-5"/>',
clock:'<circle cx="12" cy="12" r="9"/><path d="M12 6v6l4 2"/>',flight:'<path d="M12 2v20M3 14l9-6 9 6M8 21l4-3 4 3"/>',
shield:'<path d="m12 3 8 4v5c0 5-8 9-8 9s-8-4-8-9V7z"/>',
leg:'<path d="M4 3v10l5 5h10l2 3M8 3v8l4 4h5l4 4"/>',
body:'<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="2"/><path d="M12 1v4m0 14v4M1 12h4m14 0h4"/>',
pylon:'<circle cx="12" cy="5" r="3"/><circle cx="5" cy="17" r="3"/><circle cx="19" cy="17" r="3"/>',
edit:'<path d="m4 16 12-12 4 4L8 20H4zM14 6l4 4"/>',trash:'<path d="M4 6h16M9 6V3h6v3M6 6l1 15h10l1-15M10 10v7m4-7v7"/>',
eye:'<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
star:'<path d="m12 2 3 6 7 1-5 5 1 7-6-3-6 3 1-7-5-5 7-1z"/>',
copy:'<rect x="8" y="8" width="12" height="13" rx="1.5"/><path d="M16 8V3H3v13h5"/>',
document:'<path d="M6 3h8l5 5v13H6zM14 3v6h5M9 13h7M9 17h7"/>',
share:'<circle cx="5" cy="12" r="2"/><circle cx="19" cy="5" r="2"/><circle cx="19" cy="19" r="2"/><path d="m7 11 10-5M7 13l10 5"/>'
};
const icon=k=>'<svg class="icon" viewBox="0 0 24 24" aria-hidden="true">'+(icons[k]||icons.clock)+'</svg>';
const btn=(k,label,action,extra="")=>'<button type="button" title="'+esc(label)+'" aria-label="'+esc(label)+'" data-action="'+action+'" '+extra+'>'+icon(k)+'</button>';
const esc=s=>String(s??"").replace(/[&<>"']/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
const fmt=n=>Number.isFinite(n)?n.toFixed(3):"—";
const dt=n=>n?new Date(n*1000).toLocaleString("zh-CN",{hour12:false}):"—";
const metrics=[["total_time","总用时","clock","#69adff"],["total_flight_time","飞行时间","flight","#ffb451"],["total_shield_time","破盾时间","shield","#8991f1"],["total_leg_time","破腿时间","leg","#55d2da"],["total_body_time","本体击杀","body","#ec6067"],["total_pylon_time","信标耗时","pylon","#e68ce4"]];
const pages=[["home","主页","home"],["storage","历史记录","box"],["analytics","统计分析","chart"],["settings","设置","settings"]];
const phaseNames=["第一阶段","第二阶段","第三阶段","第四阶段"];
const shieldNames={Impact:"冲击",Puncture:"穿刺",Slash:"切割",Heat:"火焰",Cold:"冰冻",Electric:"电击",Toxin:"毒素",Blast:"爆炸",Radiation:"辐射",Gas:"毒气",Magnetic:"磁力",Viral:"病毒",Corrosive:"腐蚀",NoShield:"无"};
const legs={FrontLeft:"FL",FrontRight:"FR",BackLeft:"BL",BackRight:"BR"};
let state={page:"home",runs:[],selected:null,status:{mode:"waiting"},revision:-1,sort:"date",asc:false,filter:"complete",search:"",pageNo:0,chosen:new Set(),single:localStorage.getItem("single")==="1",light:localStorage.getItem("light")==="1",chartLimit:20};
let toastTimer,dialogAction,polling=false,overlaySettings=null,overlayBusy=false,overlayReadBusy=false;
function toast(t){$("toast").textContent=t;$("toast").classList.add("show");clearTimeout(toastTimer);toastTimer=setTimeout(()=>$("toast").classList.remove("show"),3500)}
function players(r){
 if(!r)return [];
 const out=[],seen=new Set();
 const add=(name,self=false)=>{if(!name)return;const key=name.toLowerCase();if(seen.has(key))return;seen.add(key);out.push({name,self})};
 add(r.player_name,Boolean(r.self_verified));
 (r.squad_members||[]).forEach(m=>add(m.member_name));
 return out;
}
function roster(r,compact=false){
 const list=players(r);
 const label=list.length===0?"玩家未知":r.self_verified?(list.length===1?"单人任务":"多人任务 · "+list.length+" 位玩家"):"已识别 "+list.length+" 位玩家 · 自身未确认";
 return '<div class="roster '+(compact?'roster-compact':'')+'">'+(compact?"":'<span class="roster-label">'+esc(label)+'</span>')+(list.length?list.map(p=>'<span class="player-chip '+(p.self?'player-self':'')+'" title="'+esc(p.name+(p.self?"（自己）":""))+'"><span class="player-name">'+esc(p.name)+'</span>'+(p.self?'<span class="self-tag">自己</span>':'')+'</span>').join(""):compact?'<span class="muted">未知</span>':"")+'</div>';
}
function selected(){return state.runs.find(r=>r.run_id===state.selected)||state.runs[0]}
const lifecycle=r=>r.lifecycle||(r.is_aborted_run?"aborted":r.is_bugged_run||r.phases.length!==4?"invalid":"completed");
const statusText=r=>({completed:"正常完成",aborted:"主动中止",interrupted:"意外中断",invalid:"数据异常",pending:"尚未开始",running:"进行中"})[lifecycle(r)]||"未知";
const reasonText=r=>({boss_defeated:"Boss 击杀",mission_abort:"主动退出任务",return_to_town:"返回福尔图娜",log_reset:"日志重置或替换",path_changed:"切换日志路径",session_changed:"游戏会话变更",recovery:"原日志不可恢复",missing_evidence:"缺少可靠日志证据"})[r.end_reason]||"";
const elapsed=r=>r.elapsed_seconds??(lifecycle(r)==="completed"?r.total_times.total_time:null);
const labelTime=r=>elapsed(r)==null?"—":(lifecycle(r)==="interrupted"?"≥ ":"")+fmt(elapsed(r))+"s";
function valid(){return state.runs.filter(Report.eligible)}
function updateStatus(){
 const s=state.status,labels={waiting:"等待游戏日志",replay:"正在同步历史日志",live:"日志监听正常",retry:"读取异常 · 自动重连",storage_error:"数据库写入异常 · 正在重试",offline:"核心连接失败"};
 $("status").innerHTML='<i class="'+esc(s.mode)+'"></i><span>'+esc(labels[s.mode]||s.mode)+(s.lifecycle==="pending"?" · 等待离开电梯":s.lifecycle==="running"?" · 任务进行中":"")+'</span>';
 $("status").title=[s.path,s.error].filter(Boolean).join("\n");
 $("progress").textContent=(s.mode==="replay"&&s.size?Math.min(100,Math.floor(s.offset/s.size*100))+"% · ":"")+(s.warnings?"异常行 "+s.warnings:"");
 if(state.page==="settings"&&$("setting-status")){$("setting-status").textContent=labels[s.mode]||s.mode;$("setting-error").textContent=s.error||"";$("setting-offset").textContent=(s.offset||0).toLocaleString()+" / "+(s.size||0).toLocaleString()+" B"}
}
async function refresh(force=false){
 if(polling){if(force)setTimeout(()=>refresh(true),150);return;}polling=true;
 try{
  const s=await invoke("get_status");state.status=s;updateStatus();
  if(force||state.revision!==s.revision){
   const follow=state.selected===null||state.selected===state.runs[0]?.run_id;
   state.runs=await invoke("get_runs");
   if(follow||!state.runs.some(r=>r.run_id===state.selected))state.selected=state.runs[0]?.run_id??null;
   state.revision=s.revision;
   if(state.page!=="settings"||force)render();
  }
 }catch(e){state.status={mode:"offline",error:String(e)};updateStatus();if(force)toast(String(e))}finally{polling=false}
}
function render(){
 document.documentElement.classList.toggle("light",state.light);
 $("rail").innerHTML=pages.map(([id,label,k])=>btn(k,label,"page:"+id,'class="'+(id===state.page?"active":"")+'"')).join("");
 $("subtitle").textContent={home:selected()?.self_verified?"你好，"+selected().player_name+"！":"你好！",storage:"你的历史记录",analytics:"你的竞速统计",settings:"应用设置"}[state.page];
 const index=state.runs.findIndex(r=>r.run_id===selected()?.run_id);
 $("toolbar").innerHTML=(state.page==="home"?btn("left","上一场（较早）","older",index>=state.runs.length-1||index<0?"disabled":"")+btn("right","下一场（较新）","newer",index<=0?"disabled":"")+btn("layout","切换阶段布局","layout"):btn("refresh","刷新","refresh"))+btn(state.light?"moon":"sun","切换主题","theme")+(state.page==="home"?btn("menu","设置","page:settings"):"");
 ({home:renderHome,storage:renderStorage,analytics:renderAnalytics,settings:renderSettings}[state.page])();
 updateStatus();
}
function metricCards(t,average=false,pb=false,partial=false,lowerBound=false){return '<div class="metric-grid">'+metrics.map(([key,label,k,color],i)=>'<article class="metric"><div class="metric-top"><span class="metric-icon" style="background:'+color+'">'+icon(k)+'</span><span>'+label+(average?"<br>平均值":partial&&i!==0?"<br>已记录":"")+'</span></div><div class="metric-value '+(average?"avg "+(i%2?"alt":""):i===0&&pb?"pb":"")+'">'+(i===0&&lowerBound&&Number.isFinite(t[key])?'≥ ':'')+fmt(t[key])+(Number.isFinite(t[key])?'<small>s</small>':'')+'</div></article>').join("")+'</div>'}
function empty(k,title,desc){return '<div class="empty">'+icon(k)+'<h2>'+esc(title)+'</h2><p>'+esc(desc)+'</p><button class="primary" data-action="refresh">刷新记录</button></div>'}
function previousRun(r){
 const index=state.runs.findIndex(x=>x.run_id===r.run_id);
 return index>=0?state.runs.slice(index+1).find(Report.eligible):null;
}
function isPB(r){return Report.eligible(r)&&!valid().some(x=>x.total_times.total_time<r.total_times.total_time);}
function reportMetrics(r){
 return '<div class="metric-grid report-metrics">'+Report.metricValues(r,previousRun(r)).map((m,i)=>
 '<article class="metric"><div class="metric-top"><span class="metric-icon" style="background:'+m.color+'">'+icon("clock")+'</span><span>'+m.label+'</span></div><div class="metric-value '+(i===0&&isPB(r)?"pb":"")+'">'+(m.lower&&Number.isFinite(m.value)?"≥ ":"")+fmt(m.value)+(Number.isFinite(m.value)?'<small>s</small>':'')+'</div><div class="metric-delta '+(m.delta<0?"faster":m.delta>0?"slower":"muted")+'" title="相对于上一场有效任务的变化">'+Report.deltaText(m.delta)+'</div></article>').join("")+'</div>';
}
function phaseCards(r){
 return '<div class="phase-grid '+(state.single?"single":"")+'">'+Report.phases(r).map(p=>{
 if(p.missing)return '<article class="phase"><div class="phase-head"><span>'+p.name+'</span><strong>—</strong></div><span class="muted">无阶段数据</span></article>';
 return '<article class="phase"><div class="phase-head"><span>'+p.name+(p.complete?"":' <span class="warn">未完成</span>')+'</span><span class="phase-duration">'+fmt(p.duration)+'s / <strong>'+fmt(p.cumulative)+'s</strong></span></div><div class="phase-details">'+p.rows.map(row=>
 '<div class="phase-detail-row"><div class="phase-detail-summary"><span>'+row.cn+'</span><span>'+fmt(row.value)+'</span></div><div class="split-row">'+row.splits.map(s=>'<span class="split" title="'+esc(s.type==="shield"?shieldNames[s.name]||s.name:s.name)+'">'+(s.icon?'<span class="damage-icon" role="img" aria-label="'+esc(shieldNames[s.name]||s.name)+'" style="--damage-image:url('+esc(s.icon)+')"></span>':'<b>'+esc(s.symbol)+'</b>')+fmt(s.time)+'</span>').join("")+'</div></div>').join("")+'</div></article>';
 }).join("")+'</div>';
}
function renderHome(){
 const r=selected();if(!r){$("content").innerHTML=empty("clock","暂无任务记录","启动 Warframe 并完成利润收割者任务。软件将自动读取 EE.log，也可在设置中指定日志文件。");return}
 $("content").innerHTML='<div class="run-title"><span class="name">你的任务「'+esc(r.run_name)+'」</span>'+btn("edit","重命名","rename:"+r.run_id)+btn("copy","复制成绩图片","copy-image:"+r.run_id)+btn("document","复制中文文本战报","copy-text:"+r.run_id)+btn("star",r.favorite?"取消收藏":"收藏","favorite:"+r.run_id,r.favorite?'style="color:#ffbe5a"':"")+(lifecycle(r)!=="completed"?'<span class="warn" title="'+esc(reasonText(r))+'">'+esc(statusText(r))+'</span>':isPB(r)?'<span class="pb-label">最佳成绩！</span>':"")+'</div>'+roster(r)+(lifecycle(r)!=="completed"?'<p class="muted">'+esc(reasonText(r))+(lifecycle(r)==="interrupted"?" · 总耗时为最后可信战斗事件的下限":"")+(elapsed(r)==null?" · 缺少计时起点或旧记录耗时不完整":" · 分项仅包含已确认事件")+"</p>":"")+'<section id="result-cards">'+reportMetrics(r)+phaseCards(r)+'</section>';
}
async function copyText(r){
 const text=Report.text(r);
 try{await invoke("copy_report_text",{text});}
 catch(nativeError){if(!navigator.clipboard?.writeText)throw nativeError;await navigator.clipboard.writeText(text);}
 toast("已复制中文文本战报");
}
async function copyImage(r){
 const icons=await Report.loadIcons(r);
 const canvas=Report.canvas(r,{light:state.light,pb:isPB(r),previous:previousRun(r),single:state.single,icons});
 const data=canvas.toDataURL("image/png");
 if(!data.startsWith("data:image/png;base64,"))throw new Error("图片生成失败");
 try{await invoke("copy_report_image",{png:data.slice("data:image/png;base64,".length)});}
 catch(nativeError){
  if(!navigator.clipboard?.write||typeof ClipboardItem==="undefined")throw nativeError;
  const blob=await new Promise(resolve=>canvas.toBlob(resolve,"image/png"));
  if(!blob)throw new Error("图片生成失败");
  await navigator.clipboard.write([new ClipboardItem({"image/png":blob})]);
 }
 toast("已复制成绩图片");
}
function filtered(){
 let rows=state.runs.filter(r=>(state.filter==="all"||state.filter==="favorite"&&r.favorite||state.filter==="complete"&&Report.eligible(r)||state.filter==="aborted"&&lifecycle(r)==="aborted"||state.filter==="interrupted"&&lifecycle(r)==="interrupted"||state.filter==="bugged"&&(lifecycle(r)==="invalid"||lifecycle(r)==="completed"&&!Report.eligible(r)))&&(r.run_name.toLowerCase().includes(state.search.toLowerCase())||players(r).some(p=>p.name.toLowerCase().includes(state.search.toLowerCase()))));
 const compare=(a,b)=>state.sort==="name"?a.run_name.localeCompare(b.run_name,undefined,{numeric:true}):state.sort==="time"?(elapsed(a)??-1)-(elapsed(b)??-1):a.time_stamp-b.time_stamp||a.run_id-b.run_id;
 return rows.sort((a,b)=>(state.asc?1:-1)*compare(a,b));
}
function renderStorage(){
 const rows=filtered(),pageSize=15,maxPage=Math.max(0,Math.ceil(rows.length/pageSize)-1);state.pageNo=Math.min(state.pageNo,maxPage);
 const visible=rows.slice(state.pageNo*pageSize,(state.pageNo+1)*pageSize);
 const sort=(name,label)=>'<button data-action="sort:'+name+'">'+label+(state.sort===name?(state.asc?" ↑":" ↓"):"")+'</button>';
 $("content").innerHTML='<div class="storage-tools"><div><input id="search" placeholder="搜索任务或玩家" value="'+esc(state.search)+'"><select id="filter">'+[["all","全部任务"],["favorite","收藏"],["complete","正常完成"],["aborted","主动中止"],["interrupted","意外中断"],["bugged","数据异常"]].map(([v,t])=>'<option value="'+v+'" '+(state.filter===v?"selected":"")+'>'+t+'</option>').join("")+'</select></div><div><span class="muted">共 '+rows.length+' 条</span><button class="danger" data-action="delete-selected" '+(state.chosen.size?"":"disabled")+'>删除已选 ('+state.chosen.size+')</button></div></div><div class="table-wrap"><table><thead><tr><th><input id="select-all" type="checkbox" aria-label="选择本页" '+(visible.length&&visible.every(r=>state.chosen.has(r.run_id))?"checked":"")+'></th><th>'+sort("name","任务名称")+'</th><th>参与玩家</th><th>'+sort("time","任务用时")+'</th><th>'+sort("date","日期")+'</th><th>操作</th></tr></thead><tbody>'+visible.map(r=>'<tr><td><input type="checkbox" data-select="'+r.run_id+'" '+(state.chosen.has(r.run_id)?"checked":"")+'></td><td class="name-cell">'+esc(r.run_name)+(lifecycle(r)!=="completed"?' <span class="warn" title="'+esc(reasonText(r))+'">'+esc(statusText(r))+'</span>':"")+'</td><td class="players-cell">'+roster(r,true)+'</td><td>'+esc(labelTime(r))+'</td><td>'+esc(dt(r.time_stamp))+'</td><td>'+btn("edit","重命名","rename:"+r.run_id)+btn("star","收藏","favorite:"+r.run_id,r.favorite?'class="fav"':"")+btn("trash","删除","delete:"+r.run_id)+btn("eye","查看","view:"+r.run_id)+'</td></tr>').join("")+'</tbody></table></div>'+(rows.length?"":'<div class="empty"><p>没有符合条件的任务记录。</p></div>')+'<div class="table-bottom"><span class="muted">第 '+(state.pageNo+1)+' / '+(maxPage+1)+' 页</span><div><button data-action="table-prev" '+(!state.pageNo?"disabled":"")+'>上一页</button><button data-action="table-next" '+(state.pageNo>=maxPage?"disabled":"")+'>下一页</button></div></div>';
 $("search").addEventListener("input",e=>{state.search=e.target.value;state.pageNo=0;let pos=e.target.selectionStart;renderStorage();$("search").focus();$("search").setSelectionRange(pos,pos)});
 $("filter").addEventListener("change",e=>{state.filter=e.target.value;state.pageNo=0;state.chosen.clear();renderStorage()});
 $("select-all").addEventListener("change",e=>{visible.forEach(r=>e.target.checked?state.chosen.add(r.run_id):state.chosen.delete(r.run_id));renderStorage()});
}
function renderAnalytics(){
 const rows=valid(),times=Object.fromEntries(metrics.map(([k])=>[k,rows.length?rows.reduce((s,r)=>s+r.total_times[k],0)/rows.length:NaN]));
 const shown=rows.slice(0,state.chartLimit).reverse(),best=rows.length?rows.reduce((best,r)=>Math.min(best,r.total_times.total_time),Infinity):NaN;
 let svg="";
 if(shown.length){
 const W=1050,H=340,left=55,right=24,top=24,bottom=54,plotW=W-left-right,plotH=H-top-bottom;
 const maximum=Math.max(...shown.map(r=>Math.max(r.total_times.total_time,r.total_times.total_pylon_time)),10),step=Math.max(5,Math.ceil(maximum/6/5)*5),max=step*6;
 const x=i=>left+plotW*(shown.length===1?.5:i/(shown.length-1)),y=v=>top+plotH*(1-v/max);
 svg='<svg viewBox="0 0 '+W+' '+H+'" role="img" aria-label="任务总耗时及信标耗时趋势">';
 for(let i=0;i<=6;i++){let yy=y(i*step);svg+='<line class="gridline" x1="'+left+'" x2="'+(W-right)+'" y1="'+yy+'" y2="'+yy+'"/><text x="'+(left-9)+'" y="'+(yy+4)+'" text-anchor="end">'+i*step+'s</text>'}
 shown.forEach((r,i)=>{if(i%Math.max(1,Math.ceil(shown.length/12))===0)svg+='<text x="'+x(i)+'" y="'+(H-20)+'" text-anchor="middle">#'+r.display_number+'</text>'});
 for(const [k,c] of [["total_time","#69adff"],["total_pylon_time","#e68ce4"]]){
 svg+='<polyline fill="none" stroke="'+c+'" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" points="'+shown.map((r,i)=>x(i)+","+y(r.total_times[k])).join(" ")+'"/>';
 shown.forEach((r,i)=>{svg+='<circle cx="'+x(i)+'" cy="'+y(r.total_times[k])+'" r="4" fill="'+c+'" stroke="white" stroke-width="1.5"><title>'+esc(r.run_name)+": "+fmt(r.total_times[k])+'s</title></circle>';if(shown.length<=12)svg+='<text x="'+x(i)+'" y="'+(y(r.total_times[k])-10)+'" text-anchor="middle">'+fmt(r.total_times[k])+'</text>'});
 }svg+='</svg>';
 }
 $("content").innerHTML=metricCards(times,true)+'<div class="chart"><div class="storage-tools"><span class="muted">有效任务 '+rows.length+' 场 · PB '+fmt(best)+'s</span><select id="chart-limit"><option value="10">最近 10 场</option><option value="20">最近 20 场</option><option value="50">最近 50 场</option><option value="100">最近 100 场</option></select></div><h2>任务耗时趋势</h2>'+(svg||'<div class="empty"><p>完成有效任务后将显示趋势图。</p></div>')+'<div class="legend"><span style="color:#69adff">● 总用时</span><span style="color:#e68ce4">● 信标耗时</span></div></div>';
 $("chart-limit").value=String(state.chartLimit);$("chart-limit").onchange=e=>{state.chartLimit=Number(e.target.value);renderAnalytics()};
}
function renderOverlayPanel(){
 const el=$("overlay-panel");if(!el)return;
 if(!overlaySettings){el.innerHTML='<h2>游戏内 Overlay</h2><p class="muted">正在读取设置…</p><button data-action="overlay-refresh">重新读取</button>';return;}
 const o=overlaySettings.config,editing=overlaySettings.editing;
 const check=(key,label)=>'<label class="overlay-option"><input type="checkbox" data-overlay="'+key+'" '+(o[key]?"checked":"")+' '+(overlayBusy?"disabled":"")+'>'+label+'</label>';
 const range=(key,label,min,max)=>'<div class="overlay-range"><label for="overlay-'+key+'">'+label+'</label><output id="overlay-'+key+'-value">'+o[key]+'%</output><input id="overlay-'+key+'" type="range" min="'+min+'" max="'+max+'" step="5" data-overlay="'+key+'" value="'+o[key]+'" '+(overlayBusy?"disabled":"")+'></div>';
 el.innerHTML=(overlaySettings.error?'<p class="error">'+esc(overlaySettings.error)+'</p>':'')+'<div class="setting-row"><div><h2>游戏内 Overlay</h2><p>独立透明置顶卡片，正常游玩时鼠标穿透。</p></div><button role="switch" aria-checked="'+o.enabled+'" class="'+(o.enabled?"primary":"")+'" data-action="overlay-toggle" '+(overlayBusy?"disabled":"")+'>'+(o.enabled?"已开启":"已关闭")+'</button></div><div class="setting-row"><span>布局</span><select data-overlay="vertical" '+(overlayBusy?"disabled":"")+'><option value="false" '+(!o.vertical?"selected":"")+'>横向</option><option value="true" '+(o.vertical?"selected":"")+'>纵向</option></select></div>'+range("scale","窗口缩放",50,200)+range("opacity","背景不透明度",20,100)+'<div class="overlay-options">'+check("details","当前阶段分项")+check("compare","与个人最佳已完成分段对比")+check("auto_hide","非任务期间自动隐藏")+check("foreground_only","仅 Warframe 前台显示")+'</div><div class="about-links"><button class="primary" data-action="overlay-edit" '+(overlayBusy?"disabled":"")+'>'+(editing?"完成调整并保存":"调整位置与大小")+'</button><button data-action="overlay-reset" '+(overlayBusy?"disabled":"")+' >恢复默认</button></div><p class="muted">调整时拖动顶部，右下角缩放；关闭调整模式后鼠标穿透。支持窗口化和无边框全屏。实时显示为估算，最终成绩以日志为准。</p>';
}
async function refreshOverlaySettings(){
 if(overlayBusy||overlayReadBusy)return;overlayReadBusy=true;
 try{
  const value=await invoke("get_overlay_settings");
  if(!overlayBusy&&JSON.stringify(value)!==JSON.stringify(overlaySettings)){overlaySettings=value;renderOverlayPanel();}
 }catch(e){if($("overlay-panel")&&!overlaySettings)$("overlay-panel").innerHTML='<h2>游戏内 Overlay</h2><p class="error">'+esc(String(e))+'</p><button data-action="overlay-refresh">重试</button>'}
 finally{overlayReadBusy=false}
}
async function overlayAction(command,args={}){
 if(overlayBusy)return;
 overlayBusy=true;renderOverlayPanel();
 try{overlaySettings=await invoke(command,args);}
 catch(e){toast(String(e));}
 finally{overlayBusy=false;renderOverlayPanel();refreshOverlaySettings();}
}
document.addEventListener("input",e=>{if(e.target.matches('input[type="range"][data-overlay]')){const out=$("overlay-"+e.target.dataset.overlay+"-value");if(out)out.textContent=e.target.value+"%";}});
document.addEventListener("change",e=>{
 const t=e.target,k=t.dataset.overlay;if(!k||!overlaySettings||overlayBusy)return;
 const value=t.type==="checkbox"?t.checked:t.type==="range"?Number(t.value):t.value==="true";
 overlayAction("set_overlay_settings",{config:{...overlaySettings.config,[k]:value}});
});

function renderSettings(){
 $("content").innerHTML='<div class="settings"><article class="settings-card"><h2>常规</h2><div class="setting-row"><div><b>界面主题</b><p>深色 / 浅色</p></div><button class="primary" data-action="theme">'+(state.light?"浅色":"深色")+'</button></div><div class="setting-row"><div><b>阶段布局</b><p>双列卡片或单列卡片</p></div><button data-action="layout">'+(state.single?"单列":"双列")+'</button></div></article><article class="settings-card" id="overlay-panel"></article><article class="settings-card"><h2>日志监听</h2><p class="muted">默认读取 Windows 用户目录下的 Warframe / EE.log，启动时回放历史记录，随后持续监听。</p><div class="path-control"><input id="log-path" placeholder="EE.log 的绝对路径" value="'+esc(state.status.path||"")+'"><button class="primary" data-action="save-path">保存</button><button data-action="reset-path">恢复默认</button></div><div class="setting-row"><span>连接状态</span><b id="setting-status"></b></div><div class="setting-row"><span>读取位置</span><span id="setting-offset" class="muted"></span></div><div id="setting-error" class="error"></div></article><article class="settings-card"><h2><img src="logo.svg" alt="" width="32" height="32" style="vertical-align:middle;border-radius:6px;margin-right:8px">关于 PT Analytics Next</h2><p>v0.2.0 · MIT License</p><p>Aameow&ChatGPT 开发的高性能、稳定且可靠的大蜘蛛竞速工具，基于 Tauri2。</p><p class="muted">非 Warframe 官方应用。第三方游戏素材保留原权利。</p><div class="about-links"><button class="primary" data-action="github:repository">GitHub 仓库</button><button class="outline" data-action="github:author">Aameow 的 GitHub</button></div></article></div>';
 renderOverlayPanel();refreshOverlaySettings();
}
function openDialog(title,description,value,action){$("dialog-title").textContent=title;$("dialog-description").textContent=description;$("dialog-input").hidden=value===null;$("dialog-input").value=value??"";dialogAction=action;$("edit").showModal();if(value!==null){$("dialog-input").focus();$("dialog-input").select()}}
$("dialog-cancel").onclick=()=>$("edit").close();
$("edit-form").onsubmit=async e=>{e.preventDefault();try{await dialogAction?.($("dialog-input").value);$("edit").close();await refresh(true)}catch(err){toast(String(err))}};
async function action(a){
 const [cmd,arg]=a.split(":");const id=Number(arg),r=state.runs.find(x=>x.run_id===id);
 if(cmd==="overlay-refresh"){await refreshOverlaySettings();return}
 if(cmd==="overlay-toggle"&&overlaySettings){await overlayAction("set_overlay_settings",{config:{...overlaySettings.config,enabled:!overlaySettings.config.enabled}});return}
 if(cmd==="overlay-edit"&&overlaySettings){await overlayAction("edit_overlay",{editing:!overlaySettings.editing});return}
 if(cmd==="overlay-reset"){await overlayAction("reset_overlay");return}
 if(cmd==="github"){await invoke("open_github",{destination:arg});return}
 if(cmd==="page"){state.page=arg;render();return}
 if(cmd==="theme"){state.light=!state.light;localStorage.setItem("light",state.light?"1":"0");render();return}
 if(cmd==="layout"){state.single=!state.single;localStorage.setItem("single",state.single?"1":"0");render();return}
 if(cmd==="refresh"){await refresh(true);return}
 if(cmd==="older"||cmd==="newer"){const i=state.runs.findIndex(x=>x.run_id===selected()?.run_id);state.selected=state.runs[i+(cmd==="older"?1:-1)]?.run_id??state.selected;render();return}
 if(cmd==="view"){state.selected=id;state.page="home";render();return}
 if(cmd==="sort"){state.asc=state.sort===arg?!state.asc:false;state.sort=arg;state.pageNo=0;renderStorage();return}
 if(cmd==="table-prev"||cmd==="table-next"){state.pageNo+=cmd==="table-next"?1:-1;renderStorage();return}
 if(cmd==="rename"&&r){openDialog("修改任务名称","输入新的任务名称",r.run_name,name=>invoke("rename_run",{id,name}));return}
 if(cmd==="delete"&&r){openDialog("删除任务","确认删除「"+r.run_name+"」？删除后不会重新导入。",null,()=>invoke("delete_run",{id}));return}
 if(cmd==="delete-selected"){const ids=[...state.chosen];openDialog("删除任务","确认删除 "+ids.length+" 条任务记录？",null,async()=>{for(const id of ids)await invoke("delete_run",{id});state.chosen.clear()});return}
 if(cmd==="favorite"&&r){await invoke("favorite_run",{id});await refresh(true);return}
 if(cmd==="copy-image"&&r){await copyImage(r);return}
 if(cmd==="copy-text"&&r){await copyText(r);return}
 if(cmd==="save-path"||cmd==="reset-path"){await invoke("set_log_path",{path:cmd==="reset-path"?"":$("log-path").value});toast("日志路径已保存");await refresh(true);return}
}
document.addEventListener("click",e=>{const b=e.target.closest("[data-action]");if(b&&!b.disabled)action(b.dataset.action).catch(err=>toast(String(err)))});
document.addEventListener("change",e=>{if(e.target.matches("[data-select]")){const id=Number(e.target.dataset.select);e.target.checked?state.chosen.add(id):state.chosen.delete(id);renderStorage()}});
document.addEventListener("keydown",e=>{if(state.page==="home"&&!["INPUT","SELECT","TEXTAREA"].includes(e.target.tagName)&&!$("edit").open){if(e.key==="ArrowLeft")action("older");if(e.key==="ArrowRight")action("newer")}});
async function showStarPrompt(){
 try {
  if($("edit").open){setTimeout(showStarPrompt,1000);return;}
  if(await invoke("claim_star_prompt"))$("star-prompt").showModal();
 } catch(e){console.warn("Star prompt:",e);}
}
$("star-dismiss").onclick=()=>$("star-prompt").close();
$("star-visit").onclick=async()=>{
 try{await invoke("open_github",{destination:"repository"});$("star-prompt").close();}
 catch(e){toast(String(e));}
};
render();refresh(true);setInterval(()=>{refresh();if(state.page==="settings")refreshOverlaySettings()},750);
setTimeout(showStarPrompt,900);
