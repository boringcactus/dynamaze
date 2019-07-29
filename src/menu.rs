//! Game menu logic

use std::net::SocketAddrV4;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{BoardController, Player, PlayerID};
use crate::colors::Color;
use crate::net::{Message, MessageCtrl};
use crate::options::GameOptions;

/// Lobby information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LobbyInfo {
    /// Lobby host
    pub host: Player,
    /// Currently connected players, not including host
    pub guests: Vec<Player>,
    /// Local socket, theoretically usable on LAN, may be Err("Loading...") if loading
    pub local_addr: Result<SocketAddrV4, String>,
    /// Remote socket, theoretically usable across Internet, may be Err("Loading...") if loading
    pub remote_addr: Result<SocketAddrV4, String>,
}

impl LobbyInfo {
    /// Creates a new lobby
    pub fn new(player_id: PlayerID) -> LobbyInfo {
        LobbyInfo {
            host: Player::new("Host McHostface".into(), Color(0.7, 0.2, 0.7), player_id),
            guests: vec![],
            local_addr: Err("Loading...".into()),
            remote_addr: Err("Loading...".into()),
        }
    }

    /// Retrieves the list of all connected players as references
    pub fn players_ref(&self) -> Vec<&Player> {
        let mut players = vec![&self.host];
        players.append(&mut self.guests.iter().collect());
        players
    }

    /// Retrieves the list of all connected players as clones
    pub fn players_cloned(&self) -> Vec<Player> {
        let mut players = vec![self.host.clone()];
        players.append(&mut self.guests.clone());
        players
    }

    /// Gets a player by ID
    pub fn player(&self, id: &PlayerID) -> &Player {
        if self.host.id == *id {
            &self.host
        } else {
            self.guests.iter().filter(|p| p.id == *id).nth(0).expect("Not in lobby!")
        }
    }

    /// Gets a mutable player by ID
    pub fn player_mut(&mut self, id: &PlayerID) -> &mut Player {
        if self.host.id == *id {
            &mut self.host
        } else {
            self.guests.iter_mut().filter(|p| p.id == *id).nth(0).expect("Not in lobby!")
        }
    }
}

/// Endgame information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameOverInfo {
    /// Winning player
    pub winner: Player,
    /// Host ID
    pub host_id: PlayerID,
}

/// Synchronized state of a network game
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetGameState {
    /// Waiting for players to connect
    Lobby(LobbyInfo),
    /// In game
    Active(BoardController),
    /// After game
    GameOver(GameOverInfo),
    /// An error occurred
    Error(String),
}

impl NetGameState {
    /// Checks if a given player ID belongs to the host
    pub fn is_host(&self, id: PlayerID) -> bool {
        let host_id = match self {
            NetGameState::Lobby(ref info) => info.host.id,
            NetGameState::Active(ref board_controller) => board_controller.host_id,
            NetGameState::GameOver(ref info) => info.host_id,
            NetGameState::Error(_) => 0,
        };
        host_id == id
    }
}

impl NetGameState {
    /// Connects to a lobby running on the given address as the given player
    pub fn join_lobby(socket: &mut mpsc::Sender<MessageCtrl>, player: Player) {
        socket.try_send(Message::JoinLobby(player).into()).map_err(|_| ()).expect("Failed to pass message")
    }
}

/// State of a connected game
pub struct ConnectedState {
    /// Message passing mechanism
    pub sender: mpsc::Sender<MessageCtrl>,
    /// Game state
    pub state: Arc<RwLock<NetGameState>>,
}

pub enum GameState {
    /// Main menu
    MainMenu,
    /// Joining, with given host:port
    ConnectMenu(String),
    /// Connected, with given connection info and state
    InGame(ConnectedState),
    /// Errored out in a serious way
    HardError(String),
    /// In options menu
    Options(GameOptions),
}
