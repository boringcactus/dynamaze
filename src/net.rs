//! Networking logic
use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::{Player, PlayerID};
use crate::anim;
use crate::menu::NetGameState;

/// A message that can be sent over the network
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    /// Join a lobby
    JoinLobby(Player),
    /// Entire game state
    State(NetGameState),
    /// Edit player info
    EditPlayer(PlayerID, Player),
    /// Synchronize animation state
    Anim(anim::AnimSync),
}

#[derive(Clone, Debug)]
pub enum MessageCtrl {
    SendGlobal(Box<Message>),
    SendNearGlobal(Box<Message>, i64),
    Disconnect,
}

impl MessageCtrl {
    pub fn send(msg: Message) -> Self {
        MessageCtrl::SendGlobal(Box::new(msg))
    }

    pub fn send_without(msg: Message, user: i64) -> Self {
        MessageCtrl::SendNearGlobal(Box::new(msg), user)
    }

    pub fn should_send(&self, dest: i64) -> bool {
        match self {
            MessageCtrl::SendGlobal(_) => true,
            MessageCtrl::SendNearGlobal(_, user) => *user != dest,
            MessageCtrl::Disconnect => false,
        }
    }
}

impl Into<MessageCtrl> for Message {
    fn into(self) -> MessageCtrl {
        MessageCtrl::SendGlobal(Box::new(self))
    }
}

/// Outgoing message queue
pub type Outbox = VecDeque<MessageCtrl>;
