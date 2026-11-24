// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Run {
    pub run_id: i32,
    pub time_stamp: i64,
    pub run_name: String,
    pub player_name: String,
    pub is_bugged_run: bool,
    pub is_aborted_run: bool,
    pub is_solo_run: bool,
    pub total_times: TotalTimes,
    pub phases: Vec<Phase>,
    pub squad_members: Vec<SquadMember>,
}
impl Run {
    pub fn new() -> Self { Self { is_solo_run: true, ..Self::default() } }
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
