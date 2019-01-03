//! Board controller

use piston::input::GenericEvent;

use crate::Board;

/// Handles events for DynaMaze game
pub struct BoardController {
    /// Board state
    pub board: Board,
    /// Selected cell
    pub selection: Option<[usize; 2]>,
    /// Mouse position
    pub cursor_pos: [f64; 2],
}

impl BoardController {
    /// Creates a new board controller
    pub fn new(board: Board) -> BoardController {
        BoardController {
            board,
            selection: None,
            cursor_pos: [0.0; 2],
        }
    }

    /// Handles events
    pub fn event<E: GenericEvent>(&mut self, pos: [f64; 2], size: f64, e: &E) {
        use piston::input::{Button, Key, MouseButton};

        if let Some(pos) = e.mouse_cursor_args() {
            self.cursor_pos = pos;
        }

        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
            // relative coordinates
            let x = self.cursor_pos[0] - pos[0];
            let y = self.cursor_pos[1] - pos[1];
            // if in board...
            if x >= 0.0 && x < size && y >= 0.0 && y < size {
                // ...compute cell position
                let cell_x = (x / size * 9.0) as usize;
                let cell_y = (y / size * 9.0) as usize;
                self.selection = Some([cell_x, cell_y]);
            }
        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            if let Some(ind) = self.selection {
                match key {
                    Key::D1 | Key::NumPad1 => self.board.set(ind, 1),
                    Key::D2 | Key::NumPad2 => self.board.set(ind, 2),
                    Key::D3 | Key::NumPad3 => self.board.set(ind, 3),
                    Key::D4 | Key::NumPad4 => self.board.set(ind, 4),
                    Key::D5 | Key::NumPad5 => self.board.set(ind, 5),
                    Key::D6 | Key::NumPad6 => self.board.set(ind, 6),
                    Key::D7 | Key::NumPad7 => self.board.set(ind, 7),
                    Key::D8 | Key::NumPad8 => self.board.set(ind, 8),
                    Key::D9 | Key::NumPad9 => self.board.set(ind, 9),
                    _ => {}
                }
            }
        }
    }
}
