//! Menu / global state controller

use std::sync::Arc;
use std::sync::Mutex;

use clipboard::{ClipboardContext, ClipboardProvider};
use piston::input::{GenericEvent, Key};
use rand::prelude::*;

use crate::{Player, PlayerID};
use crate::BoardController;
use crate::GameView;
use crate::menu::{ConnectedState, GameOverInfo, GameState, LobbyInfo, NetGameState};
use crate::net::{self, Message};
use crate::net::MessageCtrl;

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
        }
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
                            let state = NetGameState::Lobby(LobbyInfo::new(self.player_id));
                            let state = Arc::new(Mutex::new(state));
                            let sender = net::run_host(12543, state.clone(), self.player_id);
                            let conn_state = ConnectedState {
                                state,
                                sender,
                            };
                            self.state = GameState::InGame(conn_state);
                        }
                        MouseButton::Right => {
                            self.state = GameState::ConnectMenu("127.0.0.1:12543".into());
                        }
                        _ => (),
                    }
                }
            }
            GameState::ConnectMenu(ref mut address) => {
                if let Some(Button::Keyboard(key)) = e.press_args() {
                    apply_key(address, &key, self.shift);
                }
                if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                    let address = address.parse().expect("Invalid address!");
                    let state = NetGameState::Error("Connecting...".to_string());
                    let state = Arc::new(Mutex::new(state));
                    let mut sender = net::run_guest(address, state.clone(), self.player_id);
                    let mut rng = thread_rng();
                    let r = rng.gen_range(0.0, 1.0);
                    let g = rng.gen_range(0.0, 1.0);
                    let b = rng.gen_range(0.0, 1.0);
                    let player = Player::new("Guesty McGuestface".into(), [r, g, b, 1.0], self.player_id);
                    NetGameState::join_lobby(&mut sender, player);
                    let conn_state = ConnectedState {
                        sender,
                        state,
                    };
                    self.state = GameState::InGame(conn_state);
                } else if let Some(Button::Mouse(MouseButton::Right)) = e.press_args() {
                    let mut ctx: ClipboardContext = ClipboardProvider::new().expect("Failed to paste");
                    *address = ctx.get_contents().expect("Failed to paste");
                }
            }
            GameState::InGame(ref mut conn_state) => {
                let ref mut sender = conn_state.sender;
                let ref mut state = conn_state.state;
                let (broadcast, new_state, new_net_state) = {
                    let mut state = state.lock().expect("Failed to lock state");
                    let is_host = state.is_host(&self.player_id);
                    match *state {
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
                                    (true, None, None)
                                } else {
                                    let message = Message::EditPlayer(self.player_id, player);
                                    sender.try_send(message.into()).map_err(|_| ()).expect("Failed to send message");
                                    (false, None, None)
                                }
                            } else if let Some(Button::Mouse(MouseButton::Right)) = e.press_args() {
                                if is_host {
                                    let players = info.players();
                                    let board_controller = BoardController::new(7, 7, players, info.host.id);
                                    let net_state = NetGameState::Active(board_controller);
                                    (true, None, Some(net_state))
                                } else {
                                    (false, None, None)
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
                                        sender.try_send(message.into()).map_err(|_| ()).expect("Failed to send message");
                                    }
                                }
                                (false, None, None)
                            } else {
                                (false, None, None)
                            }
                        }
                        NetGameState::Active(ref mut board_controller) => {
                            let state_dirty = board_controller.event(&view.board_view, e, &self.player_id);
                            if state_dirty {
                                if let Some(winner) = board_controller.winner() {
                                    let info = GameOverInfo {
                                        winner: winner.clone(),
                                        host_id: board_controller.host_id,
                                    };
                                    (true, None, Some(NetGameState::GameOver(info)))
                                } else {
                                    (true, None, None)
                                }
                            } else {
                                (false, None, None)
                            }
                        }
                        NetGameState::GameOver(_) => {
                            if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                                sender.try_send(MessageCtrl::Disconnect).map_err(|_| ()).expect("Failed to send message");
                                (false, Some(GameState::MainMenu), None)
                            } else {
                                (false, None, None)
                            }
                        }
                        NetGameState::Error(_) => {
                            if let Some(Button::Mouse(MouseButton::Left)) = e.press_args() {
                                sender.try_send(MessageCtrl::Disconnect).map_err(|_| ()).expect("Failed to send message");
                                (false, Some(GameState::MainMenu), None)
                            } else {
                                (false, None, None)
                            }
                        }
                    }
                };
                if let Some(ns) = new_net_state {
                    let mut state = state.lock().expect("Failed to lock state");
                    *state = ns;
                }
                if let Some(s) = new_state {
                    self.state = s;
                }
                if broadcast {
                    self.broadcast_state();
                }
            }
        }
    }

    // TODO maybe don't do this
    fn broadcast_state(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let ref mut sender = conn_state.sender;
            let ref mut state = conn_state.state;
            let state = state.lock().expect("Failed to lock state");
            let message = Message::State(state.clone());
            sender.try_send(message.into()).map_err(|_| ()).expect("Failed to send message");
        }
    }
}
