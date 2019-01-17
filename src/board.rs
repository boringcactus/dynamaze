//! Board logic

use std::collections::{BTreeMap, HashSet};
use std::mem;
use crate::{Direction, Player, PlayerID, Shape, Tile, Item};
use rand::prelude::*;

/// Information about a player's token on the board
#[derive(Clone)]
pub struct PlayerToken {
    /// ID of player the token is for
    pub player_id: PlayerID,
    /// Position of token (row, col)
    pub position: (usize, usize),
    /// Target items, first pending to last
    pub targets: Vec<Item>,
}

impl PlayerToken {
    /// Create a new token for the given player at the given position
    pub fn new(player: &Player, position: (usize, usize), targets: Vec<Item>) -> PlayerToken {
        PlayerToken {
            player_id: player.id,
            position,
            targets,
        }
    }

    /// Retrieve the player's next target
    pub fn next_target(&self) -> &Item {
        self.targets.first().expect("Player has no targets!")
    }

    /// Indicate that a player has reached their target
    pub fn reached_target(&mut self) {
        self.targets.remove(0);
    }
}

/// Information about board state
pub struct Board {
    /// Cells
    cells: Vec<Vec<Tile>>,
    /// Loose tile
    pub loose_tile: Tile,
    /// Loose tile position
    pub loose_tile_position: Option<(Direction, usize)>,
    /// Player tokens
    pub player_tokens: BTreeMap<PlayerID, PlayerToken>,
}

impl Board {
    /// Creates a new board
    pub fn new(width: usize, height: usize, players: &BTreeMap<PlayerID, Player>) -> Board {
        // build tiles
        let mut cells = vec![];
        for _ in 0..height {
            let mut row = vec![];
            for _ in 0..width {
                row.push(random());
            }
            cells.push(row);
        }
        // set corners
        cells[0][0] = Tile{shape: Shape::L, orientation: Direction::East, item: None};
        cells[0][width - 1] = Tile{shape: Shape::L, orientation: Direction::South, item: None};
        cells[height - 1][0] = Tile{shape: Shape::L, orientation: Direction::North, item: None};
        cells[height - 1][width - 1] = Tile{shape: Shape::L, orientation: Direction::West, item: None};
        // place items
        let mut loose_tile: Tile = random();
        for item in &crate::item::ITEM_LIST {
            let mut rng = thread_rng();
            let row: usize = (0..height).choose(&mut rng).expect("Failed to generate position");
            let col: usize = (0..width).choose(&mut rng).expect("Failed to generate position");
            if let Some(ref old_item) = cells[row][col].item {
                if let Some(ref older_item) = loose_tile.item {
                    println!("Lost {}", older_item.char());
                }
                loose_tile.item = Some(old_item.clone());
            }
            cells[row][col].item = Some(item.clone());
        }
        // create tokens and assign targets
        let player_count = players.len();
        let mut legal_items: Vec<_> = cells.iter()
            .flat_map(|row| row.iter().map(|tile| tile.item.clone()))
            .filter_map(|x| x).collect();
        legal_items.shuffle(&mut thread_rng());
        let player_item_count = legal_items.len() / player_count;
        let player_tokens = players.iter().enumerate().map(move |(i, (_, player))| {
            let position = match i {
                0 => (0, 0),
                1 => (height - 1, width - 1),
                2 => (0, width - 1),
                3 => (height - 1, 0),
                _ => panic!("Too many players"),
            };
            let remaining_items = legal_items.split_off(player_item_count);
            let player_items = mem::replace(&mut legal_items, remaining_items);
            println!("Gave player {:?} targets {:?}", player.id, player_items);
            let result = (player.id, PlayerToken::new(player, position, player_items));
            result
        }).collect();
        Board {
            cells,
            loose_tile,
            loose_tile_position: None,
            player_tokens,
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
            // move all tokens
            let move_dir = dir * Direction::South;
            self.player_tokens = self.player_tokens.iter().map(|(id, token)| {
                let (old_row, old_col) = token.position;
                let should_be_target_idx = match move_dir {
                    Direction::North | Direction::South => old_col,
                    Direction::East | Direction::West => old_row,
                };
                if should_be_target_idx != target_idx {
                    return (*id, token.clone());
                }
                let new_position = if self.valid(token.position, move_dir) {
                    token.position + (dir * Direction::South)
                } else {
                    let (old_row, old_col) = token.position;
                    let (new_row, new_col) = match move_dir {
                        Direction::East | Direction::West => (old_row, (old_col + self.width())) + move_dir,
                        Direction::North | Direction::South => ((old_row + self.height()), old_col) + move_dir,
                    };
                    (new_row % self.width(), new_col % self.height())
                };
                (*id, PlayerToken {
                    player_id: *id,
                    position: new_position,
                    targets: token.targets.clone(),
                })
            }).collect();
        }
    }

    /// Gets the (row, col) position of the given player
    pub fn player_pos(&self, id: &PlayerID) -> (usize, usize) {
        self.player_tokens.get(id).expect("No token for player with given ID").position
    }

    /// Moves the given player to the given (row, col)
    pub fn move_player(&mut self, id: &PlayerID, pos: (usize, usize)) {
        self.player_tokens.get_mut(id).expect("No token for player with given ID").position = pos;
    }

    /// Gets all the coordinates reachable from the given (row, col)
    pub fn reachable_coords(&self, from: (usize, usize)) -> HashSet<(usize, usize)> {
        // result contains everything seen, frontier contains only things not yet scanned
        let mut result = HashSet::new();
        result.insert(from);
        let mut frontier = vec![from];
        // while frontier is nonempty...
        while let Some((curr_row, curr_col)) = frontier.pop() {
            // for each reachable direction...
            for dir in self.cells[curr_row][curr_col].paths() {
                // if it doesn't fall off the board...
                if self.valid((curr_row, curr_col), dir) {
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
        result
    }

    /// Indicates that the given player has reached their target
    pub fn player_reached_target(&mut self, player_id: &PlayerID) {
        self.player_tokens = self.player_tokens.iter().map(|(id, token)| {
            if *player_id != *id {
                return (*id, token.clone());
            }
            let targets = token.targets.clone().split_first().expect("Reached target but no targets left").1.to_vec();
            (*id, PlayerToken {
                player_id: *id,
                position: token.position,
                targets,
            })
        }).collect();
    }
}
