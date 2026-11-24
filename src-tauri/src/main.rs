// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod models; mod events;
mod engine; mod tail; mod store;
use engine::{Completed, Engine};
use tail::{Tail, Framer, Line};
use store::{Store, Record};
use serde::Serialize;
use std::{collections::VecDeque, path::PathBuf, sync::{Arc, Mutex}, time::Duration, thread};
use tauri::{Manager, State};

#[derive(Clone, Serialize, Default)]
struct Status { mode: String, path: String, offset: u64, size: u64, warnings: u64, active: bool, error: String, revision: u64 }
struct Shared { path: Mutex<PathBuf>, status: Mutex<Status>, store: Mutex<Store> }
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> { m.lock().unwrap_or_else(|e| e.into_inner()) }
fn default_path() -> PathBuf {
    #[cfg(target_os="windows")]
    { std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("Warframe").join("EE.log") }
    #[cfg(not(target_os="windows"))]
    { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".local/share/Steam/steamapps/compatdata/230410/pfx/drive_c/users/steamuser/AppData/Local/Warframe/EE.log") }
}
fn worker(shared: Arc<Shared>) {
    let mut tail = Tail::new(); let mut framer = Framer::new(); let mut engine = Engine::new();
    let mut path = PathBuf::new(); let mut pending: VecDeque<Completed> = VecDeque::new();
    loop {
        let desired = lock(&shared.path).clone();
        if desired != path { path=desired; tail=Tail::new(); framer=Framer::new(); engine.reset(); }
        let mut status = Status { path: path.to_string_lossy().into_owned(), revision: lock(&shared.status).revision, ..Status::default() };
        // Keep failed writes queued rather than dropping completed runs.
        while let Some(item) = pending.front() {
            match lock(&shared.store).insert(item) {
                Ok(()) => { pending.pop_front(); status.revision += 1; }
                Err(e) => { status.mode="storage_error".into(); status.error=e; break; }
            }
        }
        if !pending.is_empty() {
            status.offset=tail.offset; status.warnings=engine.warnings; status.active=engine.active();
            *lock(&shared.status)=status; thread::sleep(Duration::from_secs(1)); continue;
        }
        let mut idle = true;
        match tail.poll(&path) {
            Ok(batch) => {
                if batch.reset { framer=Framer::new(); engine.reset(); }
                for line in framer.push(&batch.bytes) {
                    match line {
                        Line::Complete(pos, bytes) => if let Some(item)=engine.line(pos, &bytes) { pending.push_back(item); },
                        Line::Oversized => engine.bad_line(),
                    }
                }
                idle = batch.bytes.is_empty();
                status.size=batch.size;
                status.mode=if tail.offset < batch.size { "replay" } else { "live" }.into();
            }
            Err(e) => {
                status.mode=if e.kind()==std::io::ErrorKind::NotFound { "waiting" } else { "retry" }.into();
                status.error=e.to_string();
            }
        }
        status.offset=tail.offset; status.warnings=engine.warnings; status.active=engine.active();
        *lock(&shared.status)=status;
        if idle && pending.is_empty() { thread::sleep(Duration::from_millis(250)); }
    }
}
#[tauri::command]
fn get_status(s: State<'_, Arc<Shared>>) -> Status { lock(&s.status).clone() }
#[tauri::command]
fn get_runs(s: State<'_, Arc<Shared>>) -> Result<Vec<Record>, String> { lock(&s.store).list() }
#[tauri::command]
fn rename_run(s: State<'_, Arc<Shared>>, id: i32, name: String) -> Result<(), String> {
    let name=name.trim(); if name.is_empty() || name.chars().count()>100 { return Err("名称需为 1–100 个字符".into()); }
    lock(&s.store).rename(id,name)
}
#[tauri::command]
fn favorite_run(s: State<'_, Arc<Shared>>, id: i32) -> Result<(), String> { lock(&s.store).favorite(id) }
#[tauri::command]
fn delete_run(s: State<'_, Arc<Shared>>, id: i32) -> Result<(), String> { lock(&s.store).delete(id) }
#[tauri::command]
fn set_log_path(s: State<'_, Arc<Shared>>, path: String) -> Result<(), String> {
    let p=if path.trim().is_empty() { default_path() } else { PathBuf::from(path.trim()) };
    if !p.is_absolute() || p.file_name().is_none() { return Err("请输入日志文件的绝对路径".into()); }
    lock(&s.store).set_path(&p.to_string_lossy())?;
    *lock(&s.path)=p; Ok(())
}
fn main() {
    tauri::Builder::default().setup(|app| {
        let dir=app.path().app_local_data_dir()?;
        std::fs::create_dir_all(&dir)?;
        let store=Store::open(&dir.join("runs.sqlite3")).map_err(std::io::Error::other)?;
        let path=store.path().map(PathBuf::from).unwrap_or_else(default_path);
        let shared=Arc::new(Shared { path: Mutex::new(path.clone()), store: Mutex::new(store),
            status: Mutex::new(Status { path:path.to_string_lossy().into_owned(), mode:"waiting".into(), ..Status::default() }) });
        app.manage(shared.clone());
        thread::Builder::new().name("ee-log-reader".into()).spawn(move || worker(shared))?;
        Ok(())
    }).invoke_handler(tauri::generate_handler![get_status,get_runs,rename_run,favorite_run,delete_run,set_log_path])
      .run(tauri::generate_context!()).expect("failed to start PT Analytics Next");
}
