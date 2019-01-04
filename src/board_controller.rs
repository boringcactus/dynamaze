//! Board controller

use piston::input::GenericEvent;

use crate::Board;
use crate::BoardViewSettings;

/// Handles events for DynaMaze game
pub struct BoardController {
    /// Board state
    pub board: Board,
    /// Mouse position
    pub cursor_pos: [f64; 2],
}

impl BoardController {
    /// Creates a new board controller
    pub fn new(board: Board) -> BoardController {
        BoardController {
            board,
            cursor_pos: [0.0; 2],
        }
    }

    /// Handles events
    pub fn event<E: GenericEvent>(&mut self, _settings: &BoardViewSettings, e: &E) {
        use piston::input::{Button, Key};

        if let Some(pos) = e.mouse_cursor_args() {
            self.cursor_pos = pos;
        }

//        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
//            // relative coordinates
//            let x = self.cursor_pos[0] - pos[0];
//            let y = self.cursor_pos[1] - pos[1];
//            // if in board...
//            if x >= 0.0 && x < size && y >= 0.0 && y < size {
//                // ...compute cell position
//                let cell_x = (x / size * 9.0) as usize;
//                let cell_y = (y / size * 9.0) as usize;
//                self.selection = Some([cell_x, cell_y]);
//            }
//        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            match key {
                Key::Right => self.board = Board::new(self.board.width() + 1, self.board.height()),
                Key::Left => self.board = Board::new(self.board.width() - 1, self.board.height()),
                Key::Up => self.board = Board::new(self.board.width(), self.board.height() - 1),
                Key::Down => self.board = Board::new(self.board.width(), self.board.height() + 1),
                _ => {}
            }
        }
    }
}
