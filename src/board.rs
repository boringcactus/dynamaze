//! Board logic

use crate::Tile;

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
        let mut cells = [
            [Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random()],
            [Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random()],
            [Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random()],
            [Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random()],
            [Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random()],
            [Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random()],
            [Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random(), Tile::random()],
        ];
        for i in 0..SIZE {
            for j in 0..SIZE {
                cells[j][i] = Tile::random();
            }
        }
        Board {
            cells,
        }
    }

    /// Gets a cell from the board
    pub fn get(&self, ind: [usize; 2]) -> &Tile {
        &self.cells[ind[1]][ind[0]]
    }
}
