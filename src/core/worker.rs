use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::game::command::{build, LaunchParams};
use crate::game::launch::GameProcess;
use crate::game::{offline_uuid, window};
use crate::install::installer::{Installer, Phase};
use crate::net::api::{Api, Auth, Manifest, VersionInfo};
use crate::net::channel::SecureChannel;
use crate::net::error::ApiError;
use crate::net::http::HttpTransport;

pub enum Task {
    Boot { refresh: Option<String>, hwid: String },
    Login { login: String, password: String, hwid: String },
    Logout { access: String, refresh: String },
    Play(PlayRequest),
}

pub struct PlayRequest {
    pub access: String,
    pub hwid: String,
    pub username: String,
    pub memory: u32,
    pub installed: HashMap<String, String>,
    pub cancel: Arc<AtomicBool>,
}

pub type Session = (Auth, Option<Manifest>);

pub enum Event {
    Version(Result<VersionInfo, ApiError>),
    Authed(Result<Session, ApiError>),
    Progress { phase: Phase, done: u64, total: u64 },
    PlayDone { installed: HashMap<String, String> },
    PlayFailed { code: String, installed: HashMap<String, String> },
}

pub struct Worker {
    sender: Sender<Task>,
}

impl Worker {
    pub fn spawn<W: Fn() + Send + 'static>(wake: W) -> (Self, Receiver<Event>) {
        let (task_tx, task_rx) = channel::<Task>();
        let (event_tx, event_rx) = channel::<Event>();
        thread::spawn(move || {
            let channel = SecureChannel::new(Box::new(HttpTransport::new(Config::api_url())), &Config::server_key());
            while let Ok(task) = task_rx.recv() {
                run(&channel, task, &event_tx, &wake);
                wake();
            }
        });
        (Self { sender: task_tx }, event_rx)
    }

    pub fn submit(&self, task: Task) {
        let _ = self.sender.send(task);
    }
}

fn run<W: Fn()>(channel: &SecureChannel, task: Task, events: &Sender<Event>, wake: &W) {
    let api = Api::new(channel);
    match task {
        Task::Boot { refresh, hwid } => {
            let version = api.version();
            let proceed = version.is_ok();
            let _ = events.send(Event::Version(version));
            if let (true, Some(token)) = (proceed, refresh) {
                let _ = events.send(Event::Authed(authenticate(&api, api.refresh(&token, &hwid))));
            }
        }
        Task::Login { login, password, hwid } => {
            let _ = events.send(Event::Authed(authenticate(&api, api.login(&login, &password, &hwid))));
        }
        Task::Logout { access, refresh } => {
            let _ = api.logout(&access, &refresh);
        }
        Task::Play(request) => play(channel, request, events, wake),
    }
}

fn authenticate(api: &Api, auth: Result<Auth, ApiError>) -> Result<Session, ApiError> {
    auth.map(|auth| {
        let manifest = api.manifest().ok();
        (auth, manifest)
    })
}

fn play<W: Fn()>(channel: &SecureChannel, request: PlayRequest, events: &Sender<Event>, wake: &W) {
    let api = Api::new(channel);
    let cancel = request.cancel.clone();
    let result = play_inner(channel, &api, &request, events, wake, &cancel);
    match result {
        Ok(installed) => {
            let _ = events.send(Event::PlayDone { installed });
        }
        Err((code, installed)) => {
            let _ = events.send(Event::PlayFailed { code, installed });
        }
    }
}

fn play_inner<W: Fn()>(
    channel: &SecureChannel,
    api: &Api,
    request: &PlayRequest,
    events: &Sender<Event>,
    wake: &W,
    cancel: &Arc<AtomicBool>,
) -> Result<HashMap<String, String>, (String, HashMap<String, String>)> {
    let fail = |e: ApiError, installed: HashMap<String, String>| (e.code, installed);
    let mut installed = request.installed.clone();
    let mut outcome = None;
    for attempt in 0..2 {
        let manifest = api.manifest().map_err(|e| fail(e, installed.clone()))?;
        let mut installer = Installer::new(channel, &request.access, installed.clone());
        let mut last = Instant::now();
        match installer.run(
            &manifest,
            |phase, done, total| {
                if last.elapsed() >= Duration::from_millis(60) || done == total {
                    last = Instant::now();
                    let _ = events.send(Event::Progress { phase, done, total });
                    wake();
                }
            },
            || cancel.load(Ordering::Relaxed),
        ) {
            Ok(result) => {
                installed = result.installed.clone();
                outcome = Some(result);
                break;
            }
            Err(error) => {
                let retriable = attempt == 0 && (error.is("CHECKSUM") || error.is("NOT_FOUND") || error.is("STALE_LINK"));
                if !retriable {
                    return Err(fail(error, installed));
                }
            }
        }
    }
    let outcome = outcome.ok_or_else(|| ("CHECKSUM".to_owned(), installed.clone()))?;

    let _ = events.send(Event::Progress { phase: Phase::Verifying, done: 1, total: 1 });
    wake();

    let session = api
        .session(&request.access, &request.hwid)
        .map_err(|e| fail(e, installed.clone()))?;
    let protection = api
        .protection(&session.client_token)
        .map_err(|e| fail(e, installed.clone()))?;

    let spec = build(LaunchParams {
        outcome: &outcome,
        username: &request.username,
        uuid: &offline_uuid(&request.username),
        access_token: &request.access,
        client_token: session.client_token,
        bo_token: protection.token,
        memory_gb: request.memory,
        launcher_version: Config::VERSION,
    });
    let mut process = GameProcess::spawn(&spec).map_err(|e| fail(e, installed.clone()))?;
    let pid = process.pid();

    loop {
        if window::has_visible_window(pid) {
            return Ok(installed);
        }
        if process.exited() {
            return Err(("LAUNCH_EXITED".to_owned(), installed));
        }
        thread::sleep(Duration::from_millis(150));
    }
}
