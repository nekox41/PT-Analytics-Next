const test=require("node:test"),assert=require("node:assert/strict");
const O=require("../ui/overlay-model.js");
const settings={config:{details:true,compare:true},editing:false};
function frame(){return {settings,age_ms:100,live:{elapsed:48,flight:11,phase:3,phase_elapsed:5,completed:[{phase_number:1,total_time:25},{phase_number:2,total_time:7}],active_metric:"shield",shield:"Toxin",shield_time:5,leg_time:null,body_time:null,pylon_time:null,legs:0},pb:{flight:12,cumulative:[38,46,75,82]}}}
test("only current stage and metric extrapolate; comparison uses locked splits",()=>{
 const v=O.derive(frame(),.2);
 assert.ok(Math.abs(v.total-48.3)<1e-8);assert.equal(v.stages[0].value,11);assert.equal(v.stages[1].value,25);assert.equal(v.stages[2].value,7);
 assert.ok(Math.abs(v.stages[3].value-5.3)<1e-8);assert.equal(v.stages[4].value,null);
 assert.equal(v.metrics[1].value,null);assert.equal(v.delta,-3);assert.equal(v.shield,"Toxin");
});
test("phase-specific rows do not fabricate unavailable mechanics",()=>{
 const f=frame();f.live.phase=2;f.live.active_metric="leg";f.live.leg_time=1;
 assert.deepEqual(O.derive(f).metrics.map(x=>x.key),["leg","body"]);
 f.live.phase=4;assert.deepEqual(O.derive(f).metrics.map(x=>x.key),["shield","leg","body"]);
});
test("stale snapshot is bounded; finished and pending times are fixed",()=>{
 const f=frame(),v=O.derive(f,100);
 assert.equal(v.total,48.75);assert.equal(v.stale,true);
 f.live.active_metric="finished";assert.equal(O.derive(f,100).total,48);
 f.live.active_metric="pending";f.live.elapsed=null;assert.equal(O.derive(f,100).total,null);
});
test("preview is independent of real task data, idle has no fake elapsed",()=>{
 const f=frame();f.live=null;assert.equal(O.derive(f).total,null);
 f.settings={...settings,editing:true};assert.equal(O.derive(f,100).total,48.326);
});
