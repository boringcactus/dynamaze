use std::convert::TryInto;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::{Board, Direction, Player, PlayerID};
use crate::board_controller::{BoardController, BoardSettings};
use crate::colors;
use crate::menu::{ConnectedState, NetGameState};
use crate::net;

pub fn new_conn_state(player_id: PlayerID) -> ConnectedState {
    let settings = BoardSettings {
        score_limit: 1,
        width: 3,
        height: 3,
    };
    let players = vec![Player::new(
        "Player 1".to_string(),
        colors::Color(0.2, 0.4, 0.6),
        player_id,
    )];
    let mut board = BoardController::new(settings, players, player_id);
    TutorialStep::First.apply(&mut board.board);
    let state = NetGameState::Active(board);
    let state = Arc::new(RwLock::new(state));
    let sender = net::NetHandler::run_fake();
    ConnectedState { sender, state }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TutorialStep {
    /// Basic game structure (ideally, first move)
    First,
    /// Pushing the target off the board
    Second,
    /// Wrapping around the board
    Third,
}

impl TutorialStep {
    /// Warms up the given board for this to be the current tutorial step
    pub fn apply(&self, board: &mut Board) {
        match *self {
            TutorialStep::First => {
                board.cells = Board::parse_board(
                    r"
                    ───│───
                ",
                );
                board.loose_tile = '│'.try_into().unwrap();
                board.loose_tile_position = (Direction::North, 1);
                let players = board.player_tokens.keys().collect::<Vec<_>>();
                let my_id = *players[0];
                board.cells[0][6].whose_target = Some(my_id);
                if let Some(token) = board.player_tokens.get_mut(&my_id) {
                    token.position = (0, 0);
                    token.score = 0;
                }
                board.tutorial_step = Some(TutorialStep::First);
            }
            TutorialStep::Second => {
                board.cells = Board::parse_board(
                    r"
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                ",
                );
                board.loose_tile = '┌'.try_into().unwrap();
                board.loose_tile_position = (Direction::East, 2);
                let players = board.player_tokens.keys().collect::<Vec<_>>();
                let my_id = *players[0];
                board.loose_tile.whose_target = Some(my_id);
                if let Some(token) = board.player_tokens.get_mut(&my_id) {
                    token.position = (6, 6);
                    token.score = 0;
                }
                board.tutorial_step = Some(TutorialStep::Second);
            }
            TutorialStep::Third => {
                board.cells = Board::parse_board(
                    r"
                    ┌────┘┘
                    └─┐┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                    ┘┘┘┘┘┘┘
                ",
                );
                board.loose_tile = '─'.try_into().unwrap();
                board.loose_tile_position = (Direction::North, 2);
                let players = board.player_tokens.keys().collect::<Vec<_>>();
                let my_id = *players[0];
                board.cells[2][2].whose_target = Some(my_id);
                if let Some(token) = board.player_tokens.get_mut(&my_id) {
                    token.position = (6, 5);
                    token.score = 0;
                }
                board.tutorial_step = Some(TutorialStep::Third);
            }
        }
    }

    /// Grabs the help text for this step
    pub fn text(&self) -> &str {
        match *self {
            TutorialStep::First => "You're the circle, your target is the striped square.",
            TutorialStep::Second => {
                "Targets can be pushed off the board; if you get to insert your own, put it nearby."
            }
            TutorialStep::Third => {
                "If you push yourself off the board, you'll reappear on the other side."
            }
        }
    }

    /// Gets the next step of the tutorial, if there is one
    pub fn next(&self) -> Option<Self> {
        match *self {
            TutorialStep::First => Some(TutorialStep::Second),
            TutorialStep::Second => Some(TutorialStep::Third),
            TutorialStep::Third => None,
        }
    }
}
