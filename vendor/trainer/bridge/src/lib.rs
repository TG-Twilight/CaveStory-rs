//! Game-main-thread execution. OS transport and access control are not implemented here.
pub mod channel;
pub mod mailbox;
use std::{
    collections::VecDeque,
    marker::PhantomData,
    rc::Rc,
    time::{Duration, Instant},
};
use trainer_protocol::*;

pub const LEASE: Duration = Duration::from_secs(3);
#[derive(Debug)]
pub struct PortError;
pub trait GamePort {
    fn view(&self) -> GameView;
    fn heal(&mut self) -> Result<(), PortError>;
    fn edit(&mut self, _edit: &Edit) -> Result<(), ErrorCode> {
        Err(ErrorCode::Unsupported)
    }
    fn clear_effects(&mut self) {}
    fn target_view(&self, player: u8) -> Option<GameView> {
        (player == 1).then(|| self.view())
    }
    fn target_heal(&mut self, player: u8) -> Result<(), PortError> {
        if player != 1 {
            return Err(PortError);
        }
        self.heal()
    }
    fn target_edit(&mut self, player: u8, edit: &Edit) -> Result<(), ErrorCode> {
        if player != 1 {
            return Err(ErrorCode::WrongPlayer);
        }
        self.edit(edit)
    }
    fn maintain_effects(&mut self) {
        if !self.view().can_heal() {
            self.clear_effects();
        }
    }
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::ReadHealth, Capability::HealOnce]
    }
}
struct Session {
    context_epoch: u64,
    id: Id,
    hello: Request,
    welcome: Response,
    expires: Instant,
    last_request: u64,
    cache: VecDeque<(Request, Response)>,
}

pub struct Bridge<G: GamePort> {
    game: G,
    instance: Id,
    enabled: bool,
    session: Option<Session>,
    sequence: u64,
    _main_thread: PhantomData<Rc<()>>,
}

pub fn random_id() -> Result<Id, getrandom::Error> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes)?;
    Ok(Id(bytes))
}

