use std::collections::BTreeMap;
use std::convert::TryInto;
use std::sync::{Arc, RwLock};

use crate::{Board, Direction, Player, PlayerID};
use crate::board::PlayerToken;
use crate::board_controller::{BoardController, BoardSettings};
use crate::colors;
use crate::menu::{ConnectedState, GameState, NetGameState};
use crate::menu_controller::GameController;
use crate::net;

/// Checks to see if the game was launched with the `--demo` argument.
pub fn is_demo() -> bool {
    false
}

/// Creates a demo-friendly GameController
pub fn new_controller() -> GameController {
    let player_id = 1;
    let settings = BoardSettings {
        score_limit: 3,
        width: 0,
        height: 0,
        version: 0,
    };
    let players = vec![
        Player::new(
            "Player 1".to_string(),
            colors::Color(0.2, 0.4, 0.6),
            player_id,
        ),
        Player::new_child(
            "Player 2".to_string(),
            colors::Color(0.4, 0.2, 0.6),
            2,
            player_id,
        ),
        Player::new_child(
            "Player 3".to_string(),
            colors::Color(0.6, 0.2, 0.4),
            3,
            player_id,
        ),
        Player::new_child(
            "Player 4".to_string(),
            colors::Color(0.4, 0.6, 0.2),
            4,
            player_id,
        ),
    ];
    let board = BoardController::new(settings, players, player_id);
    let state = NetGameState::Active(board);
    let state = Arc::new(RwLock::new(state));
    let sender = net::NetHandler::run_fake();
    let state = ConnectedState { sender, state };
    let state = GameState::InGame(state);
    GameController {
        state,
        player_id,
        ..Default::default()
    }
}

/// Creates a demo-friendly board
pub fn new_board(players: &BTreeMap<PlayerID, Player>) -> Board {
    let mut cells = Board::parse_board(
        r"
            ┌┬─┘┐─┐
            ┐│┬┴┌├┘
            │└┘└┤┌┤
            ┌└─┴┤├┐
            └││─┘─┐
            ┌┌└─┤┘┤
            └─┘┬└┬┘
        ",
    );
    let loose_tile = '┤'.try_into().unwrap();
    let loose_tile_position = (Direction::North, 1);
    let height = cells.len();
    let width = cells[0].len();
    let players = players.values().collect::<Vec<_>>();
    cells[2][3].whose_target = Some(players[0].id);
    if players.len() > 1 {
        cells[1][0].whose_target = Some(players[1].id);
    }
    if players.len() > 2 {
        cells[3][4].whose_target = Some(players[2].id);
    }
    if players.len() > 3 {
        cells[0][2].whose_target = Some(players[3].id);
    }
    let player_tokens = players
        .iter()
        .enumerate()
        .map(move |(i, player)| {
            let position = match i {
                0 => (0, 0),
                1 => (height - 1, width - 1),
                2 => (0, width - 1),
                3 => (height - 1, 0),
                _ => panic!("Too many players"),
            };
            (player.id, PlayerToken::new(player, position))
        })
        .collect();
    Board {
        cells,
        loose_tile,
        loose_tile_position,
        player_tokens,
        tutorial_step: None,
    }
}
