//! Bounded authenticated-platform mailbox. It never owns a GamePort.
//! Caller identity is checked by Binder before these connection IDs are used.
use crate::{Bridge, GamePort};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use trainer_protocol::{ErrorCode, Id, Request, Response};

const CAPACITY: usize = 8;
const MAX_PER_TICK: usize = 4;
const LIFETIME_MS: u64 = 500;

#[derive(Debug, PartialEq, Eq)]
pub enum ChannelError {
    Closed,
    Full,
    Expired,
    Timeout,
}
struct Job {
    request: Request,
    expires: u64,
    cancelled: Arc<AtomicBool>,
    response: SyncSender<Response>,
}
struct State {
    live: bool,
    instance: Option<Id>,
    connection: Option<u64>,
    cleanup: bool,
    jobs: VecDeque<Job>,
}
#[derive(Clone)]
pub struct Channel(Arc<Mutex<State>>);
pub struct Pending {
    response: Receiver<Response>,
    cancelled: Arc<AtomicBool>,
}
impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}
impl Channel {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(State {
            live: true,
            instance: None,
            connection: None,
            cleanup: false,
            jobs: VecDeque::new(),
        })))
    }
    pub fn open(&self, connection: u64) -> Result<Option<Id>, ChannelError> {
        let mut state = self.0.lock().unwrap();
        if !state.live || connection == 0 {
            return Err(ChannelError::Closed);
        }
        if state.connection != Some(connection) {
            state.jobs.clear();
            state.cleanup = true;
            state.connection = Some(connection);
        }
        Ok(state.instance)
    }
    pub fn close(&self, connection: u64) {
        let mut state = self.0.lock().unwrap();
        if state.connection == Some(connection) {
            state.connection = None;
            state.cleanup = true;
            state.jobs.clear();
        }
    }
    pub fn invalidate(&self) {
        let mut state = self.0.lock().unwrap();
        state.connection = None;
        state.cleanup = true;
        state.jobs.clear();
    }
    pub fn shutdown(&self) {
        let mut state = self.0.lock().unwrap();
        state.live = false;
        state.connection = None;
        state.cleanup = true;
        state.jobs.clear();
    }
    pub fn submit(
        &self,
        connection: u64,
        sent: u64,
        now: u64,
        request: Request,
    ) -> Result<Pending, ChannelError> {
        if sent > now || now - sent >= LIFETIME_MS {
            return Err(ChannelError::Expired);
        }
        let mut state = self.0.lock().unwrap();
        if !state.live || state.connection != Some(connection) {
            return Err(ChannelError::Closed);
        }
        if state.jobs.len() >= CAPACITY {
            return Err(ChannelError::Full);
        }
        let (response, receiver) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        state.jobs.push_back(Job {
            request,
            expires: sent.saturating_add(LIFETIME_MS),
            cancelled: cancelled.clone(),
            response,
        });
        Ok(Pending {
            response: receiver,
            cancelled,
        })
    }
    pub fn tick<G: GamePort>(
        &self,
        bridge: &mut Bridge<G>,
        now: Instant,
        mut elapsed: impl FnMut() -> u64,
    ) -> usize {
        // Hold the short bounded section so revocation cannot race a validated write.
        // Never wait for a response while holding this lock.
        let mut state = self.0.lock().unwrap();
        if state.cleanup {
            bridge.disconnect();
            state.cleanup = false;
        }
        bridge.expire(now);
        if !state.live {
            return 0;
        }
        state.instance = Some(bridge.instance());
        let mut processed = 0;
        while processed < MAX_PER_TICK {
            let Some(job) = state.jobs.pop_front() else {
                break;
            };
            processed += 1;
            if job.cancelled.load(Ordering::Acquire) {
                continue;
            }
            let response = if elapsed() >= job.expires {
                bridge.reject(job.request.request_id, ErrorCode::Expired)
            } else {
                bridge.handle(job.request, now)
            };
            let _ = job.response.try_send(response);
        }
        processed
    }
}
impl Pending {
    pub fn wait(&self, timeout: Duration) -> Result<Response, ChannelError> {
        self.response
            .recv_timeout(timeout)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => ChannelError::Timeout,
                mpsc::RecvTimeoutError::Disconnected => ChannelError::Closed,
            })
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
