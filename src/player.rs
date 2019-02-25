//! Player information

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
}

impl Player {
    /// Create a new player
    pub fn new(name: String, color: Color, id: PlayerID) -> Player {
        Player {
            name,
            color,
            id,
        }
    }
}
