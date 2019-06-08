//! Networking logic

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use bincode::{deserialize, serialize};
use bytes::{BufMut, BytesMut};
use futures::stream;
use tokio::codec::{Decoder, Encoder, Framed};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::sync::mpsc;

use crate::{Player, PlayerID};
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
}

#[derive(Debug)]
enum MessageCodecError {
    AddrParse(::std::net::AddrParseError),
    IO(::std::io::Error),
    Send(mpsc::error::SendError),
    Recv(mpsc::error::RecvError),
}

impl Display for MessageCodecError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MessageCodecError::AddrParse(ref x) => x.fmt(f),
            MessageCodecError::IO(ref x) => x.fmt(f),
            MessageCodecError::Send(ref x) => f.write_fmt(format_args!("Send({})", x)),
            MessageCodecError::Recv(ref x) => f.write_fmt(format_args!("Recv({:?})", x)),
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

impl From<mpsc::error::SendError> for MessageCodecError {
    fn from(e: mpsc::error::SendError) -> Self {
        MessageCodecError::Send(e)
    }
}

impl From<mpsc::error::RecvError> for MessageCodecError {
    fn from(e: mpsc::error::RecvError) -> Self {
        MessageCodecError::Recv(e)
    }
}

struct MessageCodec;

impl Encoder for MessageCodec {
    type Item = Message;
    type Error = MessageCodecError;

    fn encode(&mut self, message: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let data = serialize(&message).expect("Couldn't serialize Message for network delivery");
        let data_len = data.len();
        let data_len = serialize(&data_len).expect("Couldn't serialize message length");

        buf.reserve(data_len.len() + data.len());
        buf.put(data_len);
        buf.put(data);
        Ok(())
    }
}

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = MessageCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let message_len: usize = if buf.len() >= USIZE_NET_LEN {
            deserialize(&buf[..USIZE_NET_LEN]).expect("Failed to parse message length")
        } else {
            return Ok(None);
        };
        let message: Message = if buf.len() >= USIZE_NET_LEN + message_len {
            let frame = buf.split_to(USIZE_NET_LEN + message_len);
            deserialize(&frame[USIZE_NET_LEN..]).expect("Failed to parse message")
        } else {
            return Ok(None);
        };
        Ok(Some(message))
    }
}

// Importantly, wraps its own mutex and does its own cloning
struct SinkPool<S> where S: Sink {
    sinks: Arc<Mutex<Vec<S>>>,
}

impl<S> SinkPool<S> where S: Sink, <S as Sink>::SinkItem: Clone {
    fn new() -> SinkPool<S> {
        SinkPool {
            sinks: Arc::new(Mutex::new(vec![])),
        }
    }

    fn add_sink(&mut self, sink: S) {
        let mut sinks = self.sinks.lock().expect("Failed to lock sinks mutex");
        sinks.push(sink);
    }
}

impl<S> Sink for SinkPool<S> where S: Sink, <S as Sink>::SinkItem: Clone {
    type SinkItem = <S as Sink>::SinkItem;
    type SinkError = <S as Sink>::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> Result<AsyncSink<Self::SinkItem>, Self::SinkError> {
        let mut sinks = self.sinks.lock().expect("Failed to lock sinks mutex");
        sinks.iter_mut()
            .map(|sink| sink.start_send(item.clone()))
            .fold(Ok(AsyncSink::Ready), |a, b| {
                match a {
                    Ok(AsyncSink::Ready) => b,
                    _ => a,
                }
            })
    }

    fn poll_complete(&mut self) -> Result<Async<()>, Self::SinkError> {
        let mut sinks = self.sinks.lock().expect("Failed to lock sinks mutex");
        sinks.iter_mut()
            .map(Sink::poll_complete)
            .fold(Ok(Async::Ready(())), |a, b| {
                match a {
                    Ok(Async::Ready(())) => b,
                    _ => a,
                }
            })
    }

    fn close(&mut self) -> Result<Async<()>, Self::SinkError> {
        let mut sinks = self.sinks.lock().expect("Failed to lock sinks mutex");
        sinks.iter_mut()
            .map(Sink::close)
            .fold(Ok(Async::Ready(())), |a, b| {
                match a {
                    Ok(Async::Ready(())) => b,
                    _ => a,
                }
            })
    }
}

impl<S> Clone for SinkPool<S> where S: Sink, <S as Sink>::SinkItem: Clone {
    fn clone(&self) -> Self {
        Self {
            sinks: self.sinks.clone()
        }
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
    }
    None
}

mod nat {
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::RwLock;

    use get_if_addrs::IfAddr;
    use igd::Gateway;

