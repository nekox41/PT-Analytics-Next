// SPDX-License-Identifier: GPL-3.0-or-later
//! Translate external log text into typed observations without mutating fight state.
use crate::models::{Damage, Leg};
use chrono::{Local, NaiveDateTime, TimeZone};

pub const HEADER: &str = "Sys [Diag]: Current time:";
pub const JOB: &str = "jobId=/Lotus/Types/Gameplay/Venus/Jobs/Heists/HeistProfitTakerBountyFour";
const JOB_PATH: &str = "/Lotus/Types/Gameplay/Venus/Jobs/Heists/HeistProfitTakerBountyFour";
pub const EXIT: &str = "EidolonMP.lua: EIDOLONMP: Avatar left the zone";
pub const ABORT: &str = "GameRulesImpl - changing state from SS_STARTED to SS_ENDING";
pub const TOWN: &str = "EidolonMP.lua: EIDOLONMP: TryTownTransition";
pub const PHASE_MARKERS: [&str;4] = [
    "Orb Fight - Starting first attack Orb phase",
    "Orb Fight - Starting second attack Orb phase",
    "Orb Fight - Starting third attack Orb phase",
    "Orb Fight - Starting final attack Orb phase",
];
pub const VULNERABLE: &str = "Camper->StartVulnerable() - The Camper can now be damaged!";
pub const SHIELD: &str = "SwitchShieldVulnerability";
pub const LEG: &str = "Leg freshly destroyed at part";
pub const BODY_STATE: &str = "CamperHeistOrbFight.lua: Landscape - New State: ";
pub const PYLONS: &str = "Pylon launch complete";

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Session(i64), Job, Elevator, Phase(u8), Shield(Damage), ShieldDone,
    Leg(Leg), Vulnerable, BodyKilled, Pylons, Abort, ReturnToTown, Player(String), Member(String),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Observation { pub time: f64, pub event: Event }

