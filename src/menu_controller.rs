//! Menu / global state controller

use crate::BoardController;
use crate::{Player, PlayerID};
use crate::GameView;
use crate::menu::{GameState, NetGameState, LobbyInfo, ConnectedState, GameOverInfo};
use crate::Connection;
use crate::net::{Message, ConnectionInfo};

use rand::prelude::*;
use piston::input::{GenericEvent, Key};

// TODO don't do this, don't at all do this, why the fuck am i doing this
fn to_char(key: &Key, shift: bool) -> Option<char> {
    use piston::input::Key::*;
    let (result, shift_result) = match key {
        A | NumPadA => ('a', 'A'),
        B | NumPadB => ('b', 'B'),
        C | NumPadC => ('c', 'C'),
        D | NumPadD => ('d', 'D'),
        E | NumPadE => ('e', 'E'),
        F | NumPadF => ('f', 'F'),
        G => ('g', 'G'),
        H => ('h', 'H'),
        I => ('i', 'I'),
        J => ('j', 'J'),
        K => ('k', 'K'),
        L => ('l', 'L'),
        M => ('m', 'M'),
        N => ('n', 'N'),
        O => ('o', 'O'),
        P => ('p', 'P'),
        Q => ('q', 'Q'),
        R => ('r', 'R'),
        S => ('s', 'S'),
        T => ('t', 'T'),
        U => ('u', 'U'),
        V => ('v', 'V'),
        W => ('w', 'W'),
        X => ('x', 'X'),
        Y => ('y', 'Y'),
        Z => ('z', 'Z'),
        D0 | NumPad0 => ('0', ')'),
        D1 | NumPad1 => ('1', '!'),
        D2 | NumPad2 => ('2', '@'),
        D3 | NumPad3 => ('3', '#'),
        D4 | NumPad4 => ('4', '$'),
        D5 | NumPad5 => ('5', '%'),
        D6 | NumPad6 => ('6', '^'),
        D7 | NumPad7 => ('7', '&'),
        D8 | NumPad8 => ('8', '*'),
        D9 | NumPad9 => ('9', '('),
        Space | NumPadSpace => (' ', ' '),
        Period | NumPadPeriod => ('.', '>'),
        Exclaim | NumPadExclam => ('!', '!'),
        Quotedbl => ('"', '"'),
        Hash | NumPadHash => ('#', '#'),
        Dollar => ('$', '$'),
        Percent | NumPadPercent => ('%', '%'),
        Ampersand | NumPadAmpersand => ('&', '&'),
        Quote => ('\'', '"'),
        LeftParen | NumPadLeftParen => ('(', '('),
        RightParen | NumPadRightParen => (')', ')'),
        Asterisk | NumPadMultiply => ('*', '*'),
        Plus | NumPadPlus => ('+', '+'),
        Comma | NumPadComma => (',', '<'),
        Minus | NumPadMinus => ('-', '_'),
        Slash | NumPadDivide => ('/', '?'),
        Semicolon => (';', ':'),
        Less | NumPadLess => ('<', '<'),
        Equals | NumPadEquals | NumPadEqualsAS400 => ('=', '+'),
        Greater | NumPadGreater => ('>', '>'),
        Question => ('?', '?'),
        At | NumPadAt => ('@', '@'),
        LeftBracket => ('[', '{'),
        Backslash => ('\\', '|'),
        RightBracket => (']', '}'),
        Colon | NumPadColon => (':', ':'),
        Caret | NumPadXor => ('^', '^'),
        Underscore => ('_', '_'),
        Backquote => ('`', '~'),
        NumPadLeftBrace => ('{', '{'),
        NumPadRightBrace => ('}', '}'),
        NumPadVerticalBar => ('|', '|'),
        Backspace | Unknown | Tab | Return | Escape | Delete | CapsLock | F1 | F2 | F3 | F4 | F5
            | F6 | F7 | F8 | F9 | F10 | F11 | F12 | F13 | F14 | F15 | F16 | F17 | F18 | F19 | F20
            | F21 | F22 | F23 | F24 | PrintScreen | ScrollLock | Pause | Insert | Home | PageUp
            | PageDown | End | Right | Left | Down | Up | NumLockClear | NumPadEnter | Application
            | Power | Execute | Help | Menu | Select | Stop | Again | Undo | Cut | Copy | Paste
            | Find | Mute | VolumeDown | VolumeUp | AltErase | Sysreq | Cancel | Clear | Prior
            | Return2 | Separator | Out | Oper | ClearAgain | CrSel | ExSel | NumPad00 | NumPad000
            | ThousandsSeparator | DecimalSeparator | CurrencyUnit | CurrencySubUnit | NumPadTab
            | NumPadBackspace | NumPadPower | NumPadDblAmpersand | NumPadDblVerticalBar
            | NumPadMemStore | NumPadMemRecall | NumPadMemClear | NumPadMemAdd | NumPadMemSubtract
            | NumPadMemMultiply | NumPadMemDivide | NumPadPlusMinus | NumPadClear | NumPadClearEntry
            | NumPadBinary | NumPadOctal | NumPadDecimal | NumPadHexadecimal | LCtrl | LShift | LAlt
            | RCtrl | RShift | RAlt | LGui | RGui | Mode | AudioNext | AudioPrev | AudioStop
            | AudioPlay | AudioMute | MediaSelect | Www | Mail | Calculator | Computer | AcSearch
            | AcHome | AcBack | AcBookmarks | AcForward | AcStop | AcRefresh | BrightnessDown
            | BrightnessUp | DisplaySwitch | KbdIllumDown | KbdIllumToggle | KbdIllumUp | Eject
            | Sleep => return None,
    };
    if shift {
        Some(shift_result)
    } else {
        Some(result)
    }
}

