use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::thread;

use crate::net::error::ApiError;

pub enum Signal {
    Progress(usize, u64),
    Finished(usize),
}

pub struct Pool {
    workers: usize,
}

impl Pool {
    pub fn new(workers: usize) -> Self {
        Self { workers: workers.max(1) }
    }

    pub fn run<T, S, O, W, E>(&self, items: &[T], open: O, work: W, mut observe: E) -> Result<(), ApiError>
    where
        T: Sync,
        O: Fn() -> Result<S, ApiError> + Sync,
        W: Fn(&mut S, &T, &dyn Fn(u64)) -> Result<(), ApiError> + Sync,
        E: FnMut(Signal),
    {
        if items.is_empty() {
            return Ok(());
        }
        let next = AtomicUsize::new(0);
        let stop = AtomicBool::new(false);
        let failure = Mutex::new(None::<ApiError>);
        let (sender, receiver) = channel::<Signal>();
        thread::scope(|scope| {
            for _ in 0..self.workers.min(items.len()) {
                let sender = sender.clone();
                let (next, stop, failure, open, work) = (&next, &stop, &failure, &open, &work);
                scope.spawn(move || {
                    let fail = |error: ApiError| {
                        stop.store(true, Ordering::Relaxed);
                        if let Ok(mut slot) = failure.lock() {
                            slot.get_or_insert(error);
                        }
                    };
                    let mut state = match open() {
                        Ok(state) => state,
                        Err(error) => return fail(error),
                    };
                    while !stop.load(Ordering::Relaxed) {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index) else { break };
                        let report = |bytes: u64| {
                            let _ = sender.send(Signal::Progress(index, bytes));
                        };
                        match work(&mut state, item, &report) {
                            Ok(()) => {
                                let _ = sender.send(Signal::Finished(index));
                            }
                            Err(error) => return fail(error),
                        }
                    }
                });
            }
            drop(sender);
            for signal in receiver {
                observe(signal);
            }
        });
        match failure.into_inner().ok().flatten() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