fn seconds(line: &str) -> Result<f64, &'static str> {
    line.split_whitespace().next().and_then(|s| s.trim_start_matches('!').parse::<f64>().ok())
        .filter(|t| t.is_finite() && (0.0..=1e9).contains(t)).ok_or("invalid event timestamp")
}
fn epoch(line: &str, time: f64) -> Option<i64> {
    let (_, rest) = line.split_once(HEADER)?;
    let (value, utc) = match rest.split_once("[UTC:") {
        Some((_, u)) => (u.trim().trim_end_matches(']').trim(), true),
        None => (rest.trim(), false),
    };
    let dt = NaiveDateTime::parse_from_str(value, "%a %b %e %H:%M:%S %Y").ok()?;
    let wall = if utc { dt.and_utc().timestamp() } else { Local.from_local_datetime(&dt).single()?.timestamp() };
    Some(wall - time.round() as i64)
}
fn damage(s: &str) -> Option<Damage> {
    Some(match s {
        "DT_IMPACT"=>Damage::Impact, "DT_PUNCTURE"=>Damage::Puncture,
        "DT_SLASH"=>Damage::Slash, "DT_FIRE"=>Damage::Heat, "DT_FREEZE"=>Damage::Cold,
        "DT_ELECTRICITY"=>Damage::Electric, "DT_POISON"=>Damage::Toxin,
        "DT_EXPLOSION"=>Damage::Blast, "DT_RADIATION"=>Damage::Radiation,
        "DT_GAS"=>Damage::Gas, "DT_MAGNETIC"=>Damage::Magnetic,
        "DT_VIRAL"=>Damage::Viral, "DT_CORROSIVE"=>Damage::Corrosive, _=>return None,
    })
}
fn leg(s: &str) -> Option<Leg> {
    Some(match s {
        "ARM_RIGHT"=>Leg::FrontLeft, "ARM_LEFT"=>Leg::FrontRight,
        "LEG_RIGHT"=>Leg::BackLeft, "LEG_LEFT"=>Leg::BackRight, _=>return None,
    })
}
fn clean_name(s: &str) -> Option<String> {
    let s = s.split(['\u{e000}', '\u{e002}']).next()?.trim();
    if s.is_empty() || s.chars().count() > 128 { None } else { Some(s.to_owned()) }
}
pub fn decode(line: &str) -> Result<Option<Observation>, &'static str> {
    let event = if line.contains(HEADER) {
        let time = seconds(line)?;
        return Ok(Some(Observation { time, event: Event::Session(epoch(line, time).ok_or("invalid log session clock")?) }));
    } else if line.contains(ABORT) || line.contains("TopMenu.lua: Abort: host/no session") || line.contains("TopMenu.lua: Abort: client") { Event::Abort }
    else if line.contains(TOWN) { Event::ReturnToTown }
    else if line.trim_start().starts_with(JOB) { return Ok(None); } // multiline configuration, not a timed event
    else if line.contains(JOB) || (line.contains(JOB_PATH) && (line.contains("ThemedSquadOverlay.lua: Active jobId set to ") || line.contains("EidolonMissions.lua - ActiveJob is "))) { Event::Job }
    else if line.contains(EXIT) { Event::Elevator }
    else if let Some(i) = PHASE_MARKERS.iter().position(|p| line.contains(*p)) { Event::Phase((i+1) as u8) }
    else if line.contains(SHIELD) {
        Event::Shield(damage(line.split_whitespace().last().unwrap_or("")).ok_or("unknown shield damage type")?)
    } else if line.contains("ResourceLoader") &&
        ["DBntyFourInterPrTk0920TheBusiness","DBntyFourInterPrTk0890TheBusiness","DBntyFourSatelReal0930TheBusiness"].iter().any(|p| line.contains(*p)) { Event::ShieldDone }
    else if line.contains(LEG) {
        Event::Leg(leg(line.split_whitespace().last().unwrap_or("")).ok_or("unknown leg position")?)
    } else if line.contains(VULNERABLE) { Event::Vulnerable }
    else if let Some((_, rest)) = line.split_once(BODY_STATE) {
        match rest.split_whitespace().next().and_then(|s| s.parse::<u8>().ok()) {
            Some(3|5|6) => Event::BodyKilled, Some(_) => return Ok(None), None => return Err("invalid body state"),
        }
    } else if line.contains(PYLONS) { Event::Pylons }
    else if let Some((_, rest)) = line.split_once("Net [Info]: name: ") {
        Event::Player(rest.split_whitespace().next().and_then(clean_name).ok_or("invalid player name")?)
    } else if line.contains("loadout loader finished.") {
        Event::Member(line.split_whitespace().nth(3).and_then(clean_name).ok_or("invalid member name")?)
    } else { return Ok(None); };
    Ok(Some(Observation { time: seconds(line)?, event }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_input_is_a_value_not_a_panic() {
        assert!(decode("ordinary unstructured text").unwrap().is_none());
        assert!(decode(&format!("    {JOB}_suffix")).unwrap().is_none());
        assert!(decode("152.875 Game [Info]: SetReturnToLobbyLevelArgs: /Lotus/Levels/Proc/TheNewWar/PartTwo/PostWarFortuna/DgA.lp").unwrap().is_none());
        assert!(matches!(decode(&format!("142.168 Script [Info]: ThemedSquadOverlay.lua: Active jobId set to {JOB_PATH}_suffix")).unwrap().unwrap().event,Event::Job));
        assert!(decode("abc SwitchShieldVulnerability DT_FIRE").is_err());
        assert!(decode("1.0 SwitchShieldVulnerability DT_FUTURE").is_err());
        assert!(decode("1.0 Leg freshly destroyed at part: LEG_LEFT").unwrap().is_some());
        assert!(decode("NaN Orb Fight - Starting first attack Orb phase").is_err());
    }
    #[test]
    fn utc_clock_ignores_machine_timezone() {
        let r=decode("0.376 Sys [Diag]: Current time: Sun Nov 22 16:00:48 2026 [UTC: Sun Nov 22 08:00:48 2026]").unwrap().unwrap();
        assert_eq!(r.event, Event::Session(chrono::DateTime::parse_from_rfc3339("2026-11-22T08:00:48Z").unwrap().timestamp()));
    }
}
