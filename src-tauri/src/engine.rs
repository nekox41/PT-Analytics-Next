// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic, typed fight state machine. No log-derived panic paths.
use crate::{events::{self, Event, Observation}, models::*};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Completed { pub key: String, pub run: Run }

#[derive(Clone, Default)]
struct SplitClock {
    phase_start: f64,
    shield: Option<(Damage, f64)>,
    shield_closed: bool,
    last_leg: Option<f64>,
    vulnerable: Option<f64>,
    body_killed: Option<f64>,
    pylons: Option<f64>,
    vulnerability_count: u32,
    switches_during_pylons: u32,
}
#[derive(Clone)]
struct Fight {
    offset: u64,
    job_time: f64,
    run: Run,
    start: Option<f64>,
    last_time: Option<f64>,
    phase: Option<Phase>,
    clocks: SplitClock,
    initial_shield: Option<Damage>,
}
impl Fight {
    fn new(offset: u64, job_time: f64) -> Self {
        Self { offset, job_time, run: Run::new(), start:None, last_time:None, phase:None, clocks:SplitClock::default(), initial_shield:None }
    }
    fn mark(&mut self) { self.run.is_bugged_run=true; }
    fn close_shield(&mut self,t:f64) {
        if self.phase.is_none() || self.clocks.shield_closed { return; }
        if let (Some((damage,start)),Some(p))=(self.clocks.shield.take(),self.phase.as_mut()) {
            let duration=(t-start).max(0.0);
            p.total_shield_time+=duration;
            p.shield_changes.push(ShieldChange{shield_time:duration,status_effect:damage,shield_order:p.shield_changes.len() as i32+1});
        }
        self.clocks.shield_closed=true;
        self.clocks.last_leg=Some(t);
    }
    fn close_phase(&mut self,t:f64,complete:bool) {
        if let Some(mut p)=self.phase.take() {
            p.complete=complete;
            p.total_time=(t-self.clocks.phase_start).max(0.0);
            if let (Some(start),Some(end))=(self.clocks.vulnerable,self.clocks.body_killed) {
                p.total_body_kill_time=(end-start).max(0.0);
            }
            if let Some(start)=self.clocks.pylons { p.total_pylon_time=(t-start).max(0.0); }
            self.run.phases.push(p);
        }
    }
    fn begin_phase(&mut self,number:u8,t:f64) {
        if let Some(p)=&self.phase {
            if number as i32==p.phase_number { return; }
            if (number as i32)<p.phase_number { self.mark(); return; }
            if number as i32!=p.phase_number+1 { self.mark(); }
            self.close_phase(t,true);
        } else {
            if number!=1 { self.mark(); }
            if let Some(start)=self.start { self.run.total_times.total_flight_time=(t-start).max(0.0); } else { self.mark(); }
        }
        self.phase=Some(Phase{phase_number:number as i32,..Phase::default()});
        self.clocks=SplitClock { phase_start:t,last_leg:Some(t),shield:self.initial_shield.take().map(|d|(d,t)),..SplitClock::default() };
    }
    fn shield(&mut self,d:Damage,t:f64) {
        let Some(number)=self.phase.as_ref().map(|p|p.phase_number) else { self.initial_shield=Some(d); return; };
        if self.clocks.shield_closed || self.clocks.pylons.is_some() {
            // The next shield may already be advertised during phase-three pylons.
            self.initial_shield=Some(d);
            if number==3 && self.clocks.pylons.is_some() {
                self.clocks.switches_during_pylons=self.clocks.switches_during_pylons.saturating_add(1);
                if self.clocks.switches_during_pylons>1 {
                    self.mark(); self.begin_phase(4,t);
                }
            }
            return;
        }
        if let Some((previous,from))=self.clocks.shield.replace((d,t)) {
            if let Some(p)=self.phase.as_mut() {
                let delta=(t-from).max(0.0);
                p.total_shield_time+=delta;
                p.shield_changes.push(ShieldChange{shield_time:delta,status_effect:previous,shield_order:p.shield_changes.len() as i32+1});
            }
        }
    }
    fn leg(&mut self,leg:Leg,t:f64) {
        if self.phase.is_none() { self.mark(); return; }
        if !self.clocks.shield_closed && self.clocks.shield.is_some() { self.mark(); self.close_shield(t); }
        let Some(p)=self.phase.as_mut() else { return; };
        if p.leg_breaks.iter().any(|s|s.leg_position==leg) { self.run.is_bugged_run=true; }
        let from=self.clocks.last_leg.replace(t).unwrap_or(self.clocks.phase_start);
        let delta=(t-from).max(0.0);
        p.total_leg_time+=delta;
        p.leg_breaks.push(LegBreak{leg_break_time:delta,leg_position:leg,leg_order:p.leg_breaks.len() as i32+1});
    }
    fn observe(&mut self,obs:Observation,epoch:Option<i64>)->Option<EndReason> {
        let t=obs.time;
        // Squad/name messages do not advance the combat clock.
        match obs.event {
            Event::Player(name)=>{
                if self.run.player_name.is_empty() { self.run.player_name=name; }
                self.run.squad_members.retain(|m|m.member_name!=self.run.player_name);
                self.run.is_solo_run=self.run.squad_members.is_empty(); return None;
            }
            Event::Member(name)=>{
                if name!=self.run.player_name && self.run.squad_members.len()<3 && !self.run.squad_members.iter().any(|m|m.member_name==name) {
                    self.run.squad_members.push(SquadMember{member_name:name}); self.run.is_solo_run=false;
                } return None;
            }
            Event::Job|Event::Session(_)=>return None,
            _=>{}
        }
        if self.last_time.is_some_and(|last|t<last) { self.mark(); return None; }
        self.last_time=Some(t);
        match obs.event {
            Event::Elevator=>{
                if self.start.is_none() {
                    self.start=Some(t);
                    if let Some(epoch)=epoch { self.run.time_stamp=epoch+t as i64; }
                }
            }
            Event::Phase(n)=>self.begin_phase(n,t),
            Event::Shield(d)=>self.shield(d,t),
            Event::ShieldDone=>self.close_shield(t),
            Event::Leg(leg)=>self.leg(leg,t),
            Event::Vulnerable=>{
                if self.phase.is_none() { self.mark(); return None; }
                self.clocks.vulnerable.get_or_insert(t);
                self.clocks.vulnerability_count=self.clocks.vulnerability_count.saturating_add(1);
                if self.phase.as_ref().is_some_and(|p|p.phase_number==4) && self.clocks.vulnerability_count>=3 {
                    self.clocks.body_killed=Some(t); self.close_phase(t,true); return Some(EndReason::BossDefeated);
                }
            }
            Event::BodyKilled=>{if self.clocks.vulnerable.is_some(){self.clocks.body_killed.get_or_insert(t);}}
            Event::Pylons=>{self.clocks.pylons.get_or_insert(t);}
            Event::Abort=>return Some(EndReason::MissionAbort),
            Event::ReturnToTown=>return Some(EndReason::ReturnToTown),
            _=>{}
        }
        None
    }
    fn complete(mut self,epoch:i64,reason:EndReason)->Option<Completed> {
        let t=self.last_time.unwrap_or(self.job_time);
        let interrupted=matches!(reason,EndReason::LogReset|EndReason::PathChanged|EndReason::SessionChanged|EndReason::Recovery);
        let aborted=matches!(reason,EndReason::MissionAbort|EndReason::ReturnToTown);
        if let Some(start)=self.start {
            self.run.time_stamp=epoch+start as i64;
            self.run.elapsed_seconds=Some((t-start).max(0.0));
            if self.phase.is_some() { self.close_phase(t,false); }
            else if self.run.phases.is_empty() { self.run.total_times.total_flight_time=(t-start).max(0.0); }
        } else {
            self.run.time_stamp=epoch+self.job_time as i64;
            self.run.elapsed_seconds=None;
        }
        if self.run.time_stamp<=0 { return None; }
        if reason==EndReason::BossDefeated && !self.run.phases.iter().filter(|p|p.complete).map(|p|p.phase_number).eq(1..=4) { self.mark(); }
        self.run.lifecycle=Some(if aborted { Lifecycle::Aborted } else if interrupted { Lifecycle::Interrupted }
            else if self.start.is_none() || self.run.is_bugged_run { Lifecycle::Invalid } else { Lifecycle::Completed });
        self.run.end_reason=Some(reason);
        self.run.is_aborted_run=aborted;
        let totals=&mut self.run.total_times;
        totals.total_time=self.run.elapsed_seconds.unwrap_or(0.0);
        for p in &self.run.phases {
            totals.total_shield_time+=p.total_shield_time;
            totals.total_leg_time+=p.total_leg_time;
            totals.total_body_time+=p.total_body_kill_time;
            totals.total_pylon_time+=p.total_pylon_time;
        }
        if !totals.total_time.is_finite() || totals.total_time<0.0 { return None; }
        Some(Completed{key:format!("{epoch}:{}",self.offset),run:self.run})
    }
}

