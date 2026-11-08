use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::core::paths::Paths;
use crate::game::command::{build, LaunchParams};
use crate::game::launch::GameProcess;
use crate::game::{offline_uuid, window};
use crate::install::client::{ClientFetcher, Progress};
use crate::install::download::Scratch;
use crate::install::installer::{Installer, Phase, Runtime};
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
    pub build: String,
    pub runtime: Runtime,
    pub installed: HashMap<String, String>,
    pub cancel: Arc<AtomicBool>,
}

pub type Session = (Auth, Option<Manifest>);

pub enum Event {
    Version(Result<VersionInfo, ApiError>),
    Authed(Result<Session, ApiError>),
    Progress { phase: Phase, done: u64, total: u64 },
    PlayDone { installed: HashMap<String, String> },
    PlayFailed { code: String, detail: Option<String>, installed: HashMap<String, String> },
    Estimated { generation: u64, bytes: Option<u64>, installed: HashMap<String, String> },
}

pub struct Worker {
    sender: Sender<Task>,
    events: Sender<Event>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl Worker {
    pub fn spawn<W: Fn() + Send + Sync + 'static>(wake: W) -> (Self, Receiver<Event>) {
        let (task_tx, task_rx) = channel::<Task>();
        let (event_tx, event_rx) = channel::<Event>();
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let events = event_tx.clone();
        let notify = wake.clone();
        thread::spawn(move || {
            let channel = SecureChannel::new(Box::new(HttpTransport::new(Config::api_url())), &Config::server_key());
            let wake = || notify();
            while let Ok(task) = task_rx.recv() {
                run(&channel, task, &event_tx, &wake);
                wake();
            }
        });
        (Self { sender: task_tx, events, wake }, event_rx)
    }

    pub fn submit(&self, task: Task) {
        let _ = self.sender.send(task);
    }

    pub fn estimate(&self, generation: u64, manifest: Manifest, installed: HashMap<String, String>, runtime: Runtime) {
        let events = self.events.clone();
        let wake = self.wake.clone();
        thread::spawn(move || {
            let mut installer = Installer::new(installed, runtime);
            let bytes = installer.pending_bytes(&manifest).ok();
            let installed = installer.installed().clone();
            let _ = events.send(Event::Estimated { generation, bytes, installed });
            wake();
        });
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
        let manifest = api.manifest(&auth.access_token).ok();
        (auth, manifest)
    })
}

fn play<W: Fn()>(channel: &SecureChannel, request: PlayRequest, events: &Sender<Event>, wake: &W) {
    let api = Api::new(channel);
    let cancel = request.cancel.clone();
    let result = play_inner(&api, &request, events, wake, &cancel);
    match result {
        Ok(installed) => {
            let _ = events.send(Event::PlayDone { installed });
        }
        Err((error, installed)) => {
            let _ = events.send(Event::PlayFailed { code: error.code, detail: error.detail, installed });
        }
    }
}

fn play_inner<W: Fn()>(
    api: &Api,
    request: &PlayRequest,
    events: &Sender<Event>,
    wake: &W,
    cancel: &Arc<AtomicBool>,
) -> Result<HashMap<String, String>, (ApiError, HashMap<String, String>)> {
    let fail = |e: ApiError, installed: HashMap<String, String>| (e, installed);
    let mut installed = request.installed.clone();
    let mut outcome = None;
    let scratch = Scratch;
    for attempt in 0..2 {
        let manifest = api.manifest(&request.access).map_err(|e| fail(e, installed.clone()))?;
        let mut installer = Installer::new(installed.clone(), request.runtime.clone());
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
                installed = installer.installed().clone();
                let retriable = attempt == 0 && (error.is(crate::secret!("CHECKSUM")) || error.is(crate::secret!("NOT_FOUND")) || error.is(crate::secret!("STALE_LINK")));
                if !retriable {
                    return Err(fail(error, installed));
                }
            }
        }
    }
    let outcome = outcome.ok_or_else(|| (ApiError::new(crate::secret!("CHECKSUM"), 0), installed.clone()))?;

    let _ = events.send(Event::Progress { phase: Phase::Verifying, done: 1, total: 1 });
    wake();

    let session = api
        .session(&request.access, &request.hwid)
        .map_err(|e| fail(e, installed.clone()))?;
    let protection = api
        .protection(&session.client_token, &request.build)
        .map_err(|e| fail(e, installed.clone()))?;

    if request.runtime.client.is_none() {
        let mut last = Instant::now();
        ClientFetcher::new(Config::client_address(), &protection.token, || cancel.load(Ordering::Relaxed))
            .fetch(&Paths::mods().join(crate::secret!("client.jar")), |progress| {
                let (phase, done, total) = match progress {
                    Progress::Connecting => (Phase::ConnectingClient, 0, 0),
                    Progress::Waiting => (Phase::WaitingClient, 0, 0),
                    Progress::Receiving { done, total } => (Phase::DownloadClient, done, total),
                };
                if total == 0 || last.elapsed() >= Duration::from_millis(60) || done == total {
                    last = Instant::now();
                    let _ = events.send(Event::Progress { phase, done, total });
                    wake();
                }
            })
            .map_err(|e| fail(e, installed.clone()))?;
    }
    drop(scratch);

    let spec = build(LaunchParams {
        outcome: &outcome,
        username: &request.username,
        uuid: &offline_uuid(&request.username),
        access_token: &request.access,
        client_token: session.client_token,
        bo_token: protection.token,
        memory_gb: request.memory,
        launcher_version: &Config::version(),
    });
    let mut process = GameProcess::spawn(&spec).map_err(|e| fail(e, installed.clone()))?;
    let pid = process.pid();

    loop {
        if window::has_visible_window(pid) {
            return Ok(installed);
        }
        if process.exited() {
            return Err((ApiError::new(crate::secret!("LAUNCH_EXITED"), 0), installed));
        }
        thread::sleep(Duration::from_millis(150));
    }
}
