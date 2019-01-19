//! Game menu logic

use crate::{Player, PlayerID, BoardController, Connection};
use crate::net::Message;

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
    pub fn new(player_id: PlayerID) -> LobbyInfo {
        LobbyInfo {
            name: "DynaMaze Lobby".into(),
            host: Player::new("Host McHostface".into(), [0.7, 0.2, 0.7, 1.0], player_id),
            guests: vec![],
        }
    }

    /// Retrieves the list of all connected players as references
    pub fn players_ref(&self) -> Vec<&Player> {
        let mut players = vec![&self.host];
        players.append(&mut self.guests.iter().collect());
        players
    }

    /// Retrieves the list of all connected players as clones
    pub fn players(&self) -> Vec<Player> {
        let mut players = vec![self.host.clone()];
        players.append(&mut self.guests.clone());
        players
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
    GameOver(GameOverInfo)
}

impl NetGameState {
    /// Checks if a given player ID belongs to the host
    pub fn is_host(&self, id: &PlayerID) -> bool {
        let host_id = match self {
            NetGameState::Lobby(ref info) => info.host.id,
            NetGameState::Active(ref board_controller) => board_controller.host_id,
            NetGameState::GameOver(ref info) => info.host_id,
        };
        host_id == *id
    }
}

impl NetGameState {
    /// Connects to a lobby running on the given address as the given player
    pub fn join_lobby(socket: &Connection, player: Player) -> NetGameState {
        socket.send(&Message::JoinLobby(player));
        match socket.receive() {
            (Message::State(s), _) => {
                println!("Got state!!!!");
                s
            },
            (m, _) => panic!("Failed to synchronize with host: got {:?}", m),
        }
    }
}

/// State of a connected game
pub struct ConnectedState {
    /// Socket and connection info
    pub connection: Connection,
    /// Game state
    pub state: NetGameState,
}

pub enum GameState {
    /// Main menu
    MainMenu,
    /// Joining, with given host:port
    ConnectMenu(String),
    /// Connected, with given connection info and state
    InGame(ConnectedState),
}