pub struct Engine { epoch:Option<i64>, fight:Option<Fight>, pub warnings:u64 }
impl Engine {
    pub fn new()->Self { Self { epoch:None,fight:None,warnings:0 } }
    pub fn active(&self)->bool { self.fight.is_some() }
    pub fn lifecycle(&self)->Option<Lifecycle> { self.fight.as_ref().map(|f|if f.start.is_some(){Lifecycle::Running}else{Lifecycle::Pending}) }
    pub fn snapshot(&self)->Option<Completed> { self.fight.clone()?.complete(self.epoch?,EndReason::Recovery) }
    pub fn reset(&mut self, reason:EndReason)->Option<Completed> {
        let result=self.fight.take().zip(self.epoch).and_then(|(f,epoch)|f.complete(epoch,reason));
        self.epoch=None;
        result
    }
    pub fn bad_line(&mut self) {
        self.warnings=self.warnings.saturating_add(1);
        if let Some(f)=&mut self.fight { f.mark(); }
    }
    pub fn line(&mut self,offset:u64,bytes:&[u8])->Option<Completed> {
        let line=match std::str::from_utf8(bytes) { Ok(s)=>s.trim_end_matches(['\r','\n']),Err(_)=>{self.bad_line();return None;} };
        let observation=match events::decode(line) { Ok(Some(e))=>e,Ok(None)=>return None,Err(_)=>{self.bad_line();return None;} };
        match &observation.event {
            Event::Session(epoch)=>{
                if self.epoch!=Some(*epoch) {
                    let previous=self.reset(EndReason::SessionChanged);
                    self.epoch=Some(*epoch);
                    return previous;
                }
                return None;
            }
            Event::Job=>{
                if self.fight.is_none() { self.fight=Some(Fight::new(offset,observation.time)); }
                return None;
            }
            _=>{}
        }
        if let Some(reason)=self.fight.as_mut()?.observe(observation,self.epoch) {
            let fight=self.fight.take()?;
            let result=self.epoch.and_then(|epoch|fight.complete(epoch,reason));
            if result.is_none(){self.warnings=self.warnings.saturating_add(1);}
            result
        } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{HEADER,JOB,EXIT,ABORT,PHASE_MARKERS,SHIELD,LEG,VULNERABLE,PYLONS};
    fn header(e:&mut Engine) { e.line(0,format!("0.376 {HEADER} Sun Nov 22 16:00:48 2026 [UTC: Sun Nov 22 08:00:48 2026]").as_bytes()); }
    fn feed(e:&mut Engine,t:f64,s:&str)->Option<Completed> { e.line((t*1000.0) as u64,format!("{t:.3} {s}").as_bytes()) }
    #[test]
    fn full_four_phase_replay() {
        let mut e=Engine::new();header(&mut e);
        feed(&mut e,9.0,JOB);feed(&mut e,10.0,EXIT);
        feed(&mut e,12.0,PHASE_MARKERS[0]);
        feed(&mut e,12.2,&format!("{SHIELD} DT_FIRE"));
        feed(&mut e,13.0,&format!("{SHIELD} DT_VIRAL"));
        feed(&mut e,14.0,"ResourceLoader DBntyFourInterPrTk0920TheBusiness");
        feed(&mut e,15.0,&format!("{LEG}: ARM_RIGHT"));
        feed(&mut e,16.0,VULNERABLE);
        feed(&mut e,17.0,"CamperHeistOrbFight.lua: Landscape - New State: 3");
        feed(&mut e,18.0,PYLONS);
        feed(&mut e,20.0,PHASE_MARKERS[1]);
        feed(&mut e,25.0,PHASE_MARKERS[2]);
        feed(&mut e,35.0,PHASE_MARKERS[3]);
        feed(&mut e,36.0,VULNERABLE);feed(&mut e,37.0,VULNERABLE);
        let result=feed(&mut e,40.0,VULNERABLE).unwrap();
        assert_eq!(result.run.phases.len(),4);
        assert_eq!(result.run.total_times.total_time,30.0);
        assert!((result.run.total_times.total_shield_time-1.8).abs()<1e-9);
        assert_eq!(result.run.total_times.total_leg_time,1.0);
        assert_eq!(result.run.total_times.total_pylon_time,2.0);
        assert!(!result.run.is_aborted_run);assert!(!e.active());
    }
    #[test]
    fn malformed_lines_do_not_end_the_worker() {
        let mut e=Engine::new();header(&mut e);feed(&mut e,1.0,JOB);
        feed(&mut e,2.0,EXIT);e.line(30,b"\xff");feed(&mut e,3.0,&format!("{SHIELD} DT_UNKNOWN"));
        let first=feed(&mut e,4.0,ABORT).unwrap();assert!(first.run.is_bugged_run);
        feed(&mut e,5.0,JOB);feed(&mut e,6.0,EXIT);
        let second=feed(&mut e,7.0,ABORT).unwrap();
        assert!(second.run.is_aborted_run);assert!(!second.run.is_bugged_run);assert_eq!(e.warnings,2);
    }
    #[test]
    fn abort_mid_phase_keeps_elapsed_and_partial_splits() {
        let mut e=Engine::new();header(&mut e);
        feed(&mut e,1.0,JOB);feed(&mut e,10.0,EXIT);feed(&mut e,20.0,PHASE_MARKERS[0]);
        feed(&mut e,35.0,PHASE_MARKERS[1]);feed(&mut e,40.0,&format!("{LEG}: ARM_RIGHT"));
        let r=feed(&mut e,60.0,ABORT).unwrap().run;
        assert_eq!(r.lifecycle,Some(Lifecycle::Aborted));assert_eq!(r.elapsed_seconds,Some(50.0));
        assert_eq!(r.phases.len(),2);assert!(r.phases[0].complete);assert!(!r.phases[1].complete);
        assert_eq!(r.phases[1].total_time,25.0);assert_eq!(r.phases[1].total_leg_time,5.0);
    }
    #[test]
    fn abort_in_flight_and_before_start() {
        let mut e=Engine::new();header(&mut e);feed(&mut e,1.0,JOB);
        let r=feed(&mut e,2.0,ABORT).unwrap().run;assert_eq!(r.elapsed_seconds,None);
        feed(&mut e,3.0,JOB);feed(&mut e,4.0,EXIT);
        let r=feed(&mut e,10.0,ABORT).unwrap().run;assert_eq!(r.elapsed_seconds,Some(6.0));assert_eq!(r.total_times.total_flight_time,6.0);
    }
    #[test]
    fn reset_uses_last_trusted_event_not_wall_clock() {
        let mut e=Engine::new();header(&mut e);feed(&mut e,1.0,JOB);feed(&mut e,10.0,EXIT);
        feed(&mut e,20.0,PHASE_MARKERS[0]);feed(&mut e,23.0,&format!("{LEG}: ARM_RIGHT"));
        let snap=e.snapshot().unwrap();let r=e.reset(EndReason::LogReset).unwrap();
        assert_eq!(snap.key,r.key);assert_eq!(r.run.elapsed_seconds,Some(13.0));assert!(!r.run.phases[0].complete);
        assert_eq!(r.run.lifecycle,Some(Lifecycle::Interrupted));
    }
    #[test]
    fn startup_replay_and_live_append_produce_same_result() {
        use crate::tail::{Framer,Line};
        let log=format!("0.376 {HEADER} Sun Nov 22 16:00:48 2026 [UTC: Sun Nov 22 08:00:48 2026]\n1.0 {JOB}\n10.0 {EXIT}\n20.0 {}\n25.0 {ABORT}\n",PHASE_MARKERS[0]);
        let parse=|chunk:usize| {
            let mut f=Framer::new();let mut e=Engine::new();let mut result=None;
            for part in log.as_bytes().chunks(chunk) {
                for line in f.push(part) {if let Line::Complete(offset,bytes)=line {if let Some(r)=e.line(offset,&bytes){result=Some(r);}}}
            }
            result.unwrap()
        };
        let a=parse(log.len());let b=parse(7);
        assert_eq!(a.key,b.key);assert_eq!(a.run.elapsed_seconds,Some(15.0));assert_eq!(a.run.elapsed_seconds,b.run.elapsed_seconds);
        assert_eq!(serde_json::to_string(&a.run).unwrap(),serde_json::to_string(&b.run).unwrap());
    }
    #[test]
    fn reset_archives_pending_fight() {
        let mut e=Engine::new();header(&mut e);feed(&mut e,1.0,JOB);
        let r=e.reset(EndReason::LogReset).unwrap().run;
        assert_eq!(r.lifecycle,Some(Lifecycle::Interrupted));assert_eq!(r.elapsed_seconds,None);
        assert!(!e.active());assert!(e.epoch.is_none());
    }
}
