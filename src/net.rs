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

const USIZE_NET_LEN: usize = 8;

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

struct MessageCodec;

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

fn handle_incoming(message: Message, source: SocketAddr, state: Arc<RwLock<NetGameState>>, player_id: PlayerID) -> Option<MessageCtrl> {
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
                    lobby_info.guests.iter_mut().filter(|p| p.id == id).for_each(|p| *p = player.clone());
                    return Some(MessageCtrl::send(Message::State(state.clone())));
                }
            }
        }
        Message::State(new_state) => {
            // TODO only accept state from active player, probably by connecting player ID to source SocketAddr
            *state = new_state;
            if is_host {
                return Some(MessageCtrl::send_without(Message::State(state.clone()), source));
            }
        }
        Message::Anim(sync) => {
            anim::STATE.write().unwrap().apply(sync);
        }
    }
    None
}

pub const LOCAL_PORT: u16 = 12543;

fn handle_error<T: Error>(err: T, state: Arc<RwLock<NetGameState>>) {
    let mut state = state.write().expect("Failed to touch state");
    *state = NetGameState::Error(format!("{}", err));
}

#[cfg(unix)]
pub fn run_host(state: Arc<RwLock<NetGameState>>, player_id: PlayerID) -> mpsc::Sender<MessageCtrl> {
    let (send, recv) = mpsc::channel(20);
    let ui_thread_sender = send.clone();
    thread::spawn(move || {
        let chain = future::ok(()).map(move |_| {
            let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), LOCAL_PORT);
            let server = match TcpListener::bind(&addr) {
                Ok(x) => x,
                Err(e) => {
                    handle_error(e, state);
                    return;
                }
            };
            {
                let mut state = state.write().expect("Failed to touch state");
                if let NetGameState::Lobby(ref mut info) = *state {
                    info.local_addr = conn_info.local_addr;
                    info.remote_addr = conn_info.remote_addr;
                }
            }
            let send = send.clone();
            let err_state = state.clone();
            let mpsc_err_state = state.clone();
            let mut net_sink_pool = SinkPool::new();
            let mpsc_sink_pool = net_sink_pool.clone();
            let (mut server_kill, server_killed) = mpsc::channel(1);
            let (mut mpsc_kill, mpsc_killed) = mpsc::channel(1);
            let net_handler = server.incoming()
                .for_each(move |socket| {
                    let (mut client_kill, client_killed) = mpsc::channel(1);
                    let addr = socket.peer_addr().expect("Failed to get socket peer");
                    let socket = Framed::new(socket, MessageCodec {});
                    let (sink, stream) = socket.split();
                    {
                        let sink = sink.with_flat_map(move |data: MessageCtrl| {
                            if let MessageCtrl::Disconnect = data {
                                println!("Got Disconnect");
                                // ignore errors, because errors mean the channel was already closed
                                client_kill.try_send(()).unwrap_or(());
                            }
                            let message = data.get_message_if_should_send(addr);
                            stream::iter_ok(message)
                        });
                        net_sink_pool.add_sink(sink);
                    }
                    let send = send.clone();
                    let state = state.clone();
                    let err_state = state.clone();

                    let incoming = stream
                        .filter_map(move |message| handle_incoming(message, addr, state.clone(), player_id))
                        .forward(send)
                        .map(|_| ())
                        .map_err(|err| handle_error(err, err_state));
                    let incoming_until_killed = incoming.select2(client_killed.into_future())
                        .map(|_| ())
                        .map_err(|_| ());
                    tokio::spawn(incoming_until_killed);
                    Ok(())
                })
                .map_err(move |err| handle_error(err, err_state));
            let net_handler_until_killed = net_handler.select2(server_killed.into_future())
                .map(|_| ())
                .map_err(|_| ());
            tokio::spawn(net_handler_until_killed);
            let mpsc_handler = recv
                .inspect(move |data: &MessageCtrl| {
                    if let MessageCtrl::Disconnect = data {
                        println!("Got Disconnect");
                        // ignore errors, because errors mean the channel was already closed
                        server_kill.try_send(()).unwrap_or(());
                        mpsc_kill.try_send(()).unwrap_or(());
                    }
                })
                .from_err::<MessageCodecError>()
                .forward(mpsc_sink_pool)
                .map(|_| ())
                .map_err(move |err| handle_error(err, mpsc_err_state));
            let mpsc_handler_until_killed = mpsc_handler.select2(mpsc_killed.into_future())
                .map(|_| ())
                .map_err(|_| ());
            tokio::spawn(mpsc_handler_until_killed);
        });
        tokio::run(chain);
    });
    ui_thread_sender
}

#[cfg(unix)]
pub fn run_guest(host: &str, state: Arc<RwLock<NetGameState>>, player_id: PlayerID) -> mpsc::Sender<MessageCtrl> {
    let (send, recv) = mpsc::channel(20);
    let host = host.to_string();
    let ui_thread_sender = send.clone();
    thread::spawn(move || {
        let err_state = state.clone();
        let net_handler = host.parse::<SocketAddr>().into_future()
            .from_err::<MessageCodecError>()
            .and_then(|addr| TcpStream::connect(&addr).from_err())
            .and_then(move |socket| {
                let addr = socket.peer_addr().expect("Failed to get socket peer");
                let socket = Framed::new(socket, MessageCodec {});
                let (sink, stream) = socket.split();
                let send = send.clone();
                let state = state.clone();
                let err_state = state.clone();
                let mpsc_err_state = state.clone();
                let (mut client_kill, client_killed) = mpsc::channel(1);
                let (mut mpsc_kill, mpsc_killed) = mpsc::channel(1);
                let sink = sink.with_flat_map(move |data: MessageCtrl| {
                    let message = data.get_message_if_should_send(addr);
                    stream::iter_ok(message)
                });

                let incoming = stream
                    .filter_map(move |message| handle_incoming(message, addr, state.clone(), player_id))
                    .forward(send)
                    .map(|_| ())
                    .map_err(|err| handle_error(err, err_state));
                let incoming_until_killed = incoming.select2(client_killed.into_future())
                    .map(|_| ())
                    .map_err(|_| ());
                tokio::spawn(incoming_until_killed);
                let mpsc_handler = recv
                    .inspect(move |data: &MessageCtrl| {
                        if let MessageCtrl::Disconnect = data {
                            println!("Got Disconnect");
                            // ignore errors, because errors mean the channel was already closed
                            client_kill.try_send(()).unwrap_or(());
                            mpsc_kill.try_send(()).unwrap_or(());
                        }
                    })
                    .from_err::<MessageCodecError>()
                    .forward(sink)
                    .map(|_| ())
                    .map_err(move |err| handle_error(err, mpsc_err_state));
                let mpsc_handler_until_killed = mpsc_handler.select2(mpsc_killed.into_future())
                    .map(|_| ())
                    .map_err(|_| ());
                tokio::spawn(mpsc_handler_until_killed);
                Ok(())
            })
            .from_err::<MessageCodecError>()
            .map_err(move |err| handle_error(err, err_state));
        tokio::run(net_handler);
    });
    ui_thread_sender
}

pub fn run_dummy(state: Arc<RwLock<NetGameState>>) -> mpsc::Sender<MessageCtrl> {
    let (send, recv) = mpsc::channel(20);
    recv.map(Ok).forward(futures::sink::drain());
    send
}
