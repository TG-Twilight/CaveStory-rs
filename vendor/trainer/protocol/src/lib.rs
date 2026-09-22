use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
mod edits;
pub use edits::*;

pub const VERSION: u16 = 3;
pub const MAX_FRAME_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Id(pub [u8; 16]);

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Id(<redacted>)")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u16,
    pub request_id: u64,
    pub instance: Id,
    pub session: Option<Id>,
    pub command: Command,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Hello {},
    ReadState {
        player: u8,
    },
    Heal {
        player: u8,
        context_epoch: u64,
    },
    Edit {
        player: u8,
        context_epoch: u64,
        edit: Edit,
    },
    Disconnect {},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Title,
    Playing,
    Paused,
    Transition,
    Dead,
    Multiplayer,
    Absent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Health {
    pub current: u16,
    pub maximum: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameView {
    pub player: u8,
    pub details: Details,
    pub context_epoch: u64,
    pub stage_id: u16,
    pub status: GameStatus,
    pub health: Option<Health>,
}

impl GameView {
    pub fn can_heal(&self) -> bool {
        matches!(self.status, GameStatus::Playing | GameStatus::Paused)
            && self
                .health
                .is_some_and(|h| h.maximum > 0 && h.current > 0 && h.current <= h.maximum)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub sequence: u64,
    pub view: GameView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadHealth,
    HealOnce,
    EditState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Disabled,
    VersionMismatch,
    WrongInstance,
    InvalidRequest,
    Busy,
    InvalidSession,
    RequestConflict,
    StaleRequest,
    WrongPlayer,
    ContextChanged,
    NotPlayable,
    EffectUnconfirmed,
    Expired,
    InvalidValue,
    Unsupported,
    Capacity,
    Locked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub version: u16,
    pub request_id: u64,
    pub instance: Id,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    Welcome {
        session: Id,
        capabilities: Vec<Capability>,
    },
    State {
        snapshot: Snapshot,
    },
    Healed {
        snapshot: Snapshot,
    },
    Edited {
        before: GameView,
        snapshot: Snapshot,
    },
    Disconnected {},
    Rejected {
        code: ErrorCode,
    },
}

pub fn write_frame<W: Write, T: Serialize>(mut writer: W, value: &T) -> io::Result<()> {
    let bytes =
        serde_json::to_vec(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame length out of range",
        ));
    }
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)
}

pub fn read_frame<R: Read, T: serde::de::DeserializeOwned>(mut reader: R) -> io::Result<T> {
    let mut prefix = [0; 4];
    reader.read_exact(&mut prefix)?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame length out of range",
        ));
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
