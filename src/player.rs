//! Player information

use graphics::types::Color;
use rand::prelude::*;

/// The ID assigned to a player
pub type PlayerID = u64;

/// Information about a player
pub struct Player {
    /// Name
    pub name: String,
    /// Token color
    pub color: Color,
    /// ID
    pub id: PlayerID,
}

impl Player {
    /// Create a new player
    pub fn new(name: String, color: Color) -> Player {
        Player {
            name,
            color,
            id: random(),
        }
    }
}
