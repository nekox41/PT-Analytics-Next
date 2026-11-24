const test=require("node:test");
const assert=require("node:assert/strict");
const R=require("../ui/report.js");
const makePhase=(phase_number,total_time,shield,leg,body,pylon,shield_changes=[],leg_breaks=[])=>({phase_number,complete:true,total_time,total_shield_time:shield,total_leg_time:leg,total_body_kill_time:body,total_pylon_time:pylon,shield_changes,leg_breaks});
const shields=(...s)=>s.map(([status_effect,shield_time])=>({status_effect,shield_time}));
const legs=(...s)=>s.map(([leg_position,leg_break_time])=>({leg_position,leg_break_time}));
function run(){return {run_id:2,run_name:"Run #2",player_name:"LocalTester",self_verified:true,squad_members:[],lifecycle:"completed",elapsed_seconds:93.882,total_times:{total_time:93.882,total_flight_time:14.020,total_shield_time:18.055,total_leg_time:22.536,total_body_time:0.570,total_pylon_time:19.545},phases:[
makePhase(1,26.672,7.636,2.406,.351,9.445,shields(["Puncture",7.586],["Magnetic",.049],["Radiation",0],["Viral",.001]),legs(["BackRight",1.713],["FrontRight",.158],["BackLeft",.376],["FrontLeft",.159])),
makePhase(2,16.962,0,14.650,.067,0,[],legs(["BackLeft",13.915],["FrontLeft",.216],["BackRight",.418],["FrontRight",.101])),
makePhase(3,24.121,4.078,2.095,.017,10.100,shields(["Blast",1.525],["Corrosive",1.833],["Gas",0],["Slash",.035],["Impact",.660],["Electric",.025]),legs(["BackRight",1.441],["FrontRight",.109],["BackLeft",.386],["FrontLeft",.159])),
makePhase(4,12.107,6.341,3.385,.135,0,shields(["Toxin",4.975],["Heat",1.290],["Cold",.075],["Puncture",0],["Magnetic",.001]),legs(["FrontLeft",1.190],["FrontRight",.477],["BackLeft",.794],["BackRight",.924]))
]};}
test("correct phase rows and cumulative values",()=>{
 const p=R.phases(run());
 assert.deepEqual(p.map(x=>x.rows.map(y=>y.type)),[["shield","leg","body","pylon"],["leg","body"],["shield","leg","body","pylon"],["shield","leg","body"]]);
 assert.deepEqual(p.map(x=>R.fmt(x.cumulative)),["40.692","57.654","81.775","93.882"]);
});
test("PTA-style exact report",()=>{
 const text=R.text(run());
 const expected=[
 "",
 "-".repeat(72),
 "Profit-Taker Run #2 by LocalTester cleared in 1m 33s 882ms",
 "",
 "From elevator to Profit-Taker took 14.020s. Fight duration: 1m 19s 862ms.",
 "",
 "> Phase 1 [26.672]",
 " Shield change:   7.636s - Puncture 7.586s | Magnetic 0.049s | Radiation 0.000s | Viral 0.001s",
 " Leg break:       2.406s - 1.713s | 0.158s | 0.376s | 0.159s",
 " Body killed:     0.351s",
 " Pylons:          9.445s",
 "",
 "> Phase 2 [16.962]",
 " Leg break:       14.650s - 13.915s | 0.216s | 0.418s | 0.101s",
 " Body killed:     0.067s",
 "",
 "> Phase 3 [24.121]",
 " Shield change:   4.078s - Blast 1.525s | Corrosive 1.833s | Gas 0.000s | Slash 0.035s | Impact 0.660s | Electric 0.025s",
 " Leg break:       2.095s - 1.441s | 0.109s | 0.386s | 0.159s",
 " Body killed:     0.017s",
 " Pylons:          10.100s",
 "",
 "> Phase 4 [12.107]",
 " Shield change:   6.341s - Toxin 4.975s | Heat 1.290s | Cold 0.075s | Puncture 0.000s | Magnetic 0.001s",
 " Leg break:       3.385s - 1.190s | 0.477s | 0.794s | 0.924s",
 " Body killed:     0.135s",
 "",
 "> Sum of parts [60.706]",
 " Shield change:  18.055s",
 " Leg Break:      22.536s",
 " Body Killed:    0.570s",
 " Pylons:         19.545s",
 "-".repeat(72),
 ""].join("\n");
 // PTA has a 72-character separator and fixed column widths.
 assert.equal(text,expected);
});
test("partial and multiplayer reports are not marked cleared",()=>{
 const r=run();r.lifecycle="interrupted";r.elapsed_seconds=40.5;r.phases=r.phases.slice(0,2);r.phases[1].complete=false;r.squad_members=[{member_name:"Peer"}];
 const t=R.text(r);assert.match(t,/interrupted after at least 0m 40s 500ms/);assert.match(t,/Participants: LocalTester \(self\) \| Peer/);assert.match(t,/Phase 2 \[16.962\] \(incomplete\)/);assert.doesNotMatch(t,/cleared in|> Phase 3/);
});
test("comparison is meaningful only between valid runs",()=>{
 const a=run(),b=run();b.elapsed_seconds=100;assert.equal(R.deltaText(R.metricValues(a,b)[0].delta),"-6.118s");
 assert.equal(R.metricValues(a,null)[0].delta,null);b.lifecycle="aborted";assert.equal(R.metricValues(a,b)[0].delta,null);
});
test("canvas export draws transparent gaps and fixed layout without DOM screenshot",()=>{
 const boxes=[],texts=[];const ctx={font:"",fillStyle:"",scale(){},beginPath(){},roundRect(...x){boxes.push(x)},fill(){},stroke(){},arc(){},moveTo(){},lineTo(){},measureText(s){return {width:String(s).length*7}},fillText(s){texts.push(s)}};
 const canvas={getContext(){return ctx}};
 global.document={createElement(){return canvas}};
 try{const result=R.canvas(run(),{light:true,icons:new Map()});assert.equal(result.width,2676);assert.ok(result.height>=978);assert.equal(boxes.length,16);assert.equal(boxes[0][0],0);assert.ok(boxes[2][0]>boxes[0][2]);assert.ok(texts.includes("93.882"));assert.ok(texts.includes("26.672s /"));assert.ok(!texts.includes("NaN"));}
 finally{delete global.document;}
});

