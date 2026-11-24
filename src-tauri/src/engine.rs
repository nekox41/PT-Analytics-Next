// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{constants::*, line_utils::get_log_time, models::Run, parser::events::parse_run, parser_state::ParserState};
use std::panic::{catch_unwind, AssertUnwindSafe};

pub struct Completed { pub key: String, pub run: Run }
pub struct Engine {
    state: ParserState,
    active: Option<(u64, Run)>,
    session: Option<i64>,
    pub warnings: u64,
}
impl Engine {
    pub fn new() -> Self {
        Self { state: ParserState::new(), active: None, session: None, warnings: 0 }
    }
    pub fn active(&self) -> bool { self.active.is_some() }
    pub fn reset(&mut self) { let warnings = self.warnings; *self = Self::new(); self.warnings = warnings; }
    fn discard(&mut self) {
        self.warnings += 1;
        self.active = None;
        self.state = ParserState::with_log_start_time(self.session.unwrap_or(0));
    }
    pub fn bad_line(&mut self) { self.warnings += 1; if let Some((_, run)) = &mut self.active { run.is_bugged_run = true; } }
    pub fn line(&mut self, offset: u64, bytes: &[u8]) -> Option<Completed> {
        let line = match std::str::from_utf8(bytes) {
            Ok(s) => s.trim_end_matches(['\r', '\n']),
            Err(_) => { self.bad_line(); return None; }
        };
        if line.contains(LOG_START_TIME) {
            if let Some(epoch) = get_log_time(line) {
                if self.session != Some(epoch) {
                    self.active = None;
                    self.state = ParserState::with_log_start_time(epoch);
                    self.session = Some(epoch);
                }
            } else { self.bad_line(); }
            return None;
        }
        if line.contains(HEIST_START) && self.active.is_none() {
            self.state = ParserState::with_log_start_time(self.session.unwrap_or(0));
            self.active = Some((offset, Run::new()));
        }
        let (_, run) = self.active.as_mut()?;
        // Only relevant event lines need a timestamp. Reject malformed relevant records
        // before entering the legacy parser; unexpected panics are isolated to one run.
        let relevant = [ELEVATOR_EXIT, SHIELD_SWITCH, LEG_KILL, BODY_VULNERABLE, STATE_CHANGE,
            PYLONS_LAUNCHED, PHASE_START, ABORT_MISSION, BACK_TO_TOWN].iter().any(|s| line.contains(*s))
            || SHIELD_PHASE_ENDINGS.iter().any(|s| line.contains(*s));
        if relevant && !line.split_whitespace().next().and_then(|s| s.trim_start_matches('!').parse::<f64>().ok()).is_some_and(|t| t.is_finite() && (0.0..=1e9).contains(&t)) {
            self.discard(); return None;
        }
        if catch_unwind(AssertUnwindSafe(|| parse_run(run, line, &mut self.state))).is_err() {
            self.discard(); return None;
        }
        if !self.state.run_ended { return None; }
        let (start, mut run) = self.active.take()?;
        self.state = ParserState::with_log_start_time(self.session.unwrap_or(0));
        // No fabricated timestamp or zero-start record; a subsequent valid run still works.
        if run.time_stamp <= 0 || self.session.is_none() { self.warnings += 1; return None; }
        if !run.total_times.total_time.is_finite() || run.total_times.total_time < 0.0 { self.warnings += 1; return None; }
        if run.phases.len() != 4 && !run.is_aborted_run { run.is_bugged_run = true; }
        Some(Completed { key: format!("{}:{}", self.session?, start), run })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "0.376 Sys [Diag]: Current time: Sun Nov 22 16:00:48 2026 [UTC: Sun Nov 22 08:00:48 2026]";
    fn feed(e: &mut Engine, line: &str) -> Option<Completed> { e.line(20, line.as_bytes()) }
    #[test]
    fn abort_replays_deterministically() {
        let replay = || {
            let mut e = Engine::new();
            feed(&mut e, HEADER);
            feed(&mut e, HEIST_START);
            feed(&mut e, &format!("10.1 {ELEVATOR_EXIT}"));
            feed(&mut e, &format!("11.1 {ABORT_MISSION}")).unwrap()
        };
        let a = replay(); let b = replay();
        assert!(a.run.is_aborted_run);
        assert_eq!(a.key, b.key);
        assert_eq!(a.run.time_stamp, b.run.time_stamp);
    }
    #[test]
    fn malformed_event_cannot_kill_next_run() {
        let mut e = Engine::new();
        feed(&mut e, HEADER); feed(&mut e, HEIST_START);
        feed(&mut e, &format!("10.0 {SHIELD_SWITCH} UNKNOWN"));
        assert!(!e.active());
        feed(&mut e, HEIST_START);
        feed(&mut e, &format!("20.0 {ELEVATOR_EXIT}"));
        assert!(feed(&mut e, &format!("21.0 {ABORT_MISSION}")).is_some());
        assert_eq!(e.warnings, 1);
    }
    #[test]
    fn full_four_phase_run() {
        let mut e = Engine::new();
        feed(&mut e, HEADER); feed(&mut e, HEIST_START);
        feed(&mut e, &format!("10.0 {ELEVATOR_EXIT}"));
        feed(&mut e, &format!("12.0 {PHASE_1_START}"));
        feed(&mut e, &format!("20.0 {PHASE_ENDS_1}"));
        feed(&mut e, &format!("25.0 {PHASE_ENDS_2}"));
        feed(&mut e, &format!("35.0 {PHASE_ENDS_3}"));
        for t in [36.0,37.0] { feed(&mut e, &format!("{t} {BODY_VULNERABLE}")); }
        let r = feed(&mut e, &format!("40.0 {BODY_VULNERABLE}")).unwrap().run;
        assert_eq!(r.phases.len(), 4);
        assert_eq!(r.total_times.total_time, 30.0);
        assert!(!r.is_aborted_run);
    }
    #[test]
    fn reset_clears_active_run_and_time_anchor() {
        let mut e = Engine::new();
        feed(&mut e, HEADER); feed(&mut e, HEIST_START);
        e.reset(); assert!(!e.active()); assert!(e.session.is_none());
    }
}
