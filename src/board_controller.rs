//! Board controller

use std::collections::BTreeMap;

use piston::input::GenericEvent;
use rand::prelude::*;

use crate::{Board, BoardView, Direction, Player, PlayerID};

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
            settings,
        }
    }

    /// Gets the ID of the player whose turn it is
    pub fn active_player_id(&self) -> PlayerID {
        self.turn_order[0]
    }

    fn move_loose_tile(&mut self, new_loose_tile_position: Option<(Direction, usize)>) -> bool {
        let old_loose_tile_position = self.board.loose_tile_position;
        self.board.loose_tile_position = new_loose_tile_position;
        old_loose_tile_position != new_loose_tile_position
    }

    /// Handles events, returns whether or not the state may have changed
    pub fn event<E: GenericEvent>(&mut self, view: &BoardView, e: &E, local_id: PlayerID) -> bool {
        use piston::input::{Button, MouseButton, Key};

        // never do anything if this player is not the active player
        if local_id != self.active_player_id() {
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
                let new_loose_tile_position = view.in_insert_guide(&pos, self);
                dirty = dirty || self.move_loose_tile(new_loose_tile_position);
            }
        }

        if let Some(Button::Keyboard(key)) = e.press_args() {
            // handle insert
            if should_insert {
                let newly_dirty = match key {
                    Key::Left => self.handle_insert_key_direction(Direction::East),
                    Key::Right => self.handle_insert_key_direction(Direction::West),
                    Key::Up => self.handle_insert_key_direction(Direction::North),
                    Key::Down => self.handle_insert_key_direction(Direction::South),
                    Key::LShift => {
                        self.board.loose_tile.rotate(Direction::West);
                        true
                    }
                    Key::RShift => {
                        self.board.loose_tile.rotate(Direction::East);
                        true
                    }
                    Key::Space => {
                        self.insert_loose_tile();
                        true
                    }
                    _ => false
                };
                dirty = dirty || newly_dirty;
            }
        }

        if let Some(Button::Mouse(button)) = e.press_args() {
            // if clicked inside the loose tile and should be inserting...
            if view.in_loose_tile(&self.cursor_pos, self) && should_insert {
                // if the tile isn't aligned with a guide, or the button wasn't left...
                if self.board.loose_tile_position.is_none() || button != MouseButton::Left {
                    // rotate the loose tile
                    self.board.loose_tile.rotate(Direction::East);
                } else {
                    // otherwise, insert the tile
                    self.insert_loose_tile();
                }
                dirty = true;
            } else if let Some(pos) = view.in_tile(&self.cursor_pos, self) {
                // if clicked inside a tile, if we should be moving...
                if should_move {
                    // if that tile is reachable from the active player's position...
                    if self.board.reachable_coords(self.board.player_pos(self.active_player_id())).contains(&pos) {
                        // move the active player to the given position
                        let id = self.active_player_id();
                        self.board.move_player(id, pos);
                        // if the player has reached their target...
                        if self.board.get([pos.1, pos.0]).whose_target == Some(id) {
                            // advance the player to the next target
                            self.board.player_reached_target(id);
                        }
                        // advance turn order
                        self.turn_state = TurnState::InsertTile;
                        self.rotate_turn_order();
                        dirty = true;
                    }
                }
            }
        }

        dirty
    }

    fn insert_loose_tile(&mut self) {
        self.board.insert_loose_tile();
        // advance turn state
        self.turn_state = TurnState::MoveToken;
        // reset tile position
        self.board.loose_tile_position = None;
    }

    fn handle_insert_key_direction(&mut self, move_dir: Direction) -> bool {
        let old_loose_tile_position = self.board.loose_tile_position;
        let guides_x = self.board.width() / 2;
        let guides_y = self.board.height() / 2;
        let new_loose_tile_position = match (move_dir, old_loose_tile_position) {
            (Direction::East, None) => (Direction::West, guides_y / 2),
            (Direction::East, Some((Direction::East, n))) => {
                let count = guides_x - 1;
                let dir = if n < guides_y / 2 {
                    Direction::North
                } else {
                    Direction::South
                };
                (dir, count)
            }
            (Direction::East, Some((Direction::West, n))) => (Direction::West, n),
            (Direction::East, Some((Direction::North, 0))) => (Direction::West, 0),
            (Direction::East, Some((Direction::South, 0))) => (Direction::West, guides_y - 1),
            (Direction::East, Some((d, n))) if n > 0 => (d, n.saturating_sub(1)),
            (Direction::West, None) => (Direction::East, guides_y / 2),
            (Direction::West, Some((Direction::West, n))) => {
                let dir = if n < guides_y / 2 {
                    Direction::North
                } else {
                    Direction::South
                };
                (dir, 0)
            }
            (Direction::West, Some((Direction::East, n))) => (Direction::East, n),
            (Direction::West, Some((Direction::North, n))) if n == guides_x - 1 => (Direction::East, 0),
            (Direction::West, Some((Direction::South, n))) if n == guides_x - 1 => (Direction::East, guides_y - 1),
            (Direction::West, Some((d, n))) => (d, (n + 1).min(guides_x - 1)),
            (Direction::South, None) => (Direction::South, guides_x / 2),
            (Direction::South, Some((Direction::North, n))) => {
                let dir = if n < guides_x / 2 {
                    Direction::West
                } else {
                    Direction::East
                };
                (dir, 0)
            }
            (Direction::South, Some((Direction::South, n))) => (Direction::South, n),
            (Direction::South, Some((Direction::West, n))) if n == guides_y - 1 => (Direction::South, 0),
            (Direction::South, Some((Direction::East, n))) if n == guides_y - 1 => (Direction::South, guides_x - 1),
            (Direction::South, Some((d, n))) => (d, (n + 1).min(guides_y - 1)),
            (Direction::North, None) => (Direction::North, guides_x / 2),
            (Direction::North, Some((Direction::South, n))) => {
                let count = guides_y - 1;
                let dir = if n < guides_x / 2 {
                    Direction::West
                } else {
                    Direction::East
                };
                (dir, count)
            }
            (Direction::North, Some((Direction::North, n))) => (Direction::North, n),
            (Direction::North, Some((Direction::West, 0))) => (Direction::North, 0),
            (Direction::North, Some((Direction::East, 0))) => (Direction::North, guides_x - 1),
            (Direction::North, Some((d, n))) => (d, n.saturating_sub(1)),
            _ => {
                unreachable!("bad key")
            }
        };
        self.move_loose_tile(Some(new_loose_tile_position))
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
            .filter(|(_, token)| token.score >= self.settings.score_limit)
            .nth(0)
            .map(|(id, _)| &self.players[id])
    }
}
