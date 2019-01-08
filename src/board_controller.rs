//! Board controller

use piston::input::GenericEvent;

use std::collections::BTreeMap;

use crate::Board;
use crate::BoardView;
use crate::{Player, PlayerID};

/// Handles events for DynaMaze game
pub struct BoardController {
    /// Board state
    pub board: Board,
    /// Mouse position
    pub cursor_pos: [f64; 2],
    /// Players
    pub players: BTreeMap<PlayerID, Player>,
}

impl BoardController {
    /// Creates a new board controller with a new board
    pub fn new(width: usize, height: usize, player_list: Vec<Player>) -> BoardController {
        let players = player_list.into_iter().map(|p| (p.id, p)).collect();
        BoardController {
            board: Board::new(width, height, &players),
            cursor_pos: [0.0; 2],
            players,
        }
    }

    /// Gets the ID of the player whose turn it is
    pub fn active_player_id(&self) -> &PlayerID {
        self.players.keys().nth(0).unwrap()
    }

    /// Handles events
    pub fn event<E: GenericEvent>(&mut self, view: &BoardView, e: &E) {
        use piston::input::{Button, Key, MouseButton};

        if let Some(pos) = e.mouse_cursor_args() {
            self.cursor_pos = pos;
            self.board.loose_tile_position = view.in_insert_guide(&pos, self);
        }

        if let Some(Button::Mouse(button)) = e.press_args() {
            // if clicked inside the loose tile...
            if view.in_loose_tile(&self.cursor_pos, self) {
                // if the tile isn't aligned with a guide, or the button wasn't left...
                if self.board.loose_tile_position.is_none() || button != MouseButton::Left {
                    // rotate the loose tile
                    self.board.loose_tile.rotate();
                } else {
                    // otherwise, insert the tile
                    self.board.insert_loose_tile();
                }
            } else if let Some(pos) = view.in_tile(&self.cursor_pos, self) {
                if self.board.reachable_coords(self.board.player_pos(self.active_player_id())).contains(&pos) {
                    let id = *self.active_player_id();
                    self.board.move_player(&id, pos);
                }
            }
        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            match key {
                Key::Right => self.board = Board::new(self.board.width() + 2, self.board.height(), &self.players),
                Key::Left => self.board = Board::new(self.board.width() - 2, self.board.height(), &self.players),
                Key::Up => self.board = Board::new(self.board.width(), self.board.height() - 2, &self.players),
                Key::Down => self.board = Board::new(self.board.width(), self.board.height() + 2, &self.players),
                _ => {}
            }
        }
    }
}
