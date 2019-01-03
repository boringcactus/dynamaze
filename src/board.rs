//! Board logic

use crate::{Direction, Shape, Tile};
use rand::prelude::*;

/// Size of game board
pub const SIZE: usize = 7;

/// Information about board state
pub struct Board {
    /// Cells
    pub cells: [[Tile; SIZE]; SIZE],
}

impl Board {
    /// Creates a new board
    pub fn new() -> Board {
        let cells = [
            [Tile{shape: Shape::L, orientation: Direction::East}, random(), random(), random(), random(), random(), Tile{shape: Shape::L, orientation: Direction::South}],
            [random(), random(), random(), random(), random(), random(), random()],
            [random(), random(), random(), random(), random(), random(), random()],
            [random(), random(), random(), random(), random(), random(), random()],
            [random(), random(), random(), random(), random(), random(), random()],
            [random(), random(), random(), random(), random(), random(), random()],
            [Tile{shape: Shape::L, orientation: Direction::North}, random(), random(), random(), random(), random(), Tile{shape: Shape::L, orientation: Direction::West}],
        ];
        Board {
            cells,
        }
    }

    /// Gets a cell from the board
    pub fn get(&self, ind: [usize; 2]) -> &Tile {
        &self.cells[ind[1]][ind[0]]
    }
}
