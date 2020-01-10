//! Menu / global state controller

use std::sync::{Arc, Mutex, RwLock};

use gloo::events::EventListener;
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

fn get_context(main: &web_sys::Element) -> Option<Context> {
    let canvas = main.query_selector("canvas").unwrap_throw()?;
    let canvas = canvas.dyn_ref::<web_sys::HtmlCanvasElement>().unwrap_throw();
    let ctx = canvas.get_context("2d").unwrap_throw().unwrap_throw();
    let ctx = ctx.dyn_ref::<Context>().unwrap_throw();
    Some(ctx.clone())
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
    /// Action queue
    pub actions: Arc<Mutex<Vec<Box<dyn FnOnce(&mut GameController)>>>>,
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
            actions: Default::default(),
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

        // drain one action at a time
        let action = {
            let mut actions = self.actions.lock().unwrap();
            actions.pop()
        };
        if let Some(action) = action {
            action(self);
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
                    let state_dirty = board_controller.on_click(event, self.player_id, &self.view.board_view, &get_context(main).unwrap_throw());
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
                    let state_dirty = board_controller.on_mousemove(event, self.player_id, &self.view.board_view, &get_context(main).unwrap_throw());
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
        self.build_dom(main);
        if let Some(ctx) = get_context(main) {
            self.view.draw(self, &ctx);
        }
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

    fn curr_class(&self) -> &'static str {
        match self.state {
            GameState::MainMenu => {
                "main-menu"
            }
            GameState::ConnectMenu(_) => {
                "connect-menu"
            }
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                match *state {
                    NetGameState::Lobby(ref info) => {
                        "lobby"
                    }
                    NetGameState::Active(_) => {
                        "active"
                    }
                    NetGameState::GameOver(ref info) => {
                        "game-over"
                    }
                    NetGameState::Error(ref text) => {
                        "error"
                    }
                }
            }
            GameState::HardError(_) => {
                "hard-error"
            }
            GameState::Options(_) => {
                "options"
            }
        }
    }

    fn build_dom(&mut self, main: &web_sys::Element) {
        let old_class = main.class_name();
        let curr_class = self.curr_class();

        // deferring actions is now more complicated
        macro_rules! defer {
            (self.$e:ident($( $a:expr ),*)) => {{
                let actions = self.actions.clone();
                move |_| {
                    let mut actions = actions.lock().unwrap_throw();
                    actions.push(Box::new(move |x: &mut Self| x.$e($($a),*)))
                }
            }}
        }

        // if the UI doesn't need to be rebuilt from scratch, don't do anything
        if old_class == curr_class {
            return;
        }
        // if there's a wrong UI already...
        if old_class != "" {
            // nuke everything from orbit
            main.set_inner_html("");
        }
        // give it the right class
        main.set_class_name(curr_class);

        // get ready to make some elements
        let document = main.owner_document().unwrap_throw();

        // TODO don't leak all the event listeners

        match self.state {
            GameState::MainMenu => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("DynaMaze");
                main.append_with_node_1(&header);

                let tutorial = document.create_element("button").unwrap_throw();
                let tutorial = tutorial.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                tutorial.set_inner_text("Tutorial");
                main.append_with_node_1(&tutorial);
                EventListener::once(&tutorial, "click", defer!(self.tutorial())).forget();

                let host = document.create_element("button").unwrap_throw();
                let host = host.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                host.set_inner_text("Host Game");
                main.append_with_node_1(&host);
                EventListener::once(&host, "click", defer!(self.host())).forget();

                let connect = document.create_element("button").unwrap_throw();
                let connect = connect.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                connect.set_inner_text("Join Game");
                main.append_with_node_1(&connect);
                EventListener::once(&connect, "click", defer!(self.connect())).forget();

                let options = document.create_element("button").unwrap_throw();
                let options = options.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                options.set_inner_text("Options");
                main.append_with_node_1(&options);
                EventListener::once(&options, "click", defer!(self.enter_options())).forget();
            }
            GameState::ConnectMenu(ref mut connect_addr) => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("Connect to Game");
                main.append_with_node_1(&header);

                let main_menu = document.create_element("button").unwrap_throw();
                let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                main_menu.set_inner_text("Main Menu");
                main.append_with_node_1(&main_menu);
                EventListener::once(&main_menu, "click", defer!(self.main_menu())).forget();

                let connect_text = document.create_element("input").unwrap_throw();
                let connect_text = connect_text.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                main.append_with_node_1(&connect_text);
                // TODO handle Enter, probably with a form

                let connect = document.create_element("button").unwrap_throw();
                let connect = connect.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                connect.set_inner_text("Connect");
                main.append_with_node_1(&connect);
                EventListener::once(&connect, "click", defer!(self.do_connect())).forget();
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
                        let header = document.create_element("h1").unwrap_throw();
                        let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        header.set_inner_text(&status);
                        main.append_with_node_1(&header);

                        let main_menu = document.create_element("button").unwrap_throw();
                        let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        main_menu.set_inner_text("Main Menu");
                        main.append_with_node_1(&main_menu);
                        EventListener::once(&main_menu, "click", defer!(self.main_menu())).forget();

                        let me = info.player(&self.player_id);

                        let name_box = document.create_element("input").unwrap_throw();
                        let name_box = name_box.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        name_box.set_attribute("value", &me.name);
                        main.append_with_node_1(&name_box);
                        // TODO catch changes with self.set_own_name()

                        // TODO circle with color me.color

                        // TODO randomize color with self.randomize_color()

                        // TODO add local player with self.new_local_player()

                        if is_host {
                            let start = document.create_element("button").unwrap_throw();
                            let start = start.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                            start.set_inner_text("Begin Game");
                            main.append_with_node_1(&start);
                            EventListener::once(&start, "click", defer!(self.start_hosted_game())).forget();
                        }
                    }
                    NetGameState::Active(_) => {
                        let canvas = document.create_element("canvas").unwrap_throw();
                        let canvas = canvas.dyn_ref::<web_sys::HtmlCanvasElement>().unwrap_throw();
                        canvas.set_width(1000);
                        canvas.set_height(800);
                        main.append_with_node_1(&canvas);
                    }
                    NetGameState::GameOver(ref info) => {
                        let text = format!("{} wins!", info.winner.name);
                        let header = document.create_element("h1").unwrap_throw();
                        let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        header.set_inner_text(&text);
                        main.append_with_node_1(&header);

                        let main_menu = document.create_element("button").unwrap_throw();
                        let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        main_menu.set_inner_text("Main Menu");
                        main.append_with_node_1(&main_menu);
                        EventListener::once(&main_menu, "click", defer!(self.main_menu())).forget();
                    }
                    NetGameState::Error(ref text) => {
                        let header = document.create_element("h1").unwrap_throw();
                        let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        header.set_inner_text("Error");
                        main.append_with_node_1(&header);

                        let body = document.create_element("p").unwrap_throw();
                        let body = body.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        body.set_inner_text(text);
                        main.append_with_node_1(&body);

                        let main_menu = document.create_element("button").unwrap_throw();
                        let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        main_menu.set_inner_text("Main Menu");
                        main.append_with_node_1(&main_menu);
                        EventListener::once(&main_menu, "click", defer!(self.main_menu())).forget();
                    }
                }
            }
            GameState::HardError(ref text) => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("Error");
                main.append_with_node_1(&header);

                let body = document.create_element("p").unwrap_throw();
                let body = body.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                body.set_inner_text(text);
                main.append_with_node_1(&body);

                let main_menu = document.create_element("button").unwrap_throw();
                let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                main_menu.set_inner_text("Main Menu");
                main.append_with_node_1(&main_menu);
                EventListener::once(&main_menu, "click", defer!(self.main_menu())).forget();
            }
            GameState::Options(ref mut curr_options) => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("Options");
                main.append_with_node_1(&header);

                // TODO slider for music level with poke_options

                // TODO slider for sound level with poke_options

                let save_button = document.create_element("button").unwrap_throw();
                let save_button = save_button.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                save_button.set_inner_text("Save");
                main.append_with_node_1(&save_button);
                EventListener::once(&save_button, "click", defer!(self.save_options())).forget();

                let main_menu = document.create_element("button").unwrap_throw();
                let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                main_menu.set_inner_text("Main Menu");
                main.append_with_node_1(&main_menu);
                EventListener::once(&main_menu, "click", defer!(self.main_menu())).forget();
            }
        }
    }
}

impl Default for GameController {
    fn default() -> Self {
        Self::new()
    }
}
