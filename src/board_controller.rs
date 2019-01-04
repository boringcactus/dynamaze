//! Board controller

use piston::input::GenericEvent;

use crate::Board;
use crate::BoardView;

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
    pub fn event<E: GenericEvent>(&mut self, view: &BoardView, e: &E) {
        use piston::input::{Button, Key, MouseButton};

        if let Some(pos) = e.mouse_cursor_args() {
            self.cursor_pos = pos;
            self.board.loose_tile_position = view.in_insert_guide(&pos, self);
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
            // if clicked inside the loose tile...
            if view.in_loose_tile(&self.cursor_pos, self) {
                // if the tile isn't aligned with a guide...
                if self.board.loose_tile_position.is_none() {
                    // rotate the loose tile
                    self.board.loose_tile.rotate();
                } else {
                    // otherwise, insert the tile
                    self.board.insert_loose_tile();
                }
            }
        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            match key {
                Key::Right => self.board = Board::new(self.board.width() + 2, self.board.height()),
                Key::Left => self.board = Board::new(self.board.width() - 2, self.board.height()),
                Key::Up => self.board = Board::new(self.board.width(), self.board.height() - 2),
                Key::Down => self.board = Board::new(self.board.width(), self.board.height() + 2),
                _ => {}
            }
        }
    }
}
