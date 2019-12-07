//! Board controller

use std::collections::BTreeMap;

use piston::input::GenericEvent;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{Board, BoardView, Direction, Player, PlayerID};
use crate::anim::{self, AnimSync, RotateDir};
use crate::demo;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TurnState {
    /// Insert tile
    InsertTile,
    /// Move token
    MoveToken,
}

/// Controls session-level game settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoardSettings {
    /// Tile width of the board
    pub width: usize,
    /// Tile height of the board
    pub height: usize,
    /// Score required to win
    pub score_limit: u8,
}

/// Handles events for DynaMaze game session
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoardController {
    /// Board state
    pub board: Board,
    /// Mouse position
    pub cursor_pos: [f64; 2],
    /// Highlighted tile
    pub highlighted_tile: (usize, usize),
    /// Players
    pub players: BTreeMap<PlayerID, Player>,
    /// Host
    pub host_id: PlayerID,
    /// Turn order
    pub turn_order: Vec<PlayerID>,
    /// Current turn state
    pub turn_state: TurnState,
    /// Settings
    pub settings: BoardSettings,
}

impl BoardController {
    /// Creates a new board controller with a new board
    pub fn new(settings: BoardSettings, player_list: Vec<Player>, host_id: PlayerID) -> BoardController {
        let width = settings.width;
        let height = settings.height;
        let mut player_ids: Vec<PlayerID> = player_list.iter().map(|p| p.id).collect();
        if !demo::is_demo() {
            player_ids.shuffle(&mut thread_rng());
        }
        let players = player_list.into_iter().map(|p| (p.id, p)).collect();
        let board = Board::new(width, height, &players);
        let highlighted_tile = board.player_pos(player_ids[0]);
        BoardController {
            board,
            cursor_pos: [0.0; 2],
            highlighted_tile,
            players,
            host_id,
            turn_order: player_ids,
            turn_state: TurnState::InsertTile,
            settings,
        }
    }

    /// Gets the effective local ID (the player living here who will be moving soonest)
    pub fn effective_local_id(&self, local_id: PlayerID) -> PlayerID {
        for id in &self.turn_order {
            let player = &self.players[id];
            if player.id == local_id || player.parent == Some(local_id) {
                return player.id;
            }
        }
        local_id
    }

    /// Gets the ID of the player whose turn it is
    pub fn active_player_id(&self) -> PlayerID {
        self.turn_order[0]
    }

    /// Gets the player whose turn it is
    pub fn active_player(&self) -> &Player {
        &self.players[&self.active_player_id()]
    }

    fn move_loose_tile(&mut self, new_loose_tile_position: (Direction, usize)) -> bool {
        let old_loose_tile_position = self.board.loose_tile_position;
        self.board.loose_tile_position = new_loose_tile_position;
        old_loose_tile_position != new_loose_tile_position
    }

    fn rotate_loose_tile(&mut self, dir: RotateDir) -> bool {
        self.board.loose_tile.rotate(match dir {
            RotateDir::CW => Direction::East,
            RotateDir::CCW => Direction::West,
        });
        let sync = AnimSync::Rotate(dir);
        anim::STATE.write().unwrap().apply_send(sync);
        true
    }

    /// Checks if the player whose turn it is lives with this player (equal to or child of)
    pub fn local_turn(&self, local_id: PlayerID) -> bool {
        let active_player = self.active_player();
        let my_turn = active_player.id == local_id;
        let child_turn = active_player.parent == Some(local_id);
        my_turn || child_turn
    }

    /// Handles events, returns whether or not the state may have changed
    pub fn event<E: GenericEvent>(&mut self, view: &BoardView, e: &E, local_id: PlayerID) -> bool {
        use piston::input::{Button, MouseButton, Key};

        // never do anything if this player is not the active player
        if !self.local_turn(local_id) {
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
                if let Some(new_loose_tile_position) = view.in_insert_guide(&pos, self) {
                    dirty = dirty || self.move_loose_tile(new_loose_tile_position);
                }
            }
            if should_move {
                let old_highlighted_tile = self.highlighted_tile;
                self.highlighted_tile = view.in_tile(&pos, self).unwrap_or(self.highlighted_tile);
                dirty = dirty || old_highlighted_tile != self.highlighted_tile;
            }
        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            // handle insert
            if should_insert {
                let newly_dirty = match key {
                    Key::Left => self.handle_insert_key_direction(Direction::West),
                    Key::Right => self.handle_insert_key_direction(Direction::East),
                    Key::Up => self.handle_insert_key_direction(Direction::North),
                    Key::Down => self.handle_insert_key_direction(Direction::South),
                    Key::LShift => self.rotate_loose_tile(RotateDir::CCW),
                    Key::RShift => self.rotate_loose_tile(RotateDir::CW),
                    Key::Space => self.insert_loose_tile(),
                    _ => false
                };
                dirty = dirty || newly_dirty;
            }
            // handle move
            if should_move {
                let newly_dirty = match key {
                    Key::Left => self.handle_move_key_direction(Direction::West),
                    Key::Right => self.handle_move_key_direction(Direction::East),
                    Key::Up => self.handle_move_key_direction(Direction::North),
                    Key::Down => self.handle_move_key_direction(Direction::South),
                    Key::Space => self.attempt_move(self.highlighted_tile),
                    _ => false
                };
                dirty = dirty || newly_dirty;
            }
        }

