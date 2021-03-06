//! Network control messages
use serde::{Deserialize, Serialize};

pub type GameID = u16;

/// A network control message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MetaMessage {
    Join(GameID),
    Leave,
    Message(Vec<u8>),
}
