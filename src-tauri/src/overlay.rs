// SPDX-License-Identifier: MIT
//! A read-only view of the existing log engine; no game injection or process-memory reads.
use crate::{engine::{Engine,LiveView,Completed},lock,Shared};
use serde::{Deserialize,Serialize};
use std::time::{Instant,SystemTime,UNIX_EPOCH,Duration};
use std::sync::Arc;
use tauri::{AppHandle,Manager,State,WebviewWindow,LogicalSize,PhysicalPosition};

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(default)]
pub struct Config {
    pub enabled:bool, pub vertical:bool, pub opacity:u8, pub scale:u8,
    pub details:bool, pub compare:bool, pub auto_hide:bool, pub foreground_only:bool,
    pub x:Option<i32>,pub y:Option<i32>,pub width:Option<f64>,pub height:Option<f64>,
}
impl Default for Config {
    fn default()->Self { Self{enabled:false,vertical:false,opacity:85,scale:100,details:true,compare:true,auto_hide:true,foreground_only:true,x:None,y:None,width:None,height:None} }
}
impl Config {
    pub fn normalize(&mut self) {
        self.opacity=self.opacity.clamp(20,100);self.scale=self.scale.clamp(50,200);
        self.width=self.width.filter(|x|x.is_finite()).map(|x|x.clamp(300.0,2000.0));
        self.height=self.height.filter(|x|x.is_finite()).map(|x|x.clamp(160.0,1200.0));
    }
    fn dimensions(&self)->(f64,f64) {
        let (w,h)=if self.vertical{(360.,490.)}else{(860.,230.)};
        let scale=self.scale as f64/100.;
        (self.width.unwrap_or((w*scale).max(300.)),self.height.unwrap_or((h*scale).max(160.)))
    }
}
#[derive(Clone,Serialize)]
pub struct Settings { pub config:Config,pub editing:bool,pub error:Option<String> }
#[derive(Clone,Serialize)]
pub struct Pb { pub flight:f64,pub cumulative:Vec<f64> }
impl Pb {
    fn from_run(r:&crate::models::Run)->Self {
        let mut t=r.total_times.total_flight_time;
        Self{flight:t,cumulative:r.phases.iter().map(|p|{t+=p.total_time;t}).collect()}
    }
}
#[derive(Serialize)]
pub struct Frame { pub settings:Settings,pub live:Option<LiveView>,pub age_ms:u64,pub pb:Option<Pb> }
pub struct Runtime {
    pub config:Config, editing:bool, ready:bool, error:Option<String>, sample:Option<(LiveView,Instant)>, pb:Option<Pb>,
    pb_key:Option<String>, shown:bool, finished_until:Option<Instant>,
}
impl Runtime {
    pub fn new(config:Config)->Self {Self{config,editing:false,ready:false,error:None,sample:None,pb:None,pb_key:None,shown:false,finished_until:None}}
    fn settings(&self)->Settings {Settings{config:self.config.clone(),editing:self.editing,error:self.error.clone()}}
    fn clear(&mut self) {self.sample=None;self.finished_until=None;self.pb_key=None;self.pb=None;}
    fn frame(&self)->Frame {
        let (live,age)=self.sample.as_ref().map(|(v,t)|(Some(v.clone()),t.elapsed().as_millis().min(u64::MAX as u128) as u64)).unwrap_or((None,0));
        Frame{settings:self.settings(),live,age_ms:age,pb:self.pb.clone()}
    }
}
/// Keep the session-to-monotonic-clock mapping independent from exact recorded times.
/// Reject old logs at startup, and stop extrapolation when diagnostic lines go stale.
#[derive(Default)]
pub struct Clock { anchor:Option<(i64,f64,Instant)> }
impl Clock {
    pub fn reset(&mut self){self.anchor=None;}
    fn at(&mut self, epoch:i64, last:f64, wall:f64, instant:Instant)->Option<f64> {
        let predicted=self.anchor.filter(|(e,_,_)|*e==epoch).map(|(_,t,i)|t+instant.saturating_duration_since(i).as_secs_f64());
        let now=match predicted {
            Some(t) if last<=t+2.0=>t.max(last),
            Some(_)=>{self.anchor=Some((epoch,last,instant));last},
            None=>{
                let estimated=wall-epoch as f64;
                if !(-3.0..=30.0).contains(&(estimated-last)){return None;}
                let t=estimated.max(last);self.anchor=Some((epoch,t,instant));t
            }
        };
        if now-last>30.0 { self.anchor=None;return None; }
        Some(now)
    }
    pub fn sample(&mut self,engine:&Engine)->Option<LiveView> {
        let (epoch,last)=engine.live_clock()?;
        let wall=SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs_f64();
        engine.live_view(self.at(epoch,last,wall,Instant::now())?)
    }
}
fn err(e:impl std::fmt::Display)->String{e.to_string()}
fn window(app:&AppHandle)->Result<WebviewWindow,String>{app.get_webview_window("overlay").ok_or_else(||"Overlay 窗口不可用".into())}
#[cfg(windows)]
fn game_foreground()->bool {
    use windows_sys::Win32::{Foundation::CloseHandle,System::Threading::{OpenProcess,QueryFullProcessImageNameW,PROCESS_QUERY_LIMITED_INFORMATION},UI::WindowsAndMessaging::{GetForegroundWindow,GetWindowThreadProcessId}};
    unsafe {
        let hwnd=GetForegroundWindow();if hwnd.is_null(){return false;}
        let mut pid=0;GetWindowThreadProcessId(hwnd,&mut pid);
        let process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,0,pid);if process.is_null(){return false;}
        let mut buf=[0u16;1024];let mut size=buf.len() as u32;
        let ok=QueryFullProcessImageNameW(process,0,buf.as_mut_ptr(),&mut size);CloseHandle(process);
        if ok==0{return false;}
        let path=String::from_utf16_lossy(&buf[..size as usize]);
        path.rsplit('\\').next().is_some_and(|name|name.eq_ignore_ascii_case("Warframe.x64.exe")||name.eq_ignore_ascii_case("Warframe.exe"))
    }
}
#[cfg(not(windows))]
fn game_foreground()->bool {true}
fn apply_geometry(w:&WebviewWindow,c:&Config)->Result<(),String> {
    let (width,height)=c.dimensions();
    let (min_w,min_h)=if c.vertical{(300.0,330.0)}else{(350.0,180.0)};
    w.set_min_size(Some(LogicalSize::new(min_w,min_h))).map_err(err)?;
    w.set_size(LogicalSize::new(width.max(min_w),height.max(min_h))).map_err(err)?;
    let monitors=w.available_monitors().map_err(err)?;
    if let (Some(x),Some(y))=(c.x,c.y) {
        // Keep a visible portion on a connected monitor after DPI/display changes.
        if monitors.iter().any(|m|{
            let p=m.position();let s=m.size();
            i64::from(x)>=i64::from(p.x)&&i64::from(y)>=i64::from(p.y)
                &&i64::from(x)+60<i64::from(p.x)+i64::from(s.width)
                &&i64::from(y)+60<i64::from(p.y)+i64::from(s.height)
        }) {return w.set_position(PhysicalPosition::new(x,y)).map_err(err);}
    }
    if let Some(m)=w.primary_monitor().map_err(err)? {
        let p=m.position();let s=m.size();let width=w.outer_size().map_err(err)?.width;
        let x=i64::from(p.x)+(i64::from(s.width)-i64::from(width)).max(0)/2;
        let y=i64::from(p.y)+(28.0*m.scale_factor()) as i64;
        w.set_position(PhysicalPosition::new(x as i32,y as i32)).map_err(err)?;
    }
    Ok(())
}
pub fn initialize(app:&AppHandle,s:&Arc<Shared>)->Result<(),String>{
    let mut o=lock(&s.overlay);
    let result=(|| {
        let w=window(app)?;
        w.set_ignore_cursor_events(true).map_err(err)?;
        w.set_focusable(false).map_err(err)?;
        apply_geometry(&w,&o.config)
    })();
    o.ready=result.is_ok();o.error=result.as_ref().err().cloned();result
}
fn sync(app:&AppHandle,o:&mut Runtime)->Result<(),String>{
    if !o.ready {return Err(o.error.clone().unwrap_or_else(||"Overlay 尚未初始化".into()));}
    let active=o.sample.is_some();
    let show=o.editing || (o.config.enabled && (active||!o.config.auto_hide)
        && (!o.config.foreground_only||game_foreground()));
    if show==o.shown{return Ok(());}
    let w=window(app)?;
    if show {w.show().map_err(err)?;} else {w.hide().map_err(err)?;}
    o.shown=show;Ok(())
}
pub fn clear(app:&AppHandle,s:&Arc<Shared>){
    let mut o=lock(&s.overlay);o.clear();let _=sync(app,&mut o);
}
pub fn update(app:&AppHandle,s:&Arc<Shared>,live:Option<LiveView>,finished:Option<&Completed>){
    let mut o=lock(&s.overlay);
    if !o.config.enabled&&!o.editing {o.clear();let _=sync(app,&mut o);return;}
    if let Some(v)=live {
        if o.pb_key.as_ref()!=Some(&v.key) {
            o.pb=lock(&s.store).list().ok().and_then(|runs|runs.into_iter().filter(|r|r.run.is_ranked()).min_by(|a,b|a.run.total_times.total_time.total_cmp(&b.run.total_times.total_time))).map(|r|Pb::from_run(&r.run));
            o.pb_key=Some(v.key.clone());
        }
        o.sample=Some((v,Instant::now()));o.finished_until=None;
    } else if let Some(item)=finished.filter(|r|r.run.is_ranked()){
        let r=&item.run;
        o.sample=Some((LiveView{key:item.key.clone(),elapsed:r.elapsed_seconds,phase:4,phase_elapsed:None,
            flight:Some(r.total_times.total_flight_time),completed:r.phases.clone(),active_metric:"finished",shield:None,
            shield_time:None,leg_time:None,body_time:None,pylon_time:None,legs:4,uncertain:false},Instant::now()));
        o.finished_until=Some(Instant::now()+Duration::from_secs(5));
    } else if !o.finished_until.is_some_and(|t|Instant::now()<t) {o.clear();}
    if let Err(e)=sync(app,&mut o){o.error=Some(e);}else{o.error=None;}
}
pub fn requested(s:&Arc<Shared>)->bool {
    let o=lock(&s.overlay);o.ready&&(o.config.enabled||o.editing)
}
#[tauri::command]
pub async fn get_overlay_settings(s:State<'_,Arc<Shared>>)->Settings {lock(&s.overlay).settings()}
#[tauri::command]
pub async fn get_overlay_frame(s:State<'_,Arc<Shared>>)->Frame {lock(&s.overlay).frame()}
#[tauri::command]
pub async fn set_overlay_settings(app:AppHandle,s:State<'_,Arc<Shared>>,mut config:Config)->Result<Settings,String>{
    config.normalize();
    let mut o=lock(&s.overlay);
    // Geometry is authoritative on the native side. Only layout/scale changes reset dimensions.
    config.x=o.config.x;config.y=o.config.y;
    let resize=config.vertical!=o.config.vertical||config.scale!=o.config.scale;
    config.width=if resize{None}else{o.config.width};config.height=if resize{None}else{o.config.height};
    if resize {apply_geometry(&window(&app)?,&config)?;}
    if !config.enabled&&!o.editing{o.clear();}
    lock(&s.store).set_overlay_config(&config)?;
    o.config=config;sync(&app,&mut o)?;o.error=None;Ok(o.settings())
}
#[tauri::command]
pub async fn edit_overlay(app:AppHandle,s:State<'_,Arc<Shared>>,editing:bool)->Result<Settings,String>{
    let mut o=lock(&s.overlay);let w=window(&app)?;
    if !o.ready{return Err(o.error.clone().unwrap_or_else(||"Overlay 尚未初始化".into()));}
    if editing {
        if !o.editing {apply_geometry(&w,&o.config)?;}
        w.set_focusable(true).map_err(err)?;w.set_ignore_cursor_events(false).map_err(err)?;
        o.editing=true;sync(&app,&mut o)?;w.set_focus().map_err(err)?;
    } else if o.editing {
        let p=w.outer_position().map_err(err)?;let size=w.inner_size().map_err(err)?;let dpi=w.scale_factor().map_err(err)?;
        let mut cfg=o.config.clone();cfg.x=Some(p.x);cfg.y=Some(p.y);cfg.width=Some(size.width as f64/dpi);cfg.height=Some(size.height as f64/dpi);cfg.normalize();
        lock(&s.store).set_overlay_config(&cfg)?;o.config=cfg;
        w.set_ignore_cursor_events(true).map_err(err)?;w.set_focusable(false).map_err(err)?;
        o.editing=false;sync(&app,&mut o)?;
    }
    Ok(o.settings())
}
#[tauri::command]
pub async fn reset_overlay(app:AppHandle,s:State<'_,Arc<Shared>>)->Result<Settings,String>{
    let mut o=lock(&s.overlay);let mut cfg=Config::default();cfg.enabled=o.config.enabled;
    apply_geometry(&window(&app)?,&cfg)?;lock(&s.store).set_overlay_config(&cfg)?;
    o.config=cfg;sync(&app,&mut o)?;Ok(o.settings())
}
#[tauri::command]
pub async fn drag_overlay(w:tauri::Window,s:State<'_,Arc<Shared>>,resize:bool)->Result<(),String>{
    if w.label()!="overlay"||!lock(&s.overlay).editing{return Err("Overlay 未处于调整模式".into());}
    if resize {w.start_resize_dragging(tauri_runtime::ResizeDirection::SouthEast).map_err(err)}
    else {w.start_dragging().map_err(err)}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_log_never_starts_clock_and_wall_changes_do_not_jump_live_time(){
        let mut c=Clock::default();let t=Instant::now();
        assert_eq!(c.at(1000,20.,2000.,t),None);
        assert_eq!(c.at(1000,20.,1020.5,t),Some(20.5));
        assert_eq!(c.at(1000,21.,5000.,t+Duration::from_secs(1)),Some(21.5));
        assert_eq!(c.at(1000,21.,5000.,t+Duration::from_secs(40)),None);
        c.reset();assert_eq!(c.at(2000,10.,2010.,t),Some(10.));
    }
    #[test]
    fn config_clamps_and_has_safe_default(){
        let mut c=Config::default();assert!(!c.enabled);assert!(c.auto_hide);assert!(c.foreground_only);
        c.scale=255;c.opacity=0;c.width=Some(f64::NAN);c.normalize();
        assert_eq!(c.scale,200);assert_eq!(c.opacity,20);assert_eq!(c.width,None);
    }
}