fn apply_key(string: &mut String, key: &Key, shift: bool) {
    use piston::input::Key::*;
    if let Some(c) = to_char(key, shift) {
        string.push(c);
        return;
    }
    match key {
        Backspace => {
            string.pop();
        },
        _ => (),
    }
}

/// Handles events for DynaMaze game
pub struct GameController {
    /// Game state
    pub state: GameState,
    /// Current player ID
    pub player_id: PlayerID,
    /// Whether or not the shift key is currently pressed
    shift: bool,
}

impl GameController {
    /// Creates a new GameController
    pub fn new() -> GameController {
        let player_id = random();
        GameController {
            state: GameState::MainMenu,
            player_id,
            shift: false,
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

        if let Some(state) = e.button_args() {
            if state.button == Button::Keyboard(Key::LShift) || state.button == Button::Keyboard(Key::RShift) {
                use piston::input::ButtonState;
                self.shift = state.state == ButtonState::Press;
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
            GameState::ConnectMenu(ref mut address) => {
                if let Some(Button::Keyboard(key)) = e.press_args() {
                    apply_key(address, &key, self.shift);
                }
                if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
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
                        // TODO remember name and color
                        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                            let mut player = info.player(&self.player_id).clone();
                            let mut rng = thread_rng();
                            let r = rng.gen_range(0.0, 1.0);
                            let g = rng.gen_range(0.0, 1.0);
                            let b = rng.gen_range(0.0, 1.0);
                            player.color = [r, g, b, 1.0];
                            if is_host {
                                info.host = player;
                                self.broadcast_state();
                            } else {
                                let message = Message::EditPlayer(self.player_id, player);
                                connection.send(&message);
                            }
                        } else if let Some(Button::Mouse(MouseButton::Right)) = e.press_args() {
                            if is_host {
                                let players = info.players();
                                let board_controller = BoardController::new(7, 7, players, info.host.id);
                                let net_state = NetGameState::Active(board_controller);
                                *state = net_state;
                                self.broadcast_state();
                            }
                        } else if let Some(Button::Keyboard(ref key)) = e.press_args() {
                            let mut player = info.player(&self.player_id).clone();
                            let old_name = player.name.clone();
                            apply_key(&mut player.name, key, self.shift);
                            if player.name != old_name {
                                if is_host {
                                    info.host = player;
                                } else {
                                    let message = Message::EditPlayer(self.player_id, player);
                                    connection.send(&message);
                                }
                            }
                        }
                    },
                    NetGameState::Active(ref mut board_controller) => {
                        let state_dirty = board_controller.event(&view.board_view, e, &self.player_id);
                        if state_dirty {
                            if let Some(winner) = board_controller.winner() {
                                let info = GameOverInfo {
                                    winner: winner.clone(),
                                    host_id: board_controller.host_id,
                                };
                                *state = NetGameState::GameOver(info);
                            }
                            self.broadcast_state();
                        }
                    }
                    NetGameState::GameOver(_) => {
                        if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                            self.state = GameState::MainMenu;
                        }
                    },
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
