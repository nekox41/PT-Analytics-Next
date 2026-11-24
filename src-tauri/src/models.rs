// SPDX-License-Identifier: MIT
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Run {
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
    #[serde(default)]
    pub end_reason: Option<EndReason>,
    #[serde(default)]
    pub elapsed_seconds: Option<f64>,
    pub run_id: i32,
    pub time_stamp: i64,
    pub run_name: String,
    pub player_name: String,
    #[serde(default)]
    pub self_verified: bool,
    #[serde(default)]
    pub roster_version: u8,
    pub is_bugged_run: bool,
    pub is_aborted_run: bool,
    pub is_solo_run: bool,
    pub total_times: TotalTimes,
    pub phases: Vec<Phase>,
    pub squad_members: Vec<SquadMember>,
}
impl Run {
    pub fn new() -> Self { Self { roster_version: 1, ..Self::default() } }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Lifecycle { Pending, Running, Completed, Aborted, Interrupted, Invalid }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum EndReason { BossDefeated, MissionAbort, ReturnToTown, LogReset, PathChanged, SessionChanged, Recovery, MissingEvidence }
fn legacy_phase_complete() -> bool { true }

impl Run {
    pub fn normalize_legacy(&mut self) {
        if self.lifecycle.is_some() { return; }
        self.lifecycle=Some(if self.is_aborted_run { Lifecycle::Aborted } else if self.is_bugged_run || self.phases.len()!=4 { Lifecycle::Invalid } else { Lifecycle::Completed });
        // Legacy abort totals omit the active phase; do not present them as exact elapsed time.
        self.elapsed_seconds=if self.lifecycle==Some(Lifecycle::Completed) { Some(self.total_times.total_time) } else { None };
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalTimes {
    pub total_time: f64,
    pub total_flight_time: f64,
    pub total_shield_time: f64,
    pub total_leg_time: f64,
    pub total_body_time: f64,
    pub total_pylon_time: f64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phase {
    #[serde(default = "legacy_phase_complete")]
    pub complete: bool,
    pub phase_number: i32,
    pub total_time: f64,
    pub total_shield_time: f64,
    pub total_leg_time: f64,
    pub total_body_kill_time: f64,
    pub total_pylon_time: f64,
    pub shield_changes: Vec<ShieldChange>,
    pub leg_breaks: Vec<LegBreak>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldChange {
    pub shield_time: f64,
    pub status_effect: Damage,
    pub shield_order: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Damage { Impact, Puncture, Slash, Heat, Cold, Electric, Toxin, Blast, Radiation, Gas, Magnetic, Viral, Corrosive, NoShield }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegBreak {
    pub leg_break_time: f64,
    pub leg_position: Leg,
    pub leg_order: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Leg { FrontLeft, FrontRight, BackLeft, BackRight }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadMember { pub member_name: String }
