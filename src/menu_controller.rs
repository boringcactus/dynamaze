//! Menu / global state controller

use crate::BoardController;
use crate::{Player, PlayerID};
use crate::GameView;
use crate::menu::{GameState, NetGameState, LobbyInfo};
use crate::Socket;
use crate::net::Message;

use rand::prelude::*;
use piston::input::GenericEvent;

/// Handles events for DynaMaze game
pub struct GameController {
    /// Game state
    pub state: GameState,
    /// Current player ID
    pub player_id: PlayerID,
    /// Network socket
    pub socket: Socket,
}

impl GameController {
    /// Creates a new GameController
    pub fn new() -> GameController {
        let player_id = random();
        GameController {
            state: GameState::MainMenu,
            player_id,
            socket: Socket::new_with_backoff(12543),
        }
    }

    /// Handles events
    pub fn event<E: GenericEvent>(&mut self, view: &GameView, e: &E) {
        use piston::input::{Button, MouseButton};

        // TODO find a way to move this to its own thread cleanly
        // TODO handle host/guest reasonably
        if e.after_render_args().is_some() {
            match self.socket.try_receive() {
                Some((Message::StateRequest, source)) => {
                    if let GameState::InGame(ref state) = self.state {
                        self.socket.send_to(Message::State(state.clone()), &source);
                    }
                },
                Some((Message::JoinLobby(player), source)) => {
                    if let GameState::InGame(NetGameState::Lobby(ref mut info)) = self.state {
                        info.guests.push(player);
                        self.socket.send_to(Message::State(NetGameState::Lobby(info.clone())), &source);
                    }
                }
                Some((Message::State(state), _)) => {
                    self.state = GameState::InGame(state);
                }
                None => {}
            }
        }

        match self.state {
            GameState::MainMenu => {
                if let Some(Button::Mouse(button)) = e.press_args() {
                    match button {
                        MouseButton::Left => {
                            self.state = GameState::InGame(NetGameState::Lobby(LobbyInfo::new(self.player_id, self.socket.local_addr())));
                        },
                        MouseButton::Right => {
                            self.state = GameState::ConnectMenu("127.0.0.1:12543".into());
                        },
                        _ => (),
                    }
                }
            },
            GameState::ConnectMenu(ref address) => {
                if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                    println!("Connecting to {:?}", address);
                    let address = address.parse().expect("Invalid address!");
                    let player = Player::new("Guesty McGuestface".into(), [0.2, 0.4, 0.8, 1.0], self.player_id, self.socket.local_addr());
                    let connection = NetGameState::connect(&self.socket, address, player);
                    self.state = GameState::InGame(connection);
                }
            },
            GameState::InGame(NetGameState::Lobby(ref mut info)) => {
                if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                    let mut players = vec![info.host.clone()];
                    players.append(&mut info.guests);
                    let board_controller = BoardController::new(7, 7, players);
                    let net_state = NetGameState::Active(board_controller);
                    self.state = GameState::InGame(net_state);
                }
            },
            GameState::InGame(NetGameState::GameOver(_)) => unimplemented!("Game over isn't real yet"),
            GameState::InGame(NetGameState::Active(ref mut board_controller)) => {
                board_controller.event(&view.board_view, e);
            }
        }
    }
}
