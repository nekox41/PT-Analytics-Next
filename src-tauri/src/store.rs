// SPDX-License-Identifier: MIT
use crate::{engine::Completed, models::Run};
use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct Record { #[serde(flatten)] pub run: Run, pub favorite: bool }
pub struct Store { conn: Connection }
impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=3000;
            CREATE TABLE IF NOT EXISTS runs(id INTEGER PRIMARY KEY AUTOINCREMENT, source_key TEXT NOT NULL UNIQUE,
                stamp INTEGER NOT NULL, payload TEXT NOT NULL, name TEXT, favorite INTEGER NOT NULL DEFAULT 0,
                deleted INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);").map_err(err)?;
        Ok(Self { conn })
    }
    pub fn insert(&self, item: &Completed) -> Result<(), String> {
        let json = serde_json::to_string(&item.run).map_err(err)?;
        self.conn.execute("INSERT INTO runs(source_key,stamp,payload) VALUES (?1,?2,?3) ON CONFLICT(source_key) DO UPDATE SET stamp=excluded.stamp,payload=excluded.payload WHERE runs.deleted=0 AND ((json_extract(runs.payload, '$.lifecycle')='interrupted' AND (json_extract(excluded.payload, '$.lifecycle') IN ('completed','aborted','invalid') OR coalesce(json_extract(excluded.payload, '$.elapsed_seconds'),-1)>coalesce(json_extract(runs.payload, '$.elapsed_seconds'),-1))) OR (json_extract(runs.payload, '$.lifecycle') IS NULL AND json_extract(runs.payload, '$.is_aborted_run')=1 AND json_extract(excluded.payload, '$.lifecycle')='aborted' AND json_extract(excluded.payload, '$.elapsed_seconds') IS NOT NULL))",
            params![item.key, item.run.time_stamp, json]).map_err(err)?;
        // Replay upgrades roster metadata only. Timings, favorites, tombstones and custom names stay unchanged.
        if item.run.roster_version>0 && (item.run.self_verified || !item.run.squad_members.is_empty()) {
            self.conn.execute("UPDATE runs SET payload=json_set(payload,'$.player_name',json_extract(?1,'$.player_name'),'$.self_verified',json(?2),'$.is_solo_run',json(?3),'$.squad_members',json_extract(?1,'$.squad_members'),'$.roster_version',json_extract(?1,'$.roster_version')) WHERE source_key=?4 AND deleted=0 AND coalesce(json_extract(payload,'$.roster_version'),0)<json_extract(?1,'$.roster_version')",params![json,if item.run.self_verified {"true"}else{"false"},if item.run.is_solo_run {"true"}else{"false"},item.key]).map_err(err)?;
        }
        Ok(())
    }
    pub fn list(&self) -> Result<Vec<Record>, String> {
        let mut stmt = self.conn.prepare("SELECT id,payload,name,favorite FROM runs WHERE deleted=0 ORDER BY stamp DESC,id DESC").map_err(err)?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_,i32>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,bool>(3)?))).map_err(err)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, payload, name, favorite) = row.map_err(err)?;
            let mut run: Run = serde_json::from_str(&payload).map_err(err)?;
            run.normalize_legacy(); run.run_id=id; run.run_name=name.unwrap_or_else(|| format!("Run #{}", id));
            out.push(Record { run, favorite });
        }
        Ok(out)
    }
    pub fn rename(&self, id: i32, name: &str) -> Result<(), String> {
        self.conn.execute("UPDATE runs SET name=?1 WHERE id=?2 AND deleted=0", params![name,id]).map_err(err)?; Ok(())
    }
    pub fn favorite(&self, id: i32) -> Result<(), String> {
        self.conn.execute("UPDATE runs SET favorite=1-favorite WHERE id=?1 AND deleted=0", [id]).map_err(err)?; Ok(())
    }
    pub fn delete(&self, id: i32) -> Result<(), String> {
        // Retain the source key as a tombstone to prevent reimport on startup.
        self.conn.execute("UPDATE runs SET deleted=1 WHERE id=?1", [id]).map_err(err)?; Ok(())
    }
    // The active fight snapshot is distinct from terminal history, so an ordinary
    // application restart can replay the same log without fabricating an abort.
    pub fn checkpoint(&self) -> Result<Option<Completed>, String> {
        use rusqlite::OptionalExtension;
        let value:Option<String>=self.conn.query_row("SELECT value FROM settings WHERE key='checkpoint'", [], |r| r.get(0)).optional().map_err(err)?;
        value.map(|s|serde_json::from_str(&s).map_err(err)).transpose()
    }
    pub fn save_checkpoint(&self, item:Option<&Completed>) -> Result<(),String> {
        if let Some(item)=item {
            let json=serde_json::to_string(item).map_err(err)?;
            self.conn.execute("INSERT INTO settings(key,value) VALUES ('checkpoint',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value WHERE value<>excluded.value",[json]).map_err(err)?;
        } else { self.conn.execute("DELETE FROM settings WHERE key='checkpoint'",[]).map_err(err)?; }
        Ok(())
    }
    pub fn reconcile_checkpoint(&mut self, current:Option<&Completed>) -> Result<(),String> {
        let previous=self.checkpoint()?;
        let tx=self.conn.transaction().map_err(err)?;
        if let Some(old)=previous.filter(|old|current.is_none_or(|new|new.key!=old.key)) {
            let json=serde_json::to_string(&old.run).map_err(err)?;
            tx.execute("INSERT OR IGNORE INTO runs(source_key,stamp,payload) VALUES (?1,?2,?3)",params![old.key,old.run.time_stamp,json]).map_err(err)?;
        }
        if let Some(item)=current {
            let json=serde_json::to_string(item).map_err(err)?;
            tx.execute("INSERT INTO settings(key,value) VALUES ('checkpoint',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value WHERE value<>excluded.value",[json]).map_err(err)?;
        } else { tx.execute("DELETE FROM settings WHERE key='checkpoint'",[]).map_err(err)?; }
        tx.commit().map_err(err)
    }
    pub fn register_launch(&self) -> Result<(),String> {
        self.conn.execute("INSERT INTO settings(key,value) VALUES ('app_launch_count','1') ON CONFLICT(key) DO UPDATE SET value=CAST(MIN(10,MAX(0,CAST(value AS INTEGER))+1) AS TEXT)",[]).map_err(err)?;
        Ok(())
    }
    // Atomic claim prevents duplicate prompts across reloads and multiple processes.
    pub fn claim_star_prompt(&self) -> Result<bool,String> {
        let changed=self.conn.execute("INSERT OR IGNORE INTO settings(key,value) SELECT 'star_prompt_shown','1' WHERE EXISTS (SELECT 1 FROM settings WHERE key='app_launch_count' AND CAST(value AS INTEGER)>=10)",[]).map_err(err)?;
        Ok(changed==1)
    }
    pub fn path(&self) -> Option<String> {
        self.conn.query_row("SELECT value FROM settings WHERE key='log_path'", [], |r| r.get(0)).ok()
    }
    pub fn set_path(&self, path: &str) -> Result<(), String> {
        self.conn.execute("INSERT INTO settings(key,value) VALUES ('log_path',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [path]).map_err(err)?; Ok(())
    }
}
fn err(e: impl std::fmt::Display) -> String { e.to_string() }
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn star_prompt_only_once_on_tenth_launch_and_persists() {
        let d=tempfile::tempdir().unwrap(); let path=d.path().join("test.sqlite3");
        for _ in 0..9 {
            let s=Store::open(&path).unwrap();s.register_launch().unwrap();
            assert!(!s.claim_star_prompt().unwrap());
        }
        {
            let s=Store::open(&path).unwrap();s.register_launch().unwrap();
            assert!(s.claim_star_prompt().unwrap());assert!(!s.claim_star_prompt().unwrap());
        }
        for _ in 0..3 {
            let s=Store::open(&path).unwrap();s.register_launch().unwrap();
            assert!(!s.claim_star_prompt().unwrap());
        }
    }
    #[test]
    fn checkpoint_is_not_history_until_session_is_lost() {
        use crate::models::{Lifecycle,EndReason};
        let mut s=Store::open(Path::new(":memory:")).unwrap();
        let mut item=Completed{key:"old:1".into(),run:Run::new()};
        item.run.lifecycle=Some(Lifecycle::Interrupted);item.run.end_reason=Some(EndReason::Recovery);item.run.elapsed_seconds=Some(12.0);
        s.save_checkpoint(Some(&item)).unwrap();
        s.reconcile_checkpoint(Some(&item)).unwrap();
        assert!(s.list().unwrap().is_empty());
        s.reconcile_checkpoint(None).unwrap();
        assert_eq!(s.list().unwrap()[0].run.lifecycle,Some(Lifecycle::Interrupted));
        item.run.lifecycle=Some(Lifecycle::Completed);item.run.elapsed_seconds=Some(30.0);
        s.insert(&item).unwrap();assert_eq!(s.list().unwrap().len(),1);assert_eq!(s.list().unwrap()[0].run.lifecycle,Some(Lifecycle::Completed));
    }
    #[test]
    fn replay_enriches_legacy_roster_without_touching_times_or_custom_name() {
        let s=Store::open(Path::new(":memory:")).unwrap();
        let mut old=Completed{key:"session:1".into(),run:Run::new()};
        old.run.roster_version=0;old.run.player_name="WrongFirstPeer".into();old.run.total_times.total_time=93.882;
        s.insert(&old).unwrap();let id=s.list().unwrap()[0].run.run_id;
        s.rename(id,"My PB").unwrap();s.favorite(id).unwrap();
        let mut new=old.clone();new.run.roster_version=1;new.run.self_verified=true;new.run.player_name="LocalTester".into();
        new.run.squad_members=vec![crate::models::SquadMember{member_name:"RemoteHost".into()}];new.run.total_times.total_time=99.0;
        s.insert(&new).unwrap();let records=s.list().unwrap();assert_eq!(records.len(),1);
        let r=&records[0];assert_eq!(r.run.run_id,id);assert_eq!(r.run.run_name,"My PB");assert!(r.favorite);
        assert_eq!(r.run.total_times.total_time,93.882);assert!(r.run.self_verified);assert_eq!(r.run.player_name,"LocalTester");assert_eq!(r.run.squad_members[0].member_name,"RemoteHost");
        s.delete(id).unwrap();s.insert(&new).unwrap();assert!(s.list().unwrap().is_empty());
    }
    #[test]
    fn replay_and_delete_do_not_duplicate() {
        let s = Store::open(Path::new(":memory:")).unwrap();
        let item = Completed { key: "session:1".into(), run: Run::new() };
        s.insert(&item).unwrap(); s.insert(&item).unwrap();
        let r = s.list().unwrap(); assert_eq!(r.len(), 1);
        s.delete(r[0].run.run_id).unwrap(); s.insert(&item).unwrap();
        assert!(s.list().unwrap().is_empty());
    }
}
