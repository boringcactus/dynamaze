//! Board controller

use piston::input::GenericEvent;
use rand::prelude::*;

use std::collections::BTreeMap;

use crate::Board;
use crate::BoardView;
use crate::{Player, PlayerID};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TurnState {
    /// Insert tile
    InsertTile,
    /// Move token
    MoveToken,
}

/// Handles events for DynaMaze game session
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoardController {
    /// Board state
    pub board: Board,
    /// Mouse position
    pub cursor_pos: [f64; 2],
    /// Players
    pub players: BTreeMap<PlayerID, Player>,
    /// Host
    pub host_id: PlayerID,
    /// Turn order
    pub turn_order: Vec<PlayerID>,
    /// Current turn state
    pub turn_state: TurnState,
}

impl BoardController {
    /// Creates a new board controller with a new board
    pub fn new(width: usize, height: usize, player_list: Vec<Player>, host_id: PlayerID) -> BoardController {
        let mut player_ids: Vec<PlayerID> = player_list.iter().map(|p| p.id).collect();
        player_ids.shuffle(&mut thread_rng());
        let players = player_list.into_iter().map(|p| (p.id, p)).collect();
        let board = Board::new(width, height, &players);
        BoardController {
            board,
            cursor_pos: [0.0; 2],
            players,
            host_id,
            turn_order: player_ids,
            turn_state: TurnState::InsertTile,
        }
    }

    /// Gets the ID of the player whose turn it is
    pub fn active_player_id(&self) -> &PlayerID {
        &self.turn_order[0]
    }

    /// Handles events, returns whether or not the state may have changed
    pub fn event<E: GenericEvent>(&mut self, view: &BoardView, e: &E, local_id: &PlayerID) -> bool {
        use piston::input::{Button, MouseButton};

        // never do anything if this player is not the active player
        if *local_id != *self.active_player_id() {
            return false;
        }

        let (should_insert, should_move) = match self.turn_state {
            TurnState::InsertTile => (true, false),
            TurnState::MoveToken => (false, true),
        };

        let mut dirty = false;

        if let Some(pos) = e.mouse_cursor_args() {
            self.cursor_pos = pos;
            if should_insert {
                let old_loose_tile_position = self.board.loose_tile_position;
                let new_loose_tile_position = view.in_insert_guide(&pos, self);
                self.board.loose_tile_position = new_loose_tile_position;
                if old_loose_tile_position != new_loose_tile_position {
                    dirty = true;
                }
            }
        }

        if let Some(Button::Mouse(button)) = e.press_args() {
            // if clicked inside the loose tile and should be inserting...
            if view.in_loose_tile(&self.cursor_pos, self) && should_insert {
                // if the tile isn't aligned with a guide, or the button wasn't left...
                if self.board.loose_tile_position.is_none() || button != MouseButton::Left {
                    // rotate the loose tile
                    self.board.loose_tile.rotate();
                } else {
                    // otherwise, insert the tile
                    self.board.insert_loose_tile();
                    // advance turn state
                    self.turn_state = TurnState::MoveToken;
                    // reset tile position
                    self.board.loose_tile_position = None;
                }
                dirty = true;
            } else if let Some(pos) = view.in_tile(&self.cursor_pos, self) {
                // if clicked inside a tile, if we should be moving...
                if should_move {
                    // if that tile is reachable from the active player's position...
                    if self.board.reachable_coords(self.board.player_pos(self.active_player_id())).contains(&pos) {
                        // move the active player to the given position
                        let id = *self.active_player_id();
                        self.board.move_player(&id, pos);
                        // if the player has reached their target...
                        if Some(self.board.player_tokens[&id].next_target()) == self.board.get([pos.1, pos.0]).item.as_ref() {
                            // advance the player to the next target
                            self.board.player_reached_target(&id);
                        }
                        // advance turn order
                        self.turn_state = TurnState::InsertTile;
                        self.rotate_turn_order();
                        dirty = true;
                    }
                }
            }
        }

        // TODO resize the board from the lobby
//        if let Some(Button::Keyboard(key)) = e.press_args() {
//            match key {
//                Key::Right => self.board = Board::new(self.board.width() + 2, self.board.height(), &self.players),
//                Key::Left => self.board = Board::new(self.board.width() - 2, self.board.height(), &self.players),
//                Key::Up => self.board = Board::new(self.board.width(), self.board.height() - 2, &self.players),
//                Key::Down => self.board = Board::new(self.board.width(), self.board.height() + 2, &self.players),
//                _ => {}
//            }
//        }

        dirty
    }

    fn rotate_turn_order(&mut self) {
        let mut rest = self.turn_order.split_off(1);
        rest.append(&mut self.turn_order);
        self.turn_order = rest;
    }

    /// Gets the player who has no targets remaining, if one exists
    pub fn winner(&self) -> Option<&Player> {
        self.board
            .player_tokens
            .iter()
            .filter(|(_, token)| token.targets.is_empty())
            .nth(0)
            .map(|(id, _)| &self.players[id])
    }
}