impl<G: GamePort> Bridge<G> {
    pub fn new(game: G) -> Result<Self, getrandom::Error> {
        Ok(Self {
            game,
            instance: random_id()?,
            enabled: true,
            session: None,
            sequence: 0,
            _main_thread: PhantomData,
        })
    }
    pub fn instance(&self) -> Id {
        self.instance
    }
    /// Swap the short-lived engine adapter while retaining the session and replay cache.
    pub fn map_game<P: GamePort>(self, game: P) -> Bridge<P> {
        Bridge {
            game,
            instance: self.instance,
            enabled: self.enabled,
            session: self.session,
            sequence: self.sequence,
            _main_thread: PhantomData,
        }
    }
    pub fn game(&self) -> &G {
        &self.game
    }
    pub fn game_mut(&mut self) -> &mut G {
        &mut self.game
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.disconnect();
        }
    }
    pub fn disconnect(&mut self) {
        self.session = None;
        self.game.clear_effects();
    }
    pub fn expire(&mut self, now: Instant) {
        if self
            .session
            .as_ref()
            .is_some_and(|session| now >= session.expires)
        {
            self.disconnect();
        }
    }
    /// Explicit game-thread handoff for an ordinary map transfer in the same run.
    /// Keep transport identity and lease; old-context writes/receipts cannot cross.
    /// Never call for loading a save, death/restart, or returning to the title.
    pub fn continue_context(&mut self, context_epoch: u64) {
        if let Some(session) = &mut self.session {
            session.context_epoch = context_epoch;
            session.cache.clear();
        }
    }
    /// Call once per engine tick, even when the world is paused.
    pub fn maintain_context(&mut self) {
        if let Some(session) = &self.session {
            let view = self.game.view();
            if session.context_epoch != view.context_epoch {
                self.disconnect();
            } else {
                self.game.maintain_effects();
            }
        }
    }
    pub fn reject(&self, request_id: u64, code: ErrorCode) -> Response {
        self.response(request_id, Outcome::Rejected { code })
    }
    fn response(&self, request_id: u64, outcome: Outcome) -> Response {
        Response {
            version: VERSION,
            request_id,
            instance: self.instance,
            outcome,
        }
    }
    fn snapshot(&mut self, player: u8) -> Snapshot {
        self.sequence += 1;
        Snapshot {
            sequence: self.sequence,
            view: self.game.target_view(player).expect("validated target"),
        }
    }
    fn validate_target(&self, player: u8, epoch: u64) -> Result<GameView, ErrorCode> {
        let before = self
            .game
            .target_view(player)
            .ok_or(ErrorCode::WrongPlayer)?;
        if before.player != player {
            return Err(ErrorCode::WrongPlayer);
        }
        if before.context_epoch != epoch {
            return Err(ErrorCode::ContextChanged);
        }
        if !before.can_heal() {
            return Err(ErrorCode::NotPlayable);
        }
        Ok(before)
    }
    pub fn handle(&mut self, request: Request, now: Instant) -> Response {
        self.expire(now);
        let id = request.request_id;
        if !self.enabled {
            return self.reject(id, ErrorCode::Disabled);
        }
        if request.version != VERSION {
            return self.reject(id, ErrorCode::VersionMismatch);
        }
        if request.instance != self.instance {
            return self.reject(id, ErrorCode::WrongInstance);
        }
        if id == 0 {
            return self.reject(id, ErrorCode::InvalidRequest);
        }
        if matches!(request.command, Command::Hello {}) {
            if request.session.is_some() {
                return self.reject(id, ErrorCode::InvalidRequest);
            }
            if let Some(session) = &self.session {
                return if request == session.hello {
                    session.welcome.clone()
                } else {
                    self.reject(id, ErrorCode::Busy)
                };
            }
            let session_id = match random_id() {
                Ok(id) => id,
                Err(_) => return self.reject(id, ErrorCode::Disabled),
            };
            let welcome = self.response(
                id,
                Outcome::Welcome {
                    session: session_id,
                    capabilities: self.game.capabilities(),
                },
            );
            self.session = Some(Session {
                context_epoch: self.game.view().context_epoch,
                id: session_id,
                hello: request,
                welcome: welcome.clone(),
                expires: now + LEASE,
                last_request: id,
                cache: VecDeque::new(),
            });
            return welcome;
        }
        let Some(session) = &self.session else {
            return self.reject(id, ErrorCode::InvalidSession);
        };
        if request.session != Some(session.id) {
            return self.reject(id, ErrorCode::InvalidSession);
        }
        if session.context_epoch != self.game.view().context_epoch {
            self.disconnect();
            return self.reject(id, ErrorCode::ContextChanged);
        }
        if let Some((old, response)) = session.cache.iter().find(|(old, _)| old.request_id == id) {
            return if *old == request {
                response.clone()
            } else {
                self.reject(id, ErrorCode::RequestConflict)
            };
        }
        if id <= session.last_request {
            return self.reject(id, ErrorCode::StaleRequest);
        }
        let outcome = match request.command.clone() {
            Command::ReadState { player } => match self.game.target_view(player) {
                Some(_) => Outcome::State {
                    snapshot: self.snapshot(player),
                },
                None => Outcome::Rejected {
                    code: ErrorCode::WrongPlayer,
                },
            },
            Command::Heal {
                player,
                context_epoch,
            } => match self.validate_target(player, context_epoch) {
                Err(code) => Outcome::Rejected { code },
                Ok(before) => {
                    if self.game.target_heal(player).is_err() {
                        Outcome::Rejected {
                            code: ErrorCode::EffectUnconfirmed,
                        }
                    } else {
                        let snapshot = self.snapshot(player);
                        let after = &snapshot.view;
                        if after.player != player
                            || after.context_epoch != context_epoch
                            || !after.can_heal()
                            || !after.health.is_some_and(|h| h.current == h.maximum)
                            || after.health.map(|h| h.maximum) != before.health.map(|h| h.maximum)
                        {
                            Outcome::Rejected {
                                code: ErrorCode::EffectUnconfirmed,
                            }
                        } else {
                            Outcome::Healed { snapshot }
                        }
                    }
                }
            },
            Command::Disconnect {} => {
                self.disconnect();
                Outcome::Disconnected {}
            }
            Command::Edit {
                player,
                context_epoch,
                edit,
            } => {
                let validation = self
                    .validate_target(player, context_epoch)
                    .and_then(|before| {
                        edit.validate(&before)?;
                        self.game.target_edit(player, &edit)?;
                        Ok(before)
                    });
                match validation {
                    Err(code) => Outcome::Rejected { code },
                    Ok(before) => {
                        let snapshot = self.snapshot(player);
                        if edit.confirmed(&before, &snapshot.view) {
                            Outcome::Edited { before, snapshot }
                        } else {
                            Outcome::Rejected {
                                code: ErrorCode::EffectUnconfirmed,
                            }
                        }
                    }
                }
            }
            Command::Hello {} => unreachable!(),
        };
        let response = self.response(id, outcome);
        if let Some(session) = &mut self.session {
            session.last_request = id;
            session.expires = now + LEASE;
            session.cache.push_back((request, response.clone()));
            if session.cache.len() > 64 {
                session.cache.pop_front();
            }
        }
        response
    }
}
