//! Board logic

use std::collections::{BTreeMap, HashSet};

use rand::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{Direction, Outbox, Player, PlayerID, Shape, Tile};
use crate::anim;
use crate::demo;
use crate::tutorial;

/// Information about a player's token on the board
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PlayerToken {
    /// ID of player the token is for
    pub player_id: PlayerID,
    /// Position of token (row, col)
    pub position: (usize, usize),
    /// Number of targets reached
    pub score: u8,
}

impl PlayerToken {
    /// Create a new token for the given player at the given position
    pub fn new(player: &Player, position: (usize, usize)) -> PlayerToken {
        PlayerToken {
            player_id: player.id,
            position,
            score: 0,
        }
    }

    /// Indicate that a player has reached their target
    pub fn reached_target(&mut self) {
        self.score += 1;
    }
}

/// Information about board state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Board {
    /// Cells
    pub cells: Vec<Vec<Tile>>,
    /// Loose tile
    pub loose_tile: Tile,
    /// Loose tile position
    pub loose_tile_position: (Direction, usize),
    /// Player tokens
    pub player_tokens: BTreeMap<PlayerID, PlayerToken>,
    /// Step in tutorial, if any
    pub tutorial_step: Option<tutorial::TutorialStep>,
}

fn avoid_path(tile: &mut Tile, target: Direction) {
    while tile.paths().contains(&target) {
        *tile = random();
    }
}

fn valid_move(ind: (usize, usize), dir: Direction, (width, height): (usize, usize)) -> bool {
    let (j, i) = ind;
    match dir {
        Direction::North => j > 0,
        Direction::South => j < height - 1,
        Direction::West => i > 0,
        Direction::East => i < width - 1,
    }
}

