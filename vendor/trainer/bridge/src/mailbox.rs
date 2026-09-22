use crate::{Bridge, GamePort};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc,
    },
    time::{Duration, Instant},
};
use trainer_protocol::{ErrorCode, Request, Response};

pub const QUEUE_CAPACITY: usize = 8;
pub const MAX_PER_TICK: usize = 4;
pub const QUEUE_LIFETIME: Duration = Duration::from_millis(500);

struct Job {
    request: Request,
    expires: Instant,
    cancelled: Arc<AtomicBool>,
    response: SyncSender<Response>,
}
#[derive(Clone)]
pub struct Sender(SyncSender<Job>);
pub struct Inbox(Receiver<Job>);
pub struct Pending {
    response: Receiver<Response>,
    cancelled: Arc<AtomicBool>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError {
    Full,
    Closed,
}

pub fn mailbox() -> (Sender, Inbox) {
    let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
    (Sender(sender), Inbox(receiver))
}
impl Sender {
    pub fn submit(&self, request: Request, now: Instant) -> Result<Pending, SubmitError> {
        let (response, receiver) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let job = Job {
            request,
            expires: now + QUEUE_LIFETIME,
            cancelled: cancelled.clone(),
            response,
        };
        self.0.try_send(job).map_err(|e| match e {
            TrySendError::Full(_) => SubmitError::Full,
            TrySendError::Disconnected(_) => SubmitError::Closed,
        })?;
        Ok(Pending {
            response: receiver,
            cancelled,
        })
    }
}
impl Pending {
    pub fn try_response(&self) -> Result<Response, TryRecvError> {
        self.response.try_recv()
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
impl Inbox {
    pub fn tick<G: GamePort>(&self, bridge: &mut Bridge<G>, now: Instant) -> usize {
        bridge.expire(now);
        let mut processed = 0;
        while processed < MAX_PER_TICK {
            let Ok(job) = self.0.try_recv() else {
                break;
            };
            processed += 1;
            if job.cancelled.load(Ordering::Acquire) {
                continue;
            }
            let response = if now >= job.expires {
                bridge.reject(job.request.request_id, ErrorCode::Expired)
            } else {
                bridge.handle(job.request, now)
            };
            let _ = job.response.try_send(response);
        }
        processed
    }
}