        if let Some(Button::Mouse(button)) = e.press_args() {
            // if clicked inside the loose tile and should be inserting...
            if view.in_loose_tile(&self.cursor_pos, self) && should_insert {
                // if the tile isn't aligned with a guide, or the button wasn't left...
                if button != MouseButton::Left {
                    // rotate the loose tile
                    self.rotate_loose_tile(RotateDir::CW);
                } else {
                    // otherwise, insert the tile
                    self.insert_loose_tile();
                }
                dirty = true;
            } else if let Some(pos) = view.in_tile(&self.cursor_pos, self) {
                // if clicked inside a tile, if we should be moving...
                if should_move {
                    dirty = dirty || self.attempt_move(pos);
                }
            }
        }

        if let Some(tutorial_step) = &self.board.tutorial_step {
            if dirty && self.winner().is_some() {
                if let Some(next_step) = tutorial_step.next() {
                    next_step.apply(&mut self.board);
                }
            }
        }

        dirty
    }

    fn attempt_move(&mut self, pos: (usize, usize)) -> bool {
        let (row, col) = pos;
        // if that tile is reachable from the active player's position...
        let id = self.active_player_id();
        if self.board.reachable_coords(self.board.player_pos(id)).contains(&pos) {
            // move the active player to the given position
            self.board.move_player(id, pos);
            // if the player has reached their target...
            if self.board.get([col, row]).whose_target == Some(id) {
                // advance the player to the next target
                self.board.player_reached_target(id);
            }
            // advance turn order
            self.turn_state = TurnState::InsertTile;
            self.rotate_turn_order();
            return true;
        }
        false
    }

    fn insert_loose_tile(&mut self) -> bool {
        self.board.insert_loose_tile();
        // advance turn state
        self.turn_state = TurnState::MoveToken;
        true
    }

    fn handle_insert_key_direction(&mut self, move_dir: Direction) -> bool {
        let old_loose_tile_position = self.board.loose_tile_position;
        let guides_x = self.board.width() / 2;
        let guides_y = self.board.height() / 2;
        let new_loose_tile_position = match (move_dir, old_loose_tile_position) {
            (Direction::West, (Direction::East, n)) => {
                let count = guides_x - 1;
                let dir = if n < guides_y / 2 {
                    Direction::North
                } else {
                    Direction::South
                };
                (dir, count)
            }
            (Direction::West, (Direction::West, n)) => (Direction::West, n),
            (Direction::West, (Direction::North, 0)) => (Direction::West, 0),
            (Direction::West, (Direction::South, 0)) => (Direction::West, guides_y - 1),
            (Direction::West, (d, n)) if n > 0 => (d, n.saturating_sub(1)),
            (Direction::East, (Direction::West, n)) => {
                let dir = if n < guides_y / 2 {
                    Direction::North
                } else {
                    Direction::South
                };
                (dir, 0)
            }
            (Direction::East, (Direction::East, n)) => (Direction::East, n),
            (Direction::East, (Direction::North, n)) if n == guides_x - 1 => (Direction::East, 0),
            (Direction::East, (Direction::South, n)) if n == guides_x - 1 => (Direction::East, guides_y - 1),
            (Direction::East, (d, n)) => (d, (n + 1).min(guides_x - 1)),
            (Direction::South, (Direction::North, n)) => {
                let dir = if n < guides_x / 2 {
                    Direction::West
                } else {
                    Direction::East
                };
                (dir, 0)
            }
            (Direction::South, (Direction::South, n)) => (Direction::South, n),
            (Direction::South, (Direction::West, n)) if n == guides_y - 1 => (Direction::South, 0),
            (Direction::South, (Direction::East, n)) if n == guides_y - 1 => (Direction::South, guides_x - 1),
            (Direction::South, (d, n)) => (d, (n + 1).min(guides_y - 1)),
            (Direction::North, (Direction::South, n)) => {
                let count = guides_y - 1;
                let dir = if n < guides_x / 2 {
                    Direction::West
                } else {
                    Direction::East
                };
                (dir, count)
            }
            (Direction::North, (Direction::North, n)) => (Direction::North, n),
            (Direction::North, (Direction::West, 0)) => (Direction::North, 0),
            (Direction::North, (Direction::East, 0)) => (Direction::North, guides_x - 1),
            (Direction::North, (d, n)) => (d, n.saturating_sub(1)),
            _ => {
                unreachable!("bad key")
            }
        };
        self.move_loose_tile(new_loose_tile_position)
    }

    fn handle_move_key_direction(&mut self, direction: Direction) -> bool {
        let orig_highlight = self.highlighted_tile;
        let (row, col) = orig_highlight;
        let new_highlight = match direction {
            Direction::North => (row.saturating_sub(1), col),
            Direction::South => ((row + 1).min(self.board.height() - 1), col),
            Direction::East => (row, (col + 1).min(self.board.width() - 1)),
            Direction::West => (row, col.saturating_sub(1)),
        };
        self.highlighted_tile = new_highlight;
        orig_highlight != new_highlight
    }

    fn rotate_turn_order(&mut self) {
        let mut rest = self.turn_order.split_off(1);
        rest.append(&mut self.turn_order);
        self.turn_order = rest;
        // reset the highlighted tile
        self.highlighted_tile = self.board.player_pos(self.turn_order[0]);
    }

    /// Gets the player who has no targets remaining, if one exists
    pub fn winner(&self) -> Option<&Player> {
        self.board
            .player_tokens
            .iter()
            .filter(|(_, token)| token.score >= self.settings.score_limit)
            .nth(0)
            .map(|(id, _)| &self.players[id])
    }
}
