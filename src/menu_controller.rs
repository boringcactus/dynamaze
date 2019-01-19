//! Menu / global state controller

use crate::BoardController;
use crate::{Player, PlayerID};
use crate::GameView;
use crate::menu::{GameState, NetGameState, LobbyInfo, ConnectedState};
use crate::Connection;
use crate::net::{Message, ConnectionInfo};

use rand::prelude::*;
use piston::input::GenericEvent;

/// Handles events for DynaMaze game
pub struct GameController {
    /// Game state
    pub state: GameState,
    /// Current player ID
    pub player_id: PlayerID,
}

impl GameController {
    /// Creates a new GameController
    pub fn new() -> GameController {
        let player_id = random();
        GameController {
            state: GameState::MainMenu,
            player_id,
        }
    }

    /// Handles events
    pub fn event<E: GenericEvent>(&mut self, view: &GameView, e: &E) {
        use piston::input::{Button, MouseButton};

        // TODO find a way to move this to its own thread cleanly
        // TODO handle host/guest reasonably
        if e.after_render_args().is_some() {
            if let GameState::InGame(ref mut conn_state) = self.state {
                let ref mut connection = conn_state.connection;
                let ref mut state = conn_state.state;
                let is_host = state.is_host(&self.player_id);
                match connection.try_receive() {
                    Some((Message::StateRequest, source)) => {
                        connection.send_to(&Message::State(state.clone()), &source);
                    },
                    Some((Message::JoinLobby(player), source)) => {
                        if let NetGameState::Lobby(ref mut lobby_info) = state {
                            if let ConnectionInfo::Host(ref mut guests) = connection.info {
                                lobby_info.guests.push(player);
                                guests.push(source);
                                connection.send(&Message::State(NetGameState::Lobby(lobby_info.clone())));
                            }
                        }
                    },
                    Some((Message::EditPlayer(id, player), _)) => {
                        if let NetGameState::Lobby(ref mut lobby_info) = state {
                            if is_host {
                                lobby_info.guests.iter_mut().filter(|p| p.id == id).for_each(|p| *p = player.clone());
                                self.broadcast_state();
                            }
                        }
                    },
                    Some((Message::State(new_state), source)) => {
                        // TODO only accept state from active player, probably by connecting player ID to source SocketAddr
                        *state = new_state;
                        if is_host {
                            connection.send_without(&Message::State(state.clone()), &source);
                        }
                    },
                    None => {},
                }
            }
        }

        match self.state {
            GameState::MainMenu => {
                if let Some(Button::Mouse(button)) = e.press_args() {
                    match button {
                        MouseButton::Left => {
                            let connection = Connection::new(12543, None).expect("Failed to start server on port 12543");
                            let state = NetGameState::Lobby(LobbyInfo::new(self.player_id));
                            let conn_state = ConnectedState {
                                connection,
                                state,
                            };
                            self.state = GameState::InGame(conn_state);
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
                    let address = Some(address.parse().expect("Invalid address!"));
                    let connection = Connection::new_with_backoff(12544, address);
                    let mut rng = thread_rng();
                    let r = rng.gen_range(0.0, 1.0);
                    let g = rng.gen_range(0.0, 1.0);
                    let b = rng.gen_range(0.0, 1.0);
                    let player = Player::new("Guesty McGuestface".into(), [r, g, b, 1.0], self.player_id);
                    let state = NetGameState::join_lobby(&connection, player);
                    let conn_state = ConnectedState {
                        connection,
                        state,
                    };
                    self.state = GameState::InGame(conn_state);
                }
            },
            GameState::InGame(ref mut conn_state) => {
                let ref connection = conn_state.connection;
                let ref mut state = conn_state.state;
                let is_host = state.is_host(&self.player_id);
                match state {
                    NetGameState::Lobby(ref mut info) => {
                        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                            if is_host {
                                let players = info.players();
                                let board_controller = BoardController::new(7, 7, players, info.host.id);
                                let net_state = NetGameState::Active(board_controller);
                                *state = net_state;
                                self.broadcast_state();
                            } else {
                                // TODO don't do this
                                let mut rng = thread_rng();
                                let r = rng.gen_range(0.0, 1.0);
                                let g = rng.gen_range(0.0, 1.0);
                                let b = rng.gen_range(0.0, 1.0);
                                let player = Player::new("Guesty McGuestface".into(), [r, g, b, 1.0], self.player_id);
                                let message = Message::EditPlayer(self.player_id, player);
                                connection.send(&message);
                            }
                        }
                    },
                    NetGameState::GameOver(_) => unimplemented!("Game over isn't real yet"),
                    NetGameState::Active(ref mut board_controller) => {
                        let state_dirty = board_controller.event(&view.board_view, e, &self.player_id);
                        if state_dirty {
                            self.broadcast_state();
                        }
                    }
                }
            }
        }
    }

    // TODO maybe don't do this
    fn broadcast_state(&self) {
        if let GameState::InGame(ref conn_state) = self.state {
            let ref connection = conn_state.connection;
            let ref state = conn_state.state;
            connection.send(&Message::State(state.clone()));
        }
    }
}