impl Board {
    /// Creates a new board
    pub fn new(width: usize, height: usize, players: &BTreeMap<PlayerID, Player>) -> Board {
        if demo::is_demo() {
            return demo::new_board(players);
        }
        let mut rng = rand::thread_rng();
        // build tiles
        let loose_tile: Tile = rng.gen();
        let mut cells = vec![];
        for _ in 0..height {
            let mut row = vec![];
            for _ in 0..width {
                row.push(rng.gen());
            }
            cells.push(row);
        }
        // set corners
        cells[0][0] = Tile { shape: Shape::L, orientation: Direction::East, whose_target: None };
        cells[0][width - 1] = Tile { shape: Shape::L, orientation: Direction::South, whose_target: None };
        cells[height - 1][0] = Tile { shape: Shape::L, orientation: Direction::North, whose_target: None };
        cells[height - 1][width - 1] = Tile { shape: Shape::L, orientation: Direction::West, whose_target: None };
        // ensure top/bottom fixed tiles point inwards
        for i in 0..width {
            if i % 2 == 0 {
                avoid_path(&mut cells[0][i], Direction::North);
                avoid_path(&mut cells[height - 1][i], Direction::South);
            }
        }
        // ensure left/right fixed tiles point inwards
        #[allow(clippy::needless_range_loop)] for i in 0..height {
            if i % 2 == 0 {
                avoid_path(&mut cells[i][0], Direction::West);
                avoid_path(&mut cells[i][width - 1], Direction::East);
            }
        }
        // create tokens
        let player_tokens = players.values().enumerate().map(move |(i, player)| {
            let mut rng = thread_rng();
            let position = match i {
                0 => (0, 0),
                1 => (height - 1, width - 1),
                2 => (0, width - 1),
                3 => (height - 1, 0),
                _ => (rng.gen_range(0, height), rng.gen_range(0, width)),
            };
            (player.id, PlayerToken::new(player, position))
        }).collect();
        let loose_tile_edge = rng.gen();
        let loose_tile_spot = match loose_tile_edge {
            Direction::North | Direction::South => rng.gen_range(0, height / 2),
            Direction::East | Direction::West => rng.gen_range(0, width / 2),
        };
        // assign next locations
        let mut result = Board {
            cells,
            loose_tile,
            loose_tile_position: (loose_tile_edge, loose_tile_spot),
            player_tokens,
            tutorial_step: None,
        };
        let player_ids = result.player_tokens.keys().cloned().collect::<Vec<_>>();
        for player in &player_ids {
            result.assign_next_target(*player);
        }
        result
    }

    /// Parses a board specified with `│─└┌┐┘├┬┤┴` into an actual matrix of tiles
    pub fn parse_board(spec: &str) -> Vec<Vec<Tile>> {
        use std::convert::TryFrom;
        spec.split_whitespace()
            .filter_map(|line| {
                let result = line.trim();
                if result.is_empty() {
                    None
                } else {
                    Some(result)
                }
            }).map(|line| line.chars().filter_map(|x| Tile::try_from(x).ok()).collect()).collect()
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

    /// Inserts the loose tile at its current position
    pub fn insert_loose_tile(&mut self, outbox: &mut Outbox) {
        let (dir, guide_idx) = self.loose_tile_position;
        let dimensions = (self.width(), self.height());
        let (width, height) = dimensions;
        let target_idx = 2 * guide_idx + 1;
        let sync = anim::AnimSync::Insert(dir * Direction::South, target_idx);
        anim::STATE.write().unwrap().apply_send(sync, outbox);
        // general process: copy into the current position, so start opposite correct margin
        let (mut j, mut i) = match dir {
            Direction::North => (height - 1, target_idx),
            Direction::South => (0, target_idx),
            Direction::West => (target_idx, width - 1),
            Direction::East => (target_idx, 0),
        };
        let next_loose_tile = self.cells[j][i].clone();
        while valid_move((j, i), dir, dimensions) {
            let (next_j, next_i) = (j, i) + dir;
            self.cells[j][i] = self.cells[next_j][next_i].clone();
            j = next_j;
            i = next_i;
        }
        self.cells[j][i] = self.loose_tile.clone();
        self.loose_tile = next_loose_tile;
        self.loose_tile_position.0 *= Direction::South;
        // move all tokens
        let move_dir = dir * Direction::South;
        for token in self.player_tokens.values_mut() {
            let (old_row, old_col) = token.position;
            let should_be_target_idx = match move_dir {
                Direction::North | Direction::South => old_col,
                Direction::East | Direction::West => old_row,
            };
            if should_be_target_idx != target_idx {
                continue;
            }
            token.position = if valid_move(token.position, move_dir, dimensions) {
                token.position + (dir * Direction::South)
            } else {
                let (old_row, old_col) = token.position;
                let (new_row, new_col) = match move_dir {
                    Direction::East | Direction::West => (old_row, (old_col + width)) + move_dir,
                    Direction::North | Direction::South => ((old_row + height), old_col) + move_dir,
                };
                (new_row % width, new_col % height)
            };
        }
    }

    /// Gets the (row, col) position of the given player
    pub fn player_pos(&self, id: PlayerID) -> (usize, usize) {
        self.player_tokens.get(&id).expect("No token for player with given ID").position
    }

    /// Moves the given player to the given (row, col)
    pub fn move_player(&mut self, id: PlayerID, pos: (usize, usize)) {
        self.player_tokens.get_mut(&id).expect("No token for player with given ID").position = pos;
    }

    fn add_reachable_coords(&self, from: (usize, usize), result: &mut HashSet<(usize, usize)>) {
        let dimensions = (self.width(), self.height());
        // result contains everything seen, frontier contains only things not yet scanned
        result.insert(from);
        let mut frontier = vec![from];
        // while frontier is nonempty...
        while let Some((curr_row, curr_col)) = frontier.pop() {
            // for each reachable direction...
            for dir in self.cells[curr_row][curr_col].paths() {
                // if it doesn't fall off the board...
                if valid_move((curr_row, curr_col), dir, dimensions) {
                    // find the connecting tile
                    let (next_row, next_col) = (curr_row, curr_col) + dir;
                    // if that tile connects up as well...
                    if self.cells[next_row][next_col].paths().contains(&(dir * Direction::South)) {
                        // if we've never seen that location before...
                        if !result.contains(&(next_row, next_col)) {
                            // add it to frontier and result
                            frontier.push((next_row, next_col));
                            result.insert((next_row, next_col));
                        }
                    }
                }
            }
        }
    }

    /// Gets all the coordinates reachable from the given (row, col)
    pub fn reachable_coords(&self, from: (usize, usize)) -> HashSet<(usize, usize)> {
        let mut result = HashSet::new();
        self.add_reachable_coords(from, &mut result);
        result
    }

    /// Gets all the coordinates reachable from the given (row, col) or one tile nearby
    pub fn nearly_reachable_coords(&self, from: (usize, usize)) -> HashSet<(usize, usize)> {
        let dimensions = (self.width(), self.height());
        let mut result = HashSet::new();
        // grab all the directly reachable coordinates
        self.add_reachable_coords(from, &mut result);
        let direct_result = result.clone();
        // for everything already found...
        for pos in direct_result {
            // for every direction...
            for dir in Direction::all() {
                // if it doesn't fall off the board...
                if valid_move(pos, *dir, dimensions) {
                    // find the connecting tile
                    let next_pos = pos + *dir;
                    // if we've never seen that location before...
                    if !result.contains(&next_pos) {
                        // run that search from there
                        self.add_reachable_coords(next_pos, &mut result);
                    }
                }
            }
        }
        result
    }

    fn assign_next_target(&mut self, player_id: PlayerID) {
        let mut rng = rand::thread_rng();
        let (old_row, old_col) = self.player_tokens[&player_id].position;
        let all_targets = (0..self.height())
            .flat_map(|row| (0..self.width()).map(move |col| (row, col)))
            .collect::<HashSet<_>>();
        let banned_targets = [(old_row, old_col)].iter()
            .chain(all_targets.iter().filter(|p| self.get([p.1, p.0]).whose_target.is_some()))
            .cloned()
            .collect::<HashSet<_>>();
        let all_targets = &all_targets - &banned_targets;
        let easy_targets = self.nearly_reachable_coords((old_row, old_col));
        let valid_targets = if all_targets.len() > easy_targets.len() {
            &all_targets - &easy_targets
        } else {
            all_targets
        };
        let (row, col) = valid_targets.into_iter().choose(&mut rng).expect("Failed to choose next target");
        self.cells[row][col].whose_target = Some(player_id);
    }

    /// Indicates that the given player has reached their target
    pub fn player_reached_target(&mut self, player_id: PlayerID) {
        if let Some(token) = self.player_tokens.get_mut(&player_id) {
            let (row, col) = token.position;
            self.cells[row][col].whose_target = None;
            token.score += 1;
            self.assign_next_target(player_id);
        }
    }
}
