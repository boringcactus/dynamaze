//! Networking logic

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use bincode::{deserialize, serialize};

use crate::{Player, PlayerID};
use crate::menu::NetGameState;

/// A message that can be sent over the network
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// Request the entire game state
    StateRequest,
    /// Join a lobby
    JoinLobby(Player),
    /// Entire game state
    State(NetGameState),
    /// Edit player info
    EditPlayer(PlayerID, Player),
}

/// Tracks the type of connection this is
pub enum ConnectionInfo {
    /// Guest, with host address if connected
    Guest(SocketAddr),
    /// Host, with list of connected clients
    Host(Vec<SocketAddr>),
}

/// Encapsulates connection information and behavior
pub struct Connection {
    /// UDP socket
    pub socket: UdpSocket,
    /// Connection type
    pub info: ConnectionInfo,
}

impl Connection {
    /// Creates a new socket on the given local port
    pub fn new(port: u16, host: Option<SocketAddr>) -> io::Result<Connection> {
        let socket = UdpSocket::bind(("127.0.0.1", port))?;
        let info = match host {
            None => ConnectionInfo::Host(vec![]),
            Some(host) => ConnectionInfo::Guest(host)
        };
        Ok(Connection {
            socket,
            info,
        })
    }

    /// Creates a new socket on the given local port, or something higher if it is in use
    pub fn new_with_backoff(target_port: u16, host: Option<SocketAddr>) -> Connection {
        for port in target_port.. {
            let udp_socket = UdpSocket::bind(("127.0.0.1", port));
            if let Ok(socket) = udp_socket {
                let info = match host {
                    None => ConnectionInfo::Host(vec![]),
                    Some(host) => ConnectionInfo::Guest(host)
                };
                return Connection {
                    socket,
                    info,
                };
            }
        }
        panic!("Failed to find an open port");
    }

    /// Sends a message to the given address
    pub fn send_to(&self, message: &Message, dest: &SocketAddr) {
        let data = serialize(message).expect("Couldn't serialize Message for network delivery");
        self.socket.send_to(&data, dest).expect("Failed to send message");
    }

    fn send_to_all<'a, I>(&self, message: &Message, dests: I) where I: IntoIterator<Item=&'a SocketAddr> {
        dests.into_iter().for_each(|guest| self.send_to(message, guest));
    }

    /// Sends a message to whoever it needs to go to, whether that's the host or all guests
    pub fn send(&self, message: &Message) {
        let dests = match self.info {
            ConnectionInfo::Guest(ref host) => vec![host],
            ConnectionInfo::Host(ref guests) => guests.iter().collect(),
        };
        self.send_to_all(message, dests);
    }

    /// Sends a message to whoever it needs to go to, but ignoring the given address if it would
    /// otherwise be included
    pub fn send_without(&self, message: &Message, filtered_addr: &SocketAddr) {
        let dests = match self.info {
            ConnectionInfo::Guest(ref host) => vec![host],
            ConnectionInfo::Host(ref guests) => guests.iter().collect(),
        };
        let dests = dests.into_iter().filter(|addr| *filtered_addr != **addr);
        self.send_to_all(message, dests);
    }

    /// Receives a message with a long timeout
    pub fn receive(&self) -> (Message, SocketAddr) {
        self.socket.set_read_timeout(Some(Duration::from_secs(10))).expect("Failed to set read timeout");
        let mut buf = [0u8; 65536];
        let (bytes, source) = self.socket.recv_from(&mut buf).expect("Failed to read message");
        let message = deserialize(&buf[..bytes]).expect("Failed to parse message");
        (message, source)
    }

    /// Receives a message with a very short timeout
    pub fn try_receive(&self) -> Option<(Message, SocketAddr)> {
        self.socket.set_read_timeout(Some(Duration::from_millis(5))).expect("Failed to set read timeout");
        let mut buf = [0u8; 65536];
        match self.socket.recv_from(&mut buf) {
            Ok((bytes, source)) => {
                let message = deserialize(&buf[..bytes]).expect("Failed to parse message");
                Some((message, source))
            }
            Err(e) => {
                if let io::ErrorKind::TimedOut = e.kind() {
                    None
                } else {
                    panic!("Unexpected error when peeking for UDP message: {:?}", e);
                }
            }
        }
    }
}
