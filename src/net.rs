//! Networking logic
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, RwLock};

use bincode::{deserialize, serialize};
use gloo::events::EventListener;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::{Player, PlayerID};
use crate::anim;
use crate::menu::NetGameState;
pub use crate::meta_net::{GameID, MetaMessage};

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

impl Into<MetaMessage> for Message {
    fn into(self) -> MetaMessage {
        let data = serialize(&self).unwrap_throw();
        MetaMessage::Message(data)
    }
}

fn handle_incoming(
    message: Message,
    state: Arc<RwLock<NetGameState>>,
    player_id: PlayerID,
) -> Option<Message> {
    let mut state = state.write().expect("Failed to acquire state");
    let is_host = state.is_host(player_id);
    match message {
        Message::JoinLobby(player) => {
            if let NetGameState::Lobby(ref mut lobby_info) = *state {
                lobby_info.guests.push(player);
                if is_host {
                    return Some(Message::State(state.clone()));
                }
            }
        }
        Message::EditPlayer(id, player) => {
            if let NetGameState::Lobby(ref mut lobby_info) = *state {
                let p = lobby_info.player_mut(&id);
                *p = player;
            }
        }
        Message::State(new_state) => {
            *state = new_state;
        }
        Message::Anim(sync) => {
            anim::STATE.write().unwrap().apply(sync);
        }
    }
    None
}

pub struct NetHandler {
    socket: Option<web_sys::WebSocket>,
    message_listener: Option<EventListener>,
    queue: Arc<Mutex<VecDeque<MetaMessage>>>,
}

impl Drop for NetHandler {
    fn drop(&mut self) {
        drop(self.message_listener.take());
        if let Some(socket) = &self.socket {
            socket.close().unwrap_throw();
        }
    }
}

impl NetHandler {
    pub fn run(state: Arc<RwLock<NetGameState>>, game: GameID, player: PlayerID) -> NetHandler {
        let is_localhost = {
            let window = web_sys::window().unwrap_throw();
            let location = window.location();
            let hostname = location.hostname().unwrap_throw();
            hostname == "127.0.0.1" || hostname == "localhost"
        };
        let addr = if is_localhost {
            "ws://127.0.0.1:8080"
        } else {
            "ws://api.dynamaze.fun"
        };
        let socket = web_sys::WebSocket::new(addr).unwrap_throw();
        socket.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let queue = {
            let join = MetaMessage::Join(game);
            let mut queue = VecDeque::new();
            queue.push_back(join);
            Arc::new(Mutex::new(queue))
        };
        let reply_queue = queue.clone();
        let message_listener = EventListener::new(&socket, "message", move |event| {
            let event = event
                .dyn_ref::<web_sys::MessageEvent>()
                .expect_throw("Bad message received");
            let data = event.data();
            let data = data
                .dyn_ref::<js_sys::ArrayBuffer>()
                .expect_throw("Bad message received");
            let data = js_sys::Uint8Array::new(data);
            let data = data.to_vec();
            let message = deserialize(&data).expect_throw("Bad message received");
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                "Got message: {:?}",
                message
            )));
            let reply = handle_incoming(message, state.clone(), player);
            if let Some(reply) = reply {
                reply_queue.lock().unwrap().push_back(reply.into());
            }
        });
        NetHandler {
            socket: Some(socket),
            message_listener: Some(message_listener),
            queue,
        }
    }

    pub fn run_fake() -> NetHandler {
        NetHandler {
            socket: None,
            message_listener: None,
            queue: Default::default(),
        }
    }

    pub fn queue(&self) -> Arc<Mutex<VecDeque<MetaMessage>>> {
        self.queue.clone()
    }

    pub fn send<M: Into<MetaMessage>>(&self, message: M) {
        self.queue.lock().unwrap().push_back(message.into());
    }

    pub fn drain_queue(&self) {
        if let Some(socket) = &self.socket {
            if socket.ready_state() != web_sys::WebSocket::OPEN {
                return;
            }
            let mut queue = self.queue.lock().unwrap();
            while let Some(message) = queue.pop_front() {
                let mut data = serialize(&message).expect_throw("Bad message sent");
                match socket.send_with_u8_array(&mut data) {
                    Ok(_) => (),
                    Err(e) => {
                        web_sys::console::error_1(&e);
                    }
                }
            }
        } else {
            let mut queue = self.queue.lock().unwrap();
            while let Some(_) = queue.pop_front() {}
        }
    }
}
