//! Menu / global state controller

use std::sync::{Arc, RwLock};

use rand::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d as Context;

use crate::{BoardController, BoardSettings, GameView, Player, PlayerID};
use crate::anim;
use crate::demo;
use crate::menu::{ConnectedState, GameOverInfo, GameState, LobbyInfo, NetGameState};
use crate::net::{self, Message, MessageCtrl};
use crate::options;
use crate::sound::{self, SoundEngine};
use crate::tutorial;

fn get_context(main: &web_sys::Element) -> Context {
    let canvas = main.query_selector("canvas").unwrap_throw().unwrap_throw();
    let canvas = canvas.dyn_ref::<web_sys::HtmlCanvasElement>().unwrap_throw();
    let ctx = canvas.get_context("2d").unwrap_throw().unwrap_throw();
    let ctx = ctx.dyn_ref::<Context>().unwrap_throw();
    ctx.clone()
}

/// Handles events for DynaMaze game
pub struct GameController {
    /// Game state
    pub state: GameState,
    /// Current player ID
    pub player_id: PlayerID,
    /// Active player ID the last time the state was checked for a notification
    pub last_player: Option<PlayerID>,
    /// View
    pub view: GameView,
    /// Sound controller
    pub sound_engine: SoundEngine,
}

impl GameController {
    /// Creates a new GameController
    pub fn new() -> GameController {
        if demo::is_demo() {
            return demo::new_controller();
        }
        let player_id = random();
        let sound_engine = SoundEngine::new();
        sound_engine.play_music(sound::Music::Menu);
        GameController {
            state: GameState::MainMenu,
            player_id,
            last_player: None,
            view: GameView::new(),
            sound_engine,
        }
    }

    fn tutorial(&mut self) {
        self.state = GameState::InGame(tutorial::new_conn_state(self.player_id));
    }

    fn host(&mut self) {
        let state = NetGameState::Lobby(LobbyInfo::new(self.player_id));
        let state = Arc::new(RwLock::new(state));
        let sender = net::run_dummy(state.clone());
        anim::STATE.write().unwrap().set_send(sender.clone());
        let conn_state = ConnectedState {
            state,
            sender,
        };
        self.state = GameState::InGame(conn_state);
    }

    fn connect(&mut self) {
        self.state = GameState::ConnectMenu("127.0.0.1:12543".into());
    }

    fn enter_options(&mut self) {
        self.state = GameState::Options(options::HANDLE.fetch().clone());
    }

    fn do_connect(&mut self) {
        if let GameState::ConnectMenu(ref address) = self.state {
            let state = NetGameState::Error("Connecting...".to_string());
            let state = Arc::new(RwLock::new(state));
            let mut sender = net::run_dummy(state.clone());
            anim::STATE.write().unwrap().set_send(sender.clone());
            let player = Player::new("Guesty McGuestface".into(), random(), self.player_id);
            NetGameState::join_lobby(&mut sender, player);
            let conn_state = ConnectedState {
                sender,
                state,
            };
            self.state = GameState::InGame(conn_state);
        }
    }

    fn save_options(&mut self) {
        if let GameState::Options(ref opts) = self.state {
            options::HANDLE.save(opts);
            self.state = GameState::MainMenu;
            self.sound_engine.fetch_volume();
        }
    }

