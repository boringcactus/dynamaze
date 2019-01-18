//! Player information

use std::net;

use graphics::types::Color;

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
    /// Network address
    pub address: net::SocketAddr,
}

impl Player {
    /// Create a new player
    pub fn new(name: String, color: Color, id: PlayerID, address: net::SocketAddr) -> Player {
        Player {
            name,
            color,
            id,
            address,
        }
    }
}
