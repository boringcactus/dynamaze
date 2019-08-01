//! Game menu logic

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::{BoardController, Outbox, Player, PlayerID};
use crate::colors::Color;
use crate::options::GameOptions;

/// Lobby information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LobbyInfo {
    /// Currently connected players
    pub players: Vec<Player>,
    /// Activity join secret (may still be loading)
    pub join_secret: Option<String>,
}

impl LobbyInfo {
    /// Creates a new lobby
    pub fn new(player_id: PlayerID, name: String) -> LobbyInfo {
        LobbyInfo {
            players: vec![Player::new(name, Color(0.7, 0.2, 0.7), player_id)],
            join_secret: None,
        }
    }

    /// Gets a player by ID
    pub fn player(&self, id: PlayerID) -> Option<&Player> {
        self.players.iter().filter(|p| p.id == id).nth(0)
    }

    /// Gets a mutable player by ID
    pub fn player_mut(&mut self, id: PlayerID) -> &mut Player {
        self.players.iter_mut().filter(|p| p.id == id).nth(0).expect("Not in lobby!")
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
    GameOver(GameOverInfo),
    /// An error occurred
    Error(String),
}

/// State of a connected game
pub struct ConnectedState {
    /// Game state
    pub state: Arc<RwLock<NetGameState>>,
    /// Outgoing message queue
    pub outbox: Outbox,
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