    #[derive(Debug)]
    pub enum NatError {
        Io(std::io::Error),
        IgdSearch(igd::SearchError),
        IgdAdd(igd::AddAnyPortError),
        Poison(String),
        NoneFound,
    }

    impl From<std::io::Error> for NatError {
        fn from(e: std::io::Error) -> Self {
            NatError::Io(e)
        }
    }

    impl From<igd::SearchError> for NatError {
        fn from(e: igd::SearchError) -> Self {
            NatError::IgdSearch(e)
        }
    }

    impl From<igd::AddAnyPortError> for NatError {
        fn from(e: igd::AddAnyPortError) -> Self {
            NatError::IgdAdd(e)
        }
    }

    impl<T> From<std::sync::PoisonError<T>> for NatError {
        fn from(e: std::sync::PoisonError<T>) -> Self {
            NatError::Poison(format!("{}", e))
        }
    }

    impl Error for NatError {}

    impl Display for NatError {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            match self {
                NatError::Io(e) => e.fmt(f),
                NatError::IgdSearch(e) => e.fmt(f),
                NatError::IgdAdd(e) => e.fmt(f),
                NatError::Poison(e) => e.fmt(f),
                NatError::NoneFound => f.write_str("Could not get local IP"),
            }
        }
    }

    fn bitwise_and(a: [u8; 4], b: [u8; 4]) -> [u8; 4] {
        [a[0] & b[0], a[1] & b[1], a[2] & b[2], a[3] & b[3]]
    }

    fn netmask_equivalent(addr1: Ipv4Addr, addr2: Ipv4Addr, netmask: Ipv4Addr) -> bool {
        let addr1 = addr1.octets();
        let addr2 = addr2.octets();
        let netmask = netmask.octets();
        bitwise_and(addr1, netmask) == bitwise_and(addr2, netmask)
    }

    fn get_local_addr(gateway: &Gateway) -> Result<Ipv4Addr, NatError> {
        for iface in get_if_addrs::get_if_addrs()? {
            let addr: IfAddr = iface.addr;
            if let IfAddr::V4(addr) = addr {
                if netmask_equivalent(addr.ip, *gateway.addr.ip(), addr.netmask) {
                    return Ok(addr.ip);
                }
            }
        }
        Err(NatError::NoneFound)
    }

    #[derive(Clone, Copy, Debug)]
    pub struct ServerInfo {
        pub local_port: u16,
        pub remote_addr: SocketAddrV4,
    }

    fn fetch_info() -> Result<ServerInfo, NatError> {
        let local_port = 12543;

        let gateway = igd::search_gateway(Default::default())?;
        let local_addr = get_local_addr(&gateway)?;
        let local_addr = SocketAddrV4::new(local_addr, local_port);

        let protocol = igd::PortMappingProtocol::TCP;
        let lease_duration = 60 * 60; // 1 hour, in seconds
        let description = "DynaMaze";
        let remote_addr = gateway.get_any_address(protocol, local_addr, lease_duration, description)?;
        Ok(ServerInfo {
            local_port,
            remote_addr,
        })
    }

    pub struct ServerInfoHandle {
        info: RwLock<Option<ServerInfo>>,
    }

    impl ServerInfoHandle {
        pub fn get(&self) -> Result<ServerInfo, NatError> {
            if let Some(x) = *(self.info.read()?) {
                return Ok(x);
            }
            let result = fetch_info()?;
            let mut info = self.info.write()?;
            *info = Some(result);
            Ok(result)
        }
    }

    lazy_static! {
        pub static ref HANDLE: ServerInfoHandle = {
            ServerInfoHandle {
                info: RwLock::new(None)
            }
        };
    }
}

fn handle_error<T: Error>(err: T, state: Arc<RwLock<NetGameState>>) {
    let mut state = state.write().expect("Failed to touch state");
    *state = NetGameState::Error(format!("{}", err));
}

pub fn run_host(state: Arc<RwLock<NetGameState>>, player_id: PlayerID) -> Result<(String, mpsc::Sender<MessageCtrl>), nat::NatError> {
    let conn_info: nat::ServerInfo = nat::HANDLE.get()?;
    let (send, recv) = mpsc::channel(20);
    let ui_thread_sender = send.clone();
    thread::spawn(move || {
        let chain = future::ok(()).map(move |_| {
            let addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), conn_info.local_port);
            let server = match TcpListener::bind(&addr) {
                Ok(x) => x,
                Err(e) => {
                    handle_error(e, state);
                    return;
                }
            };
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
    Ok((format!("{}", conn_info.remote_addr), ui_thread_sender))
}

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
