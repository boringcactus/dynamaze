//! Board logic

use crate::{Direction, Shape, Tile};
use rand::prelude::*;

/// Information about board state
pub struct Board {
    /// Cells
    cells: Vec<Vec<Tile>>,
    /// Loose tile
    pub loose_tile: Tile,
    /// Loose tile position
    pub loose_tile_position: Option<(Direction, usize)>,
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
            loose_tile_position: None,
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

    fn valid(&self, ind: (usize, usize), dir: Direction) -> bool {
        let (j, i) = ind;
        match dir {
            Direction::North => j > 0,
            Direction::South => j < self.height() - 1,
            Direction::West => i > 0,
            Direction::East => i < self.width() - 1,
        }
    }

    /// Inserts the loose tile at its current position
    pub fn insert_loose_tile(&mut self) {
        if let Some((dir, guide_idx)) = self.loose_tile_position {
            let target_idx = 2 * guide_idx + 1;
            // general process: copy into the current position, so start opposite correct margin
            let (mut j, mut i) = match dir {
                Direction::North => (self.height() - 1, target_idx),
                Direction::South => (0, target_idx),
                Direction::West => (target_idx, self.width() - 1),
                Direction::East => (target_idx, 0),
            };
            let next_loose_tile = self.cells[j][i].clone();
            while self.valid((j, i), dir) {
                let (next_j, next_i) = (j, i) + dir;
                self.cells[j][i] = self.cells[next_j][next_i].clone();
                j = next_j;
                i = next_i;
            }
            self.cells[j][i] = self.loose_tile.clone();
            self.loose_tile = next_loose_tile;
        }
    }
}
