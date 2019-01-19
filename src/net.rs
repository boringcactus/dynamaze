//! Networking logic

use std::io;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use bincode::{deserialize, serialize};
use netbuf::Buf;

use crate::{Player, PlayerID};
use crate::menu::NetGameState;

const USIZE_NET_LEN: usize = 8;

/// A message that can be sent over the network
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// Join a lobby
    JoinLobby(Player),
    /// Entire game state
    State(NetGameState),
    /// Edit player info
    EditPlayer(PlayerID, Player),
}

/// Maintains a TCP stream and receive buffer
struct TcpConnection {
    /// Socket
    stream: TcpStream,
    /// Receive buffer
    buf: Buf,
}

impl From<TcpStream> for TcpConnection {
    fn from(stream: TcpStream) -> Self {
        TcpConnection {
            stream,
            buf: Buf::new(),
        }
    }
}

impl AsRef<TcpStream> for TcpConnection {
    fn as_ref(&self) -> &TcpStream {
        &self.stream
    }
}

impl TcpConnection {
    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.peer_addr()
    }

    fn send(&mut self, message: &Message) -> io::Result<()> {
        let data = serialize(message).expect("Couldn't serialize Message for network delivery");
        let data_len = data.len();
        let data_len = serialize(&data_len).expect("Couldn't serialize message length");
        let mut buf = Buf::new();
        buf.extend(&data_len);
        buf.extend(&data);
        self.stream.write_all(buf.as_ref())
    }

    fn do_receive(&mut self) -> io::Result<Message> {
        let ref mut stream = self.stream;
        let ref mut buf = self.buf;
        while buf.len() < USIZE_NET_LEN {
            buf.read_from(stream)?;
        }
        let message_len = deserialize(&buf[..USIZE_NET_LEN]).expect("Failed to parse message length");
        buf.consume(USIZE_NET_LEN);
        while buf.len() < message_len {
            buf.read_from(stream)?;
        }
        let message = deserialize(&buf[..message_len]).expect("Failed to parse message");
        buf.consume(message_len);
        Ok(message)
    }

    fn receive(&mut self) -> io::Result<Message> {
        let ref mut stream = self.stream;
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(10))).expect("Failed to set read timeout");
        self.do_receive()
    }

    fn try_receive_impl(&mut self) -> io::Result<Message> {
        let ref mut stream = self.stream;
        stream.set_nonblocking(true)?;
        self.do_receive()
    }

    fn try_receive(&mut self) -> io::Result<Option<Message>> {
        match self.try_receive_impl() {
            Ok(x) => Ok(Some(x)),
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Ok(None),
                _ => Err(e),
            }
        }
    }
}

/// Tracks the type of connection this is
enum ConnectionInfo {
    /// Guest, with host connection
    Guest(TcpConnection),
    /// Host, with server and list of connected clients
    Host(TcpListener, Vec<TcpConnection>),
}

/// Encapsulates connection information and behavior
pub struct Connection {
    /// Connection info
    info: ConnectionInfo,
}

impl Connection {
    /// Creates a new host on the given local port
    pub fn new_host(port: u16) -> io::Result<Connection> {
        let server = TcpListener::bind(("0.0.0.0", port))?;
        server.set_nonblocking(true)?;
        let info = ConnectionInfo::Host(server, vec![]);
        Ok(Connection {
            info,
        })
    }

    /// Creates a new guest
    pub fn new_guest(host: SocketAddr) -> io::Result<Connection> {
        let socket = TcpStream::connect(host)?;
        let info = ConnectionInfo::Guest(socket.into());
        Ok(Connection {
            info,
        })
    }

    /// Gets the port of the host
    pub fn host_port(&self) -> u16 {
        let result = match self.info {
            ConnectionInfo::Guest(ref stream) => stream.peer_addr(),
            ConnectionInfo::Host(ref server, _) => server.local_addr(),
        };
        result.expect("Failed to get address").port()
    }

    fn send_to_all<'a, I>(message: &Message, dests: I) -> io::Result<()> where I: IntoIterator<Item=&'a mut TcpConnection> {
        dests.into_iter().map(|conn| conn.send(message)).fold(Ok(()), |a, b| a.and(b))
    }

    fn streams<'a>(&'a mut self) -> Vec<&'a mut TcpConnection> {
        match self.info {
            ConnectionInfo::Guest(ref mut host) => vec![host],
            ConnectionInfo::Host(_, ref mut guests) => guests.iter_mut().collect(),
        }
    }

    /// Sends a message to whoever it needs to go to, whether that's the host or all guests
    pub fn send(&mut self, message: &Message) -> io::Result<()> {
        let streams = self.streams();
        Self::send_to_all(message, streams)
    }

    /// Sends a message to whoever it needs to go to, but ignoring the given address if it would
    /// otherwise be included
    pub fn send_without(&mut self, message: &Message, filtered_addr: &SocketAddr) -> io::Result<()> {
        let dests = self.streams().into_iter().filter(
            |stream| *filtered_addr != stream.peer_addr().expect("Failed to get address")
        );
        Self::send_to_all(message, dests)
    }

    /// Accepts an incoming connection if we are a host; does nothing if we are a guest
    pub fn accept(&mut self) -> io::Result<()> {
        match self.info {
            ConnectionInfo::Guest(_) => Ok(()),
            ConnectionInfo::Host(ref mut server, ref mut clients) => {
                match server.accept() {
                    Ok((socket, _)) => {
                        clients.push(socket.into());
                        Ok(())
                    },
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => Ok(()),
                        _ => Err(e)
                    }
                }
            }
        }
    }

    /// Receives a message from the host with a long timeout
    pub fn receive(&mut self) -> io::Result<Message> {
        match self.info {
            ConnectionInfo::Host(_, _) => unimplemented!("Called receive() on a host socket!"),
            ConnectionInfo::Guest(ref mut stream) => {
                stream.receive()
            }
        }
    }

    /// Receives a message with a very short timeout
    pub fn try_receive(&mut self) -> io::Result<Option<(Message, SocketAddr)>> {
        for stream in self.streams() {
            match stream.try_receive() {
                Ok(None) => continue,
                Ok(Some(m)) => return Ok(Some((m, stream.peer_addr().expect("Failed to get address")))),
                Err(e) => return Err(e)
            }
        }
        Ok(None)
    }
}
