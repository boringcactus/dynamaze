//! Tile logic

use rand::prelude::*;

/// Cardinal directions
#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum Direction {
    /// Up
    North,
    /// Down
    South,
    /// Right
    East,
    /// Left
    West,
}

/// Contents of a tile
pub struct Tile {
    /// Directions on which the tile is connected
    pub connections: Vec<Direction>,
}

impl Tile {
    /// Generate a random tile
    pub fn random() -> Self {
        let mut connections = vec![];
        if random() {
            connections.push(Direction::North);
        }
        if random() {
            connections.push(Direction::South);
        }
        if random() {
            connections.push(Direction::East);
        }
        if random() {
            connections.push(Direction::West);
        }
        Tile {
            connections
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            connections: vec![],
        }
    }
}
