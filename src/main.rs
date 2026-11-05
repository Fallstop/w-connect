#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
mod config;
mod control;
mod device;
mod sensors;
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use config::Config;
use fs2::FileExt;
use serde::Serialize;
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Serialize)]
struct Hub {
    connected: bool,
    name: String,
    serial: String,
    vid: u16,
    pid: u16,
    error: Option<String>,
    lighting_supported: bool,
    hardware_tested: bool,
    model_name: String,
    max_fans_per_bank: u8,
}
#[derive(Clone, Serialize)]
struct ChannelState {
    index: usize,
    name: String,
    rpm: Option<u32>,
    duty: Option<u8>,
    target: u8,
    mode: String,
    status: String,
}
#[derive(Clone, Serialize)]
struct Event {
    time: String,
    message: String,
}
#[derive(Clone, Serialize)]
struct Snapshot {
    version: String,
    platform: String,
    demo: bool,
    emergency: bool,
    requested_revision: u64,
    applied_revision: u64,
    controller: Hub,
    sensors: sensors::Sensors,
    channels: Vec<ChannelState>,
    config: Config,
    events: Vec<Event>,
    uptime_secs: u64,
}
struct Shared {
    snapshot: Snapshot,
    path: PathBuf,
    start: Instant,
    revision: u64,
    applied_revision: u64,
    control_updated: Instant,
    confirmed_revision: u64,
    emergency: bool,
}
type App = Arc<Mutex<Shared>>;
fn event(s: &mut Shared, message: String) {
    if s.snapshot
        .events
        .last()
        .is_some_and(|e| e.message == message)
    {
        return;
    }
    s.snapshot.events.push(Event {
        time: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string(),
        message,
    });
    if s.snapshot.events.len() > 50 {
        s.snapshot.events.remove(0);
    }
}
fn snapshot(app: &App) -> Snapshot {
    let mut s = app.lock().unwrap();
    s.snapshot.emergency = s.emergency;
    s.snapshot.requested_revision = s.revision;
    s.snapshot.applied_revision = s.confirmed_revision;
    s.snapshot.uptime_secs = s.start.elapsed().as_secs();
    let mut result = s.snapshot.clone();
    if s.control_updated.elapsed() > Duration::from_secs(10) {
        result.controller.connected = false;
        result.controller.error = Some(
            "Control loop has not responded for over ten seconds; cooling cannot be confirmed"
                .into(),
        );
        result.sensors = sensors::Sensors::default();
        for c in &mut result.channels {
            c.rpm = None;
            c.duty = None;
            c.status = "Control loop unresponsive".into();
        }
    }
    result
}
async fn state(State(app): State<App>) -> Json<Snapshot> {
    Json(snapshot(&app))
}
fn permitted(headers: &HeaderMap) -> bool {
    headers.get("x-w-connect").and_then(|v| v.to_str().ok()) == Some("1")
        && headers
            .get("sec-fetch-site")
            .and_then(|v| v.to_str().ok())
            .is_none_or(|v| ["same-origin", "none"].contains(&v))
        && headers
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .is_none_or(|v| v == "http://127.0.0.1:7676" || v == "http://localhost:7676")
}
fn error(code: StatusCode, message: impl ToString) -> Response {
    (code, Json(serde_json::json!({"error":message.to_string()}))).into_response()
}
async fn configure(
    State(app): State<App>,
    headers: HeaderMap,
    Json(config): Json<Config>,
) -> Response {
    if !permitted(&headers) {
        return error(StatusCode::FORBIDDEN, "Local app request header required");
    }
    if let Err(e) = config.validate() {
        return error(StatusCode::BAD_REQUEST, e);
    }
    {
        let current = app.lock().unwrap();
        if current.snapshot.controller.connected
            && config.controller == current.snapshot.config.controller
            && config
                .channels
                .iter()
                .any(|c| c.fan_count > current.snapshot.controller.max_fans_per_bank)
        {
            return error(
                StatusCode::BAD_REQUEST,
                format!(
                    "This controller supports up to {} fans per bank",
                    current.snapshot.controller.max_fans_per_bank
                ),
            );
        }
    }
    {
        let mut s = app.lock().unwrap();
        if let Err(e) = config.save(&s.path) {
            return error(StatusCode::INTERNAL_SERVER_ERROR, e);
        }
        s.snapshot.config = config;
        s.revision += 1;
        s.emergency = false;
        event(
            &mut s,
            "Configuration saved · waiting for controller".into(),
        );
    }
    Json(snapshot(&app)).into_response()
}
async fn emergency(State(app): State<App>, headers: HeaderMap) -> Response {
    if !permitted(&headers) {
        return error(StatusCode::FORBIDDEN, "Local app request header required");
    }
    {
        let mut s = app.lock().unwrap();
        s.emergency = true;
        s.revision += 1;
        event(
            &mut s,
            "Full cooling requested for all banks; save a profile to resume".into(),
        );
    }
    Json(snapshot(&app)).into_response()
}
async fn resume(State(app): State<App>, headers: HeaderMap) -> Response {
    if !permitted(&headers) {
        return error(StatusCode::FORBIDDEN, "Local app request header required");
    }
    {
        let mut s = app.lock().unwrap();
        s.emergency = false;
        s.revision += 1;
        event(&mut s, "Saved cooling profile requested".into());
    }
    Json(snapshot(&app)).into_response()
}
async fn devices(State(app): State<App>) -> Response {
    let demo = app.lock().unwrap().snapshot.demo;
    let found = if demo {
        Ok(vec![])
    } else {
        match tokio::task::spawn_blocking(device::discover).await {
            Ok(r) => r,
            Err(e) => Err(e.to_string()),
        }
    };
    match found {
        Ok(found) => Json(
            serde_json::json!({"devices":found,"models":device::supported_models(),"demo":demo}),
        )
        .into_response(),
        Err(e) => error(StatusCode::SERVICE_UNAVAILABLE, e),
    }
}
fn config_path() -> PathBuf {
    if let Some(p) = std::env::var_os("W_CONNECT_CONFIG") {
        return p.into();
    }
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("w-connect/config.json")
}
fn run_control(
    app: App,
    stop: Arc<AtomicBool>,
    sensor_data: Arc<Mutex<(sensors::Sensors, Instant)>>,
    demo: bool,
) {
    let mut hub: Option<device::Controller> = None;
    let mut duties = [100u8; 4];
    let mut selection: Option<config::ControllerConfig> = None;
    let mut last_write = Instant::now() - Duration::from_secs(20);
    let mut next_tick = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        let immediate = {
            let s = app.lock().unwrap();
            s.revision != s.applied_revision
        };
        if Instant::now() < next_tick && !immediate {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        let (cfg, revision, emergency) = {
            let s = app.lock().unwrap();
            (s.snapshot.config.clone(), s.revision, s.emergency)
        };
        next_tick = Instant::now() + Duration::from_millis(cfg.poll_ms);
        let (sensors, sampled) = sensor_data.lock().unwrap().clone();
        let stale = sampled.elapsed() > Duration::from_secs(10);
        let mut failure = None;
        let mut reconnected = false;
        if selection.as_ref() != Some(&cfg.controller) {
            if let Some(previous) = hub.as_mut() {
                for i in 0..4 {
                    if let Err(e) = previous.set_duty(i, 100) {
                        event(
                            &mut app.lock().unwrap(),
                            format!("Previous controller full-speed request failed: {e}"),
                        );
                        break;
                    }
                }
            }
            hub = None;
            selection = Some(cfg.controller.clone());
        }
        if hub.is_none() && !demo {
            match device::Controller::connect_with_options(
                cfg.controller.allow_untested,
                cfg.controller.serial.as_deref(),
            ) {
                Ok(d) => {
                    hub = Some(d);
                    reconnected = true;
                    duties = [100; 4];
                }
                Err(e) => failure = Some(e),
            }
        }
        let configuration_error=hub.as_ref().and_then(|d|cfg.channels.iter().any(|c|c.fan_count>d.max_fans_per_bank).then(||format!("This controller supports up to {} fans per bank. Cooling continues; correct fan counts before lighting can apply.",d.max_fans_per_bank)));
        let mut channels = vec![];
        let mut read_rpm = None;
        let mut wrote = false;
        if let Some(d) = hub.as_mut() {
            // First establish safe cooling on every reconnect before any cosmetic writes.
            if reconnected {
                for i in 0..4 {
                    if let Err(e) = d.set_duty(i, 100) {
                        failure = Some(e);
                        break;
                    }
                }
            }
        }
        for (i, c) in cfg.channels.iter().enumerate() {
            let (target, status) = if emergency {
                (100, "Full cooling override")
            } else {
                control::target(c, &sensors, stale, duties[i])
            };
            let mut duty = None;
            if let Some(d) = hub.as_mut().filter(|_| failure.is_none()) {
                if target != duties[i]
                    || reconnected
                    || last_write.elapsed() > Duration::from_secs(10)
                {
                    match d.set_duty(i, target) {
                        Ok(()) => {
                            duties[i] = target;
                            wrote = true;
                        }
                        Err(e) => failure = Some(e),
                    }
                }
                if failure.is_none() {
                    duty = Some(duties[i]);
                }
            }
            if demo {
                duties[i] = target;
            }
            channels.push(ChannelState {
                index: i,
                name: c.name.clone(),
                rpm: None,
                duty,
                target,
                mode: c.mode.clone(),
                status: if demo {
                    "Preview · no hardware writes".into()
                } else {
                    status.into()
                },
            });
        }
        if wrote {
            last_write = Instant::now();
        }
        if let Some(d) = hub.as_mut().filter(|_| failure.is_none()) {
            match d.rpm() {
                Ok(r) => read_rpm = Some(r),
                Err(e) => failure = Some(e),
            }
            let needs_lighting = {
                let s = app.lock().unwrap();
                reconnected || revision != s.applied_revision
            };
            if needs_lighting
                && !d.lighting_supported
                && cfg.channels.iter().any(|c| c.lighting.mode != "preserve")
            {
                event(&mut app.lock().unwrap(),"Cooling profile applied; this model's lighting is not supported and was left unchanged".into());
            }
            if needs_lighting
                && d.lighting_supported
                && configuration_error.is_none()
                && failure.is_none()
            {
                for (i, c) in cfg.channels.iter().enumerate() {
                    if c.lighting.mode != "preserve" {
                        let color = [
                            u8::from_str_radix(&c.lighting.color[1..3], 16).unwrap(),
                            u8::from_str_radix(&c.lighting.color[3..5], 16).unwrap(),
                            u8::from_str_radix(&c.lighting.color[5..7], 16).unwrap(),
                        ];
                        if let Err(e) = d.lighting(
                            i,
                            c.fan_count,
                            &c.lighting.mode,
                            color,
                            c.lighting.brightness,
                        ) {
                            failure = Some(e);
                            break;
                        }
                    }
                }
            }
        }
        if let Some(rpms) = read_rpm {
            for (i, c) in channels.iter_mut().enumerate() {
                c.rpm = Some(rpms[i]);
            }
        }
        if failure.is_some() {
            // Best-effort full-speed fallback before closing a failed connection.
            if let Some(d) = hub.as_mut() {
                for i in 0..4 {
                    let _ = d.set_duty(i, 100);
                }
            }
            hub = None;
            for c in &mut channels {
                c.duty = None;
                c.status = "Controller unavailable · cooling cannot be confirmed".into();
            }
        }
        {
            let mut s = app.lock().unwrap();
            let connected = hub.is_some();
            if let Some(d) = hub.as_ref() {
                s.snapshot.controller.name = d.name.clone();
                s.snapshot.controller.serial = d.serial.clone();
                s.snapshot.controller.pid = d.product_id;
                s.snapshot.controller.model_name = d.model_name.clone();
                s.snapshot.controller.max_fans_per_bank = d.max_fans_per_bank;
                s.snapshot.controller.hardware_tested = d.hardware_tested;
                s.snapshot.controller.lighting_supported = d.lighting_supported;
            }
            if connected && !s.snapshot.controller.connected {
                event(
                    &mut s,
                    "Controller connected · safe cooling established".into(),
                );
            }
            if let Some(ref e) = failure {
                if s.snapshot.controller.error.as_ref() != Some(e) {
                    event(&mut s, e.clone());
                }
            }
            if connected && configuration_error.is_none() && revision != s.applied_revision {
                event(&mut s, "Profile applied to controller".into());
            }
            // Failed revisions retry on the next scheduled tick, never in a tight loop.
            if connected && configuration_error.is_none() {
                s.confirmed_revision = revision;
            }
            s.applied_revision = revision;
            s.control_updated = Instant::now();
            s.snapshot.controller.connected = connected;
            s.snapshot.controller.error = failure.or(configuration_error);
            s.snapshot.channels = channels;
            s.snapshot.sensors = if stale {
                sensors::Sensors {
                    cpu: None,
                    gpu: None,
                    source: "Temperature data stale".into(),
                }
            } else {
                sensors
            };
        }
    }
    if let Some(d) = hub.as_mut() {
        for i in 0..4 {
            let _ = d.set_duty(i, 100);
        }
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|s| s == "--help" || s == "-h") {
        println!("W-Connect {}\nUsage: w-connect [--open] [--demo] [--list-devices]\nLocal UI: http://127.0.0.1:7676\n--list-devices: read-only controller discovery and support catalog\n--demo: real sensors, no USB reads or writes\nW_CONNECT_CONFIG: override config file path",env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|s| s == "--list-devices") {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"devices":device::discover().map_err(std::io::Error::other)?,"models":device::supported_models()})
            )?
        );
        return Ok(());
    }
    let demo = args.iter().any(|s| s == "--demo");
    let path = config_path();
    fs::create_dir_all(path.parent().ok_or("Invalid config path")?)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))?;
    lock.try_lock_exclusive()
        .map_err(|_| "Another W-Connect process is already using this configuration")?;
    let config = if path.exists() {
        let c: Config = serde_json::from_slice(&fs::read(&path)?)?;
        c.validate().map_err(std::io::Error::other)?;
        c
    } else {
        let c = Config::default();
        c.save(&path).map_err(std::io::Error::other)?;
        c
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:7676").await?;
    let app = Arc::new(Mutex::new(Shared {
        snapshot: Snapshot {
            version: env!("CARGO_PKG_VERSION").into(),
            platform: std::env::consts::OS.into(),
            demo,
            emergency: false,
            requested_revision: 1,
            applied_revision: 0,
            controller: Hub {
                connected: false,
                name: "Searching for controller".into(),
                serial: String::new(),
                vid: 0x0cf2,
                pid: 0,
                error: None,
                lighting_supported: demo,
                hardware_tested: false,
                model_name: String::new(),
                max_fans_per_bank: 4,
            },
            sensors: Default::default(),
            channels: vec![],
            config,
            events: vec![],
            uptime_secs: 0,
        },
        path,
        start: Instant::now(),
        revision: 1,
        applied_revision: 0,
        control_updated: Instant::now(),
        confirmed_revision: 0,
        emergency: false,
    }));
    let stop = Arc::new(AtomicBool::new(false));
    let data = Arc::new(Mutex::new((
        sensors::Sensors::default(),
        Instant::now() - Duration::from_secs(30),
    )));
    let ds = data.clone();
    let ss = stop.clone();
    std::thread::spawn(move || {
        while !ss.load(Ordering::Relaxed) {
            let v = sensors::read();
            *ds.lock().unwrap() = (v, Instant::now());
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    let ac = app.clone();
    let sc = stop.clone();
    let worker = std::thread::spawn(move || run_control(ac, sc, data, demo));
    let router = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [("content-type", "text/javascript")],
                    include_str!("../web/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [("content-type", "text/css")],
                    include_str!("../web/style.css"),
                )
            }),
        )
        .route(
            "/favicon.svg",
            get(|| async {
                (
                    [("content-type", "image/svg+xml")],
                    include_str!("../web/favicon.svg"),
                )
            }),
        )
        .route("/api/state", get(state))
        .route("/api/devices", get(devices))
        .route("/api/config", post(configure))
        .route("/api/emergency", post(emergency))
        .route("/api/resume", post(resume))
        .layer(DefaultBodyLimit::max(32768))
        .layer(axum::middleware::from_fn(local_only))
        .with_state(app);
    println!(
        "W-Connect {} · http://127.0.0.1:7676{}",
        env!("CARGO_PKG_VERSION"),
        if demo {
            " · hardware writes disabled"
        } else {
            ""
        }
    );
    if args.iter().any(|s| s == "--open") {
        open_browser();
    }
    let result = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal(stop.clone()))
        .await;
    stop.store(true, Ordering::Relaxed);
    let _ = worker.join();
    result?;
    Ok(())
}
async fn local_only(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !["127.0.0.1:7676", "localhost:7676"].contains(&host) {
        return error(StatusCode::FORBIDDEN, "Local access only");
    }
    let mut response = next.run(req).await;
    for (k,v) in [("content-security-policy","default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"),("x-content-type-options","nosniff"),("cache-control","no-store"),("referrer-policy","no-referrer")] {response.headers_mut().insert(axum::http::HeaderName::from_static(k),axum::http::HeaderValue::from_static(v));}
    response
}
async fn shutdown_signal(stop: Arc<AtomicBool>) {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal handler");
        tokio::select! {_=tokio::signal::ctrl_c()=>{},_=term.recv()=>{}}
    }
    #[cfg(windows)]
    {
        let mut close = tokio::signal::windows::ctrl_close().expect("close signal");
        let mut logoff = tokio::signal::windows::ctrl_logoff().expect("logoff signal");
        let mut shutdown = tokio::signal::windows::ctrl_shutdown().expect("shutdown signal");
        tokio::select! {_=tokio::signal::ctrl_c()=>{},_=close.recv()=>{},_=logoff.recv()=>{},_=shutdown.recv()=>{}}
    }
    stop.store(true, Ordering::Relaxed);
}
fn open_browser() {
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open")
        .arg("http://127.0.0.1:7676")
        .spawn();
    #[cfg(windows)]
    let _ = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", "http://127.0.0.1:7676"])
        .spawn();
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_cross_origin_writes() {
        let mut h = HeaderMap::new();
        assert!(!permitted(&h));
        h.insert("x-w-connect", "1".parse().unwrap());
        assert!(permitted(&h));
        h.insert("origin", "https://evil.example".parse().unwrap());
        assert!(!permitted(&h));
    }
}
