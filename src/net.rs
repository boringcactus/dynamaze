//! Networking logic
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use futures::channel::mpsc;
use futures::prelude::*;
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

#[derive(Debug)]
enum MessageCodecError {
    AddrParse(::std::net::AddrParseError),
    IO(::std::io::Error),
    Send(mpsc::SendError),
}

impl Display for MessageCodecError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MessageCodecError::AddrParse(ref x) => x.fmt(f),
            MessageCodecError::IO(ref x) => x.fmt(f),
            MessageCodecError::Send(ref x) => f.write_fmt(format_args!("Send({})", x)),
        }
    }
}

impl Error for MessageCodecError {}

impl From<::std::net::AddrParseError> for MessageCodecError {
    fn from(e: ::std::net::AddrParseError) -> Self {
        MessageCodecError::AddrParse(e)
    }
}

impl From<std::io::Error> for MessageCodecError {
    fn from(e: std::io::Error) -> Self {
        MessageCodecError::IO(e)
    }
}

impl From<mpsc::SendError> for MessageCodecError {
    fn from(e: mpsc::SendError) -> Self {
        MessageCodecError::Send(e)
    }
}

#[derive(Clone, Debug)]
pub enum MessageCtrl {
    SendGlobal(Box<Message>),
    SendNearGlobal(Box<Message>, SocketAddr),
    Disconnect,
}

impl MessageCtrl {
    pub fn send(msg: Message) -> Self {
        MessageCtrl::SendGlobal(Box::new(msg))
    }

    pub fn send_without(msg: Message, addr: SocketAddr) -> Self {
        MessageCtrl::SendNearGlobal(Box::new(msg), addr)
    }

    pub fn get_message_if_should_send(self, dest: SocketAddr) -> Option<Message> {
        match self {
            MessageCtrl::SendGlobal(m) => Some(*m),
            MessageCtrl::SendNearGlobal(m, addr) => {
                if addr == dest {
                    None
                } else {
                    Some(*m)
                }
            }
            MessageCtrl::Disconnect => None,
        }
    }
}

impl Into<MessageCtrl> for Message {
    fn into(self) -> MessageCtrl {
        MessageCtrl::SendGlobal(Box::new(self))
    }
}

fn handle_incoming(
    message: Message,
    source: SocketAddr,
    state: Arc<RwLock<NetGameState>>,
    player_id: PlayerID,
) -> Option<MessageCtrl> {
    let mut state = state.write().expect("Failed to acquire state");
    let is_host = state.is_host(player_id);
    match message {
        Message::JoinLobby(player) => {
            if let NetGameState::Lobby(ref mut lobby_info) = *state {
                lobby_info.guests.push(player);
                return Some(MessageCtrl::send(Message::State(state.clone())));
            }
        }
        Message::EditPlayer(id, player) => {
            if let NetGameState::Lobby(ref mut lobby_info) = *state {
                if is_host {
                    lobby_info
                        .guests
                        .iter_mut()
                        .filter(|p| p.id == id)
                        .for_each(|p| *p = player.clone());
                    return Some(MessageCtrl::send(Message::State(state.clone())));
                }
            }
        }
        Message::State(new_state) => {
            // TODO only accept state from active player, probably by connecting player ID to source SocketAddr
            *state = new_state;
            if is_host {
                return Some(MessageCtrl::send_without(
                    Message::State(state.clone()),
                    source,
                ));
            }
        }
        Message::Anim(sync) => {
            anim::STATE.write().unwrap().apply(sync);
        }
    }
    None
}

pub const LOCAL_PORT: u16 = 12543;

pub fn run_dummy(_state: Arc<RwLock<NetGameState>>) -> mpsc::Sender<MessageCtrl> {
    let (send, recv) = mpsc::channel(20);
    recv.map(Ok).forward(futures::sink::drain());
    send
}
