//! Board logic

use crate::{Direction, Shape, Tile};
use rand::prelude::*;

/// Information about board state
pub struct Board {
    /// Cells
    cells: Vec<Vec<Tile>>,
    /// Loose tile
    pub loose_tile: Tile,
}

impl Board {
    /// Creates a new board
    pub fn new(width: usize, height: usize) -> Board {
        let mut cells = vec![];
        for _ in 0..height {
            let mut row = vec![];
            for _ in 0..width {
                row.push(random());
            }
            cells.push(row);
        }
        cells[0][0] = Tile{shape: Shape::L, orientation: Direction::East};
        cells[0][width - 1] = Tile{shape: Shape::L, orientation: Direction::South};
        cells[height - 1][0] = Tile{shape: Shape::L, orientation: Direction::North};
        cells[height - 1][width - 1] = Tile{shape: Shape::L, orientation: Direction::West};
        Board {
            cells,
            loose_tile: random(),
        }
    }

    /// Gets a cell from the board
    pub fn get(&self, ind: [usize; 2]) -> &Tile {
        &self.cells[ind[1]][ind[0]]
    }

    /// Gets the width of the board
    pub fn width(&self) -> usize {
        self.cells[0].len()
    }

    /// Gets the height of the board
    pub fn height(&self) -> usize {
        self.cells.len()
    }
}