test("all 13 shield types use original asset paths, no improvised glyphs",()=>{
 const keys=["Impact","Puncture","Slash","Heat","Cold","Electric","Toxin","Blast","Radiation","Gas","Magnetic","Viral","Corrosive"];
 for(const k of keys)assert.equal(R.damageIcon(k),"damage-icons/"+k+".png");
 assert.equal(R.damageIcon("NoShield"),null);assert.equal(R.damageIcon("../other"),null);
 const row=R.phases(run())[0].rows[0];assert.equal(row.splits[0].icon,"damage-icons/Puncture.png");assert.equal(row.splits[0].symbol,"Puncture");
});

test("export draws original image masks for shield splits rather than glyph text",()=>{
 const draws=[],texts=[],masks=[];
 const ctx={font:"",fillStyle:"",scale(){},beginPath(){},roundRect(){},fill(){},stroke(){},arc(){},moveTo(){},lineTo(){},measureText(s){return {width:String(s).length*7}},fillText(s){texts.push(s)},drawImage(...args){draws.push(args)}};
 const canvas={getContext(){return ctx}};
 let first=true;
 global.document={createElement(){
   if(first){first=false;return canvas}
   const m={drawImage(...args){masks.push(args)},fillRect(){}};
   return {getContext(){return m}};
 }};
 const icons=new Map(["Impact","Puncture","Slash","Heat","Cold","Electric","Toxin","Blast","Radiation","Gas","Magnetic","Viral","Corrosive"].map(k=>[k,{naturalWidth:32,naturalHeight:32,name:k}]));
 try{
  R.canvas(run(),{light:true,icons});
  assert.equal(draws.length,15);assert.equal(masks.length,15);
  assert.ok(masks.some(x=>x[0].name==="Puncture"));
  assert.ok(!texts.includes("Puncture"));
 }finally{delete global.document;}
});
