//! Networking logic

use crate::Player;
use crate::menu::NetGameState;

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use bincode::{serialize, deserialize};

/// A message that can be sent over the network
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// Request the entire game state
    StateRequest,
    /// Join a lobby
    JoinLobby(Player),
    /// Entire game state
    State(NetGameState),
}

/// Encapsulates a socket that can send and receive messages
pub struct Socket {
    /// UDP socket
    socket: UdpSocket,
}

impl Socket {
    /// Creates a new socket on the given local port
    pub fn new(port: u16) -> io::Result<Socket> {
        Ok(Socket {
            socket: UdpSocket::bind(("127.0.0.1", port))?,
        })
    }

    /// Creates a new socket on the given local port, or something higher if it is in use
    pub fn new_with_backoff(target_port: u16) -> Socket {
        for port in target_port.. {
            let udp_socket = UdpSocket::bind(("127.0.0.1", port));
            if let Ok(socket) = udp_socket {
                return Socket {
                    socket
                };
            }
        }
        panic!("Failed to find an open port");
    }

    /// Gets the local address of this socket
    pub fn local_addr(&self) -> SocketAddr {
        self.socket.local_addr().expect("Failed to retrieve local address")
    }

    /// Sends a message to the given address
    pub fn send_to(&self, message: Message, dest: &SocketAddr) {
        let data = serialize(&message).expect("Couldn't serialize Message for network delivery");
        self.socket.send_to(&data, dest);
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
            },
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