    fn randomize_color(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            let is_host = state.is_host(self.player_id);
            if let NetGameState::Lobby(ref mut info) = *state {
                let player = info.player_mut(&self.player_id);
                player.color = random();
                if is_host {
                    drop(state);
                    self.broadcast_state();
                } else {
                    let message = Message::EditPlayer(self.player_id, player.clone());
                    sender.try_send(message.into()).map_err(|_| ()).expect("Failed to send message");
                }
            }
        }
    }

    fn set_own_name(&mut self, new_name: &str) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            let is_host = state.is_host(self.player_id);
            if let NetGameState::Lobby(ref mut info) = *state {
                let player = info.player_mut(&self.player_id);
                player.name = new_name.to_string();
                if is_host {
                    drop(state);
                    self.broadcast_state();
                } else {
                    let message = Message::EditPlayer(self.player_id, player.clone());
                    sender.try_send(message.into()).map_err(|_| ()).expect("Failed to send message");
                }
            }
        }
    }

    fn new_local_player(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            let is_host = state.is_host(self.player_id);
            if let NetGameState::Lobby(ref mut info) = *state {
                let me = info.player(&self.player_id);
                let child = Player::new_child(me.name.clone(), me.color, random(), me.id);
                info.guests.push(child.clone());
                if is_host {
                    drop(state);
                    self.broadcast_state();
                } else {
                    sender.try_send(Message::JoinLobby(child).into()).map_err(|_| ()).expect("Failed to pass message")
                }
            }
        }
    }

    fn start_hosted_game(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            let is_host = state.is_host(self.player_id);
            if let NetGameState::Lobby(ref mut info) = *state {
                if is_host {
                    let players = info.players_cloned();
                    // TODO edit these
                    let settings = BoardSettings {
                        width: 7,
                        height: 7,
                        score_limit: 10,
                    };
                    let board_controller = BoardController::new(settings, players, info.host.id);
                    let net_state = NetGameState::Active(board_controller);
                    *state = net_state;
                    drop(state);
                    self.broadcast_state();
                }
            }
        }
    }

    fn main_menu(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            sender.try_send(MessageCtrl::Disconnect).map_err(|e| println!("{:?}", e)).unwrap_or(());
            println!("Attempted to disconnect");
        }
        self.sound_engine.fetch_volume();
        self.state = GameState::MainMenu;
    }

    /// Handles tick
    pub fn on_tick(&mut self, dt: f64) {
        anim::STATE.write().unwrap().advance_by(dt);

        let old_last_player = self.last_player;

        let music = match self.state {
            GameState::MainMenu | GameState::ConnectMenu(_) |
            GameState::HardError(_) | GameState::Options(_) => {
                self.last_player = None;
                sound::Music::Menu
            }
            GameState::InGame(ref conn_state) => {
                let state = conn_state.state.read().unwrap();
                match *state {
                    NetGameState::Active(ref board) => {
                        self.last_player = Some(board.active_player_id());
                        sound::Music::InGame
                    }
                    _ => {
                        self.last_player = None;
                        sound::Music::Menu
                    }
                }
            }
        };
        self.sound_engine.play_music(music);

        if old_last_player != self.last_player && self.last_player == Some(self.player_id) {
            self.sound_engine.play_sound(sound::Sound::YourTurn);
        }
    }

    /// Handles click event
    pub fn on_click(&mut self, event: &web_sys::MouseEvent, main: &web_sys::Element) {
        web_sys::console::log_1(&JsValue::from_str("clicking in menu"));
        if let GameState::InGame(ref mut conn_state) = self.state {
            let state = &mut conn_state.state;
            let (broadcast, new_state, new_net_state) = {
                let mut state = state.write().expect("Failed to lock state");
                if let NetGameState::Active(ref mut board_controller) = *state {
                    let state_dirty = board_controller.on_click(event, self.player_id, &self.view.board_view, &get_context(main));
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("clicked in board"));
                    if state_dirty {
                        event.prevent_default();
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
                } else {
                    (false, None, None)
                }
            };
            if let Some(ns) = new_net_state {
                let mut state = state.write().expect("Failed to lock state");
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

    /// Handles mousemove event
    pub fn on_mousemove(&mut self, event: &web_sys::MouseEvent, main: &web_sys::Element) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let state = &mut conn_state.state;
            let (broadcast, new_state, new_net_state) = {
                let mut state = state.write().expect("Failed to lock state");
                if let NetGameState::Active(ref mut board_controller) = *state {
                    let state_dirty = board_controller.on_mousemove(event, self.player_id, &self.view.board_view, &get_context(main));
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
                } else {
                    (false, None, None)
                }
            };
            if let Some(ns) = new_net_state {
                let mut state = state.write().expect("Failed to lock state");
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

    /// Handles keydown event
    pub fn on_keydown(&mut self, event: &web_sys::KeyboardEvent, main: &web_sys::Element) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let state = &mut conn_state.state;
            let (broadcast, new_state, new_net_state) = {
                let mut state = state.write().expect("Failed to lock state");
                if let NetGameState::Active(ref mut board_controller) = *state {
                    let state_dirty = board_controller.on_keydown(event, self.player_id);
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
                } else {
                    (false, None, None)
                }
            };
            if let Some(ns) = new_net_state {
                let mut state = state.write().expect("Failed to lock state");
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

    /// Draw to the given element
    pub fn draw(&mut self, main: &web_sys::Element) {
        if main.child_element_count() == 0 {
            let document = main.owner_document().unwrap_throw();
            let canvas = document.create_element("canvas").unwrap_throw();
            let canvas = canvas.dyn_ref::<web_sys::HtmlCanvasElement>().unwrap_throw();
            canvas.set_width(1000);
            canvas.set_height(800);
            main.append_with_node_1(&canvas);
        }
        self.view.draw(self, &get_context(main));
    }

    fn broadcast_state(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("not broadcasting"));
            return;
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let state = state.read().expect("Failed to lock state");
            let message = Message::State(state.clone());
            sender.try_send(message.into()).map_err(|_| ()).expect("Failed to send message");
        }
    }

    /// Run Conrod GUI
    #[cfg(unix)]
    pub fn gui(&mut self, ui: &mut conrod_core::UiCell, ids: &Ids) {
        const MARGIN: conrod_core::Scalar = 30.0;
        const TITLE_SIZE: conrod_core::FontSize = 42;
        const SUBTITLE_SIZE: conrod_core::FontSize = 32;
        const BUTTON_DIMENSIONS: conrod_core::Dimensions = [200.0, 60.0];

        widget::Canvas::new().pad(MARGIN).set(ids.canvas, ui);

        let mut deferred_actions: Vec<Box<dyn Fn(&mut Self)>> = vec![];

        macro_rules! defer {
            (self.$e:ident($( $a:expr ),*)) => {
                #[allow(clippy::redundant_closure)]
                deferred_actions.push(Box::new(move |x: &mut Self| x.$e($($a),*)))
            }
        }

        match self.state {
            GameState::MainMenu => {
                widget::Text::new("DynaMaze")
                    .color(colors::DARK.into())
                    .font_size(TITLE_SIZE)
                    .mid_top_of(ids.canvas)
                    .set(ids.menu_header, ui);

                let tutorial_button = widget::Button::new()
                    .label("Tutorial")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .align_middle_x_of(ids.canvas)
                    .down_from(ids.menu_header, 3.0 * MARGIN)
                    .set(ids.tutorial_button, ui);
                for _ in tutorial_button {
                    self.tutorial();
                }

                let host_button = widget::Button::new()
                    .label("Host Game")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .align_middle_x_of(ids.canvas)
                    .down_from(ids.tutorial_button, MARGIN)
                    .set(ids.host_button, ui);
                for _host in host_button {
                    self.host();
                }

                let connect_button = widget::Button::new()
                    .label("Join Game")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .align_middle_x_of(ids.canvas)
                    .down_from(ids.host_button, MARGIN)
                    .set(ids.connect_button, ui);
                for _connect in connect_button {
                    self.connect();
                }

                let options_button = widget::Button::new()
                    .label("Options")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .align_middle_x_of(ids.canvas)
                    .down_from(ids.connect_button, MARGIN)
                    .set(ids.options_button, ui);
                for _options in options_button {
                    self.enter_options();
                }
            }
            GameState::ConnectMenu(ref mut connect_addr) => {
                widget::Text::new("Connect to Game")
                    .color(colors::DARK.into())
                    .font_size(SUBTITLE_SIZE)
                    .mid_top_of(ids.canvas)
                    .set(ids.menu_header, ui);

                let main_menu_button = widget::Button::new()
                    .label("Main Menu")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .top_left_of(ids.canvas)
                    .set(ids.main_menu_button, ui);
                for _press in main_menu_button {
                    defer!(self.main_menu());
                }

                let text = widget::TextBox::new(connect_addr)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .text_color(colors::PURPLE.into())
                    .align_middle_x_of(ids.canvas)
                    .align_middle_y_of(ids.canvas)
                    .set(ids.ip_box, ui);
                for evt in text {
                    match evt {
                        widget::text_box::Event::Update(new_text) => {
                            self.state = GameState::ConnectMenu(new_text);
                        }
                        widget::text_box::Event::Enter => {
                            self.do_connect();
                        }
                    }
                }

                let connect_button = widget::Button::new()
                    .label("Connect")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .align_middle_x_of(ids.canvas)
                    .down_from(ids.ip_box, MARGIN)
                    .set(ids.connect_button, ui);
                for _press in connect_button {
                    self.do_connect();
                }
            }
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                let is_host = state.is_host(self.player_id);
                match *state {
                    NetGameState::Lobby(ref info) => {
                        let status = if is_host {
                            let local_piece = match info.local_addr {
                                Ok(ref addr) => format!("Local: {}", addr),
                                Err(ref err) => format!("Local on port {} - error: {}", crate::net::LOCAL_PORT, err),
                            };
                            let remote_piece = match info.remote_addr {
                                Ok(ref addr) => format!("Remote: {}", addr),
                                Err(ref err) => format!("Auto port forwarding failed: {}", err),
                            };
                            format!("Hosting lobby\n{}\n{}", local_piece, remote_piece)
                        } else {
                            "Connected to lobby".into()
                        };
                        widget::Text::new(&status)
                            .color(colors::DARK.into())
                            .font_size(SUBTITLE_SIZE)
                            .mid_top_of(ids.canvas)
                            .set(ids.menu_header, ui);

                        let main_menu_button = widget::Button::new()
                            .label("Main Menu")
                            .wh(BUTTON_DIMENSIONS)
                            .color(conrod_core::color::WHITE.with_alpha(0.4))
                            .label_color(colors::DARK.into())
                            .top_left_of(ids.canvas)
                            .set(ids.main_menu_button, ui);
                        for _press in main_menu_button {
                            defer!(self.main_menu());
                        }

                        let me = info.player(&self.player_id);

                        let name_box = widget::TextBox::new(&me.name)
                            .color(conrod_core::color::WHITE.with_alpha(0.4))
                            .text_color(colors::PURPLE.into())
                            .w(BUTTON_DIMENSIONS[0])
                            .align_right_of(ids.canvas)
                            .down_from(ids.menu_header, MARGIN)
                            .set(ids.name_box, ui);
                        for evt in name_box {
                            match evt {
                                widget::text_box::Event::Update(new_text) => {
                                    let text = new_text.clone();
                                    defer!(self.set_own_name(&text));
                                }
                                widget::text_box::Event::Enter => {}
                            }
                        }

                        widget::Circle::fill(MARGIN / 2.0)
                            .color(me.color.into())
                            .align_middle_y_of(ids.name_box)
                            .left_from(ids.name_box, MARGIN)
                            .set(ids.color_demo, ui);

                        let color_button = widget::Button::new()
                            .label("Randomize Color")
                            .color(conrod_core::color::WHITE.with_alpha(0.4))
                            .label_color(colors::DARK.into())
                            .wh(BUTTON_DIMENSIONS)
                            .align_right_of(ids.name_box)
                            .down_from(ids.name_box, MARGIN)
                            .set(ids.color_button, ui);
                        for _press in color_button {
                            defer!(self.randomize_color());
                        }

                        let new_local_button = widget::Button::new()
                            .label("Add Local Player")
                            .color(conrod_core::color::WHITE.with_alpha(0.4))
                            .label_color(colors::DARK.into())
                            .wh(BUTTON_DIMENSIONS)
                            .align_right_of(ids.name_box)
                            .down_from(ids.color_button, MARGIN)
                            .set(ids.new_local_button, ui);
                        for _press in new_local_button {
                            defer!(self.new_local_player());
                        }

                        if is_host {
                            let start_button = widget::Button::new()
                                .label("Begin Game")
                                .color(conrod_core::color::WHITE.with_alpha(0.4))
                                .label_color(colors::DARK.into())
                                .wh(BUTTON_DIMENSIONS)
                                .mid_bottom_with_margin_on(ids.canvas, MARGIN)
                                .set(ids.start_button, ui);
                            for _press in start_button {
                                defer!(self.start_hosted_game());
                            }
                        }
                    }
                    NetGameState::Active(_) => {}
                    NetGameState::GameOver(ref info) => {
                        let text = format!("{} wins!", info.winner.name);
                        widget::Text::new(&text)
                            .color(colors::DARK.into())
                            .font_size(SUBTITLE_SIZE)
                            .mid_top_of(ids.canvas)
                            .set(ids.menu_header, ui);

                        // TODO just throw this in at the end for everything
                        let main_menu_button = widget::Button::new()
                            .label("Main Menu")
                            .wh(BUTTON_DIMENSIONS)
                            .color(conrod_core::color::WHITE.with_alpha(0.4))
                            .label_color(colors::DARK.into())
                            .top_left_of(ids.canvas)
                            .set(ids.main_menu_button, ui);
                        for _press in main_menu_button {
                            defer!(self.main_menu());
                        }
                    }
                    NetGameState::Error(ref text) => {
                        widget::Text::new("Error")
                            .color(colors::DARK.into())
                            .font_size(SUBTITLE_SIZE)
                            .mid_top_of(ids.canvas)
                            .set(ids.menu_header, ui);

                        widget::Text::new(text)
                            .color(colors::DARK.into())
                            .align_middle_x_of(ids.menu_header)
                            .down_from(ids.menu_header, MARGIN)
                            .set(ids.error_text, ui);

                        let main_menu_button = widget::Button::new()
                            .label("Main Menu")
                            .wh(BUTTON_DIMENSIONS)
                            .color(conrod_core::color::WHITE.with_alpha(0.4))
                            .label_color(colors::DARK.into())
                            .top_left_of(ids.canvas)
                            .set(ids.main_menu_button, ui);
                        for _press in main_menu_button {
                            defer!(self.main_menu());
                        }
                    }
                }
            }
            GameState::HardError(ref text) => {
                widget::Text::new("Error")
                    .color(colors::DARK.into())
                    .font_size(SUBTITLE_SIZE)
                    .mid_top_of(ids.canvas)
                    .set(ids.menu_header, ui);

                widget::Text::new(text)
                    .color(colors::DARK.into())
                    .align_middle_x_of(ids.menu_header)
                    .down_from(ids.menu_header, MARGIN)
                    .set(ids.error_text, ui);

                let main_menu_button = widget::Button::new()
                    .label("Main Menu")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .top_left_of(ids.canvas)
                    .set(ids.main_menu_button, ui);
                for _press in main_menu_button {
                    defer!(self.main_menu());
                }
            }
            GameState::Options(ref mut curr_options) => {
                widget::Text::new("Options")
                    .color(colors::DARK.into())
                    .font_size(TITLE_SIZE)
                    .mid_top_of(ids.canvas)
                    .set(ids.menu_header, ui);

                if let Some(new_music) = widget::Slider::new(f32::from(curr_options.music_level), 0.0, 100.0)
                    .label("Music Level")
                    .down_from(ids.menu_header, MARGIN)
                    .padded_w_of(ids.menu_header, -MARGIN)
                    .align_middle_x_of(ids.menu_header)
                    .set(ids.music_slider, ui) {
                    curr_options.music_level = new_music as u8;
                    sound::SOUND.poke_options(curr_options);
                }

                if let Some(new_sound) = widget::Slider::new(f32::from(curr_options.sound_level), 0.0, 100.0)
                    .label("Sound Level")
                    .down_from(ids.music_slider, MARGIN)
                    .w_of(ids.music_slider)
                    .align_middle_x_of(ids.music_slider)
                    .set(ids.sound_slider, ui) {
                    curr_options.sound_level = new_sound as u8;
                    sound::SOUND.poke_options(curr_options);
                }

                let save_button = widget::Button::new()
                    .label("Save")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .mid_bottom_of(ids.canvas)
                    .set(ids.save_button, ui);
                for _press in save_button {
                    defer!(self.save_options());
                }

                let main_menu_button = widget::Button::new()
                    .label("Main Menu")
                    .wh(BUTTON_DIMENSIONS)
                    .color(conrod_core::color::WHITE.with_alpha(0.4))
                    .label_color(colors::DARK.into())
                    .top_left_of(ids.canvas)
                    .set(ids.main_menu_button, ui);
                for _press in main_menu_button {
                    defer!(self.main_menu());
                }
            }
        }

        for action in deferred_actions {
            action(self);
        }
    }
}

impl Default for GameController {
    fn default() -> Self {
        Self::new()
    }
}
