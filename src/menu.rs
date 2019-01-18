//! Game menu logic

use crate::{Player, PlayerID, BoardController, Socket};
use crate::net::Message;

use std::net::SocketAddr;

/// Lobby information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LobbyInfo {
    /// Lobby name
    pub name: String,
    /// Lobby host
    pub host: Player,
    /// Currently connected players, not including host
    pub guests: Vec<Player>,
}

impl LobbyInfo {
    /// Creates a new lobby
    pub fn new(player_id: PlayerID, address: SocketAddr) -> LobbyInfo {
        LobbyInfo {
            name: "DynaMaze Lobby".into(),
            host: Player::new("Host McHostface".into(), [0.7, 0.2, 0.7, 1.0], player_id, address),
            guests: vec![],
        }
    }
}

/// Endgame information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameOverInfo {
    /// Winning player
    pub winner: Player,
}

/// Synchronized state of a network game
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetGameState {
    /// Waiting for players to connect
    Lobby(LobbyInfo),
    /// In game
    Active(BoardController),
    /// After game
    GameOver(GameOverInfo)
}

impl NetGameState {
    /// Connects to a lobby running on the given address as the given player
    pub fn connect(socket: &Socket, address: SocketAddr, player: Player) -> NetGameState {
        socket.send_to(Message::JoinLobby(player), &address);
        match socket.receive() {
            (Message::State(s), _) => {
                println!("Got state!!!!");
                s
            },
            (m, _) => panic!("Failed to synchronize with host: got {:?}", m),
        }
    }
}

pub enum GameState {
    /// Main menu
    MainMenu,
    /// Joining, with given host:port
    ConnectMenu(String),
    /// Connected
    InGame(NetGameState),
}
