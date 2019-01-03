//! Board logic

/// Size of game board
const SIZE: usize = 9;

/// Information about board state
pub struct Board {
    /// Cells
    pub cells: [[u8; SIZE]; SIZE],
}

impl Board {
    /// Creates a new board
    pub fn new() -> Board {
        Board {
            cells: [[0; SIZE]; SIZE],
        }
    }

    /// Gets the character at a given index
    pub fn char(&self, ind: [usize; 2]) -> Option<char> {
        match self.cells[ind[1]][ind[0]] {
            1 => Some('1'),
            2 => Some('2'),
            3 => Some('3'),
            4 => Some('4'),
            5 => Some('5'),
            6 => Some('6'),
            7 => Some('7'),
            8 => Some('8'),
            9 => Some('9'),
            _ => None
        }
    }

    /// Sets cell value
    pub fn set(&mut self, ind: [usize; 2], val: u8) {
        self.cells[ind[1]][ind[0]] = val;
    }
}
