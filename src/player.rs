//! Player information

use serde::{Deserialize, Serialize};

use crate::colors::Color;

/// The ID assigned to a player
pub type PlayerID = u64;

/// Information about a player
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    /// Name
    pub name: String,
    /// Token color
    pub color: Color,
    /// ID
    pub id: PlayerID,
    /// Parent player (player whose ID is attached to the game instance)
    pub parent: Option<PlayerID>,
}

impl Player {
    /// Create a new player
    pub fn new(name: String, color: Color, id: PlayerID) -> Player {
        Player {
            name,
            color,
            id,
            parent: None,
        }
    }

    /// Create a new player with the given parent ID
    pub fn new_child(name: String, color: Color, id: PlayerID, parent: PlayerID) -> Player {
        Player {
            name,
            color,
            id,
            parent: Some(parent),
        }
    }

    /// Checks if the given player has, or has a parent with, the given ID
    pub fn lives_with(&self, target: PlayerID) -> bool {
        self.id == target || self.parent == Some(target)
    }
}
