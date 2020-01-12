//! Menu / global state controller

use std::sync::{Arc, Mutex, RwLock};

use gloo::events::{EventListener, EventListenerOptions};
use rand::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d as Context;

use crate::{BoardController, BoardSettings, GameView, Player, PlayerID};
use crate::anim;
use crate::colors::Color;
use crate::demo;
use crate::menu::{ConnectedState, GameOverInfo, GameState, LobbyInfo, NetGameState};
use crate::net::{self, Message};
use crate::options;
use crate::sound::{self, SoundEngine};
use crate::tutorial;

fn get_context(main: &web_sys::Element) -> Option<Context> {
    let canvas = main.query_selector("canvas").unwrap_throw()?;
    let canvas = canvas
        .dyn_ref::<web_sys::HtmlCanvasElement>()
        .unwrap_throw();
    let ctx = canvas.get_context("2d").unwrap_throw().unwrap_throw();
    let ctx = ctx.dyn_ref::<Context>().unwrap_throw();
    Some(ctx.clone())
}

type DeferredAction = Box<dyn FnOnce(&mut GameController)>;

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
    pub actions: Arc<Mutex<Vec<DeferredAction>>>,
    /// DOM event listeners
    pub listeners: Vec<EventListener>,
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
            listeners: vec![],
        }
    }

    fn tutorial(&mut self) {
        self.state = GameState::InGame(tutorial::new_conn_state(self.player_id));
    }

    fn host(&mut self) {
        let game = random();
        let state = NetGameState::Lobby(LobbyInfo::new(self.player_id, game));
        let state = Arc::new(RwLock::new(state));
        let sender = net::NetHandler::run(state.clone(), game, self.player_id);
        anim::STATE.write().unwrap().set_send(sender.queue());
        let conn_state = ConnectedState { state, sender };
        self.state = GameState::InGame(conn_state);
    }

    fn connect(&mut self) {
        self.state = GameState::ConnectMenu;
    }

    fn enter_options(&mut self) {
        self.state = GameState::Options(options::HANDLE.fetch().clone());
    }

    fn do_connect(&mut self, form: web_sys::HtmlFormElement) {
        if let GameState::ConnectMenu = self.state {
            let elements = form.elements();
            let game = elements.item(0).unwrap_throw();
            let game = game.dyn_ref::<web_sys::HtmlInputElement>().unwrap_throw();
            let game = game.value().parse().unwrap_throw();
            let state = NetGameState::Error("Connecting...".to_string());
            let state = Arc::new(RwLock::new(state));
            let mut sender = net::NetHandler::run(state.clone(), game, self.player_id);
            anim::STATE.write().unwrap().set_send(sender.queue());
            let player = Player::new("Guesty McGuestface".into(), random(), self.player_id);
            NetGameState::join_lobby(&mut sender, player);
            let conn_state = ConnectedState { sender, state };
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
                    sender.send(message);
                }
            }
        }
    }

    fn set_name(&mut self, name_field: web_sys::HtmlInputElement, id: PlayerID) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                let player = info.player_mut(&id);
                let new_name = name_field.value();
                player.name = new_name;
                let message = Message::EditPlayer(id, player.clone());
                sender.send(message);
            }
        }
    }

    fn set_color(&mut self, color_field: web_sys::HtmlInputElement, id: PlayerID) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                let player = info.player_mut(&id);
                let color = color_field.value();
                let color_r = u8::from_str_radix(&color[1..3], 16).unwrap_throw();
                let color_g = u8::from_str_radix(&color[3..5], 16).unwrap_throw();
                let color_b = u8::from_str_radix(&color[5..7], 16).unwrap_throw();
                let color_r = color_r as f32 / 255.0;
                let color_g = color_g as f32 / 255.0;
                let color_b = color_b as f32 / 255.0;
                let color = Color(color_r, color_g, color_b);
                player.color = color;
                let message = Message::EditPlayer(id, player.clone());
                sender.send(message);
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
                let child = Player::new_child(format!("{} - Copy", me.name), random(), random(), me.id);
                info.guests.push(child.clone());
                if is_host {
                    drop(state);
                    self.broadcast_state();
                } else {
                    sender.send(Message::JoinLobby(child));
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
        self.sound_engine.fetch_volume();
        self.state = GameState::MainMenu;
    }

    /// Handles tick
    pub fn on_tick(&mut self, dt: f64) {
        anim::STATE.write().unwrap().advance_by(dt);

        let old_last_player = self.last_player;

        let music = match self.state {
            GameState::MainMenu
            | GameState::ConnectMenu
            | GameState::HardError(_)
            | GameState::Options(_) => {
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

        if let GameState::InGame(ref state) = self.state {
            state.sender.drain_queue();
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
                    let state_dirty = board_controller.on_click(
                        event,
                        self.player_id,
                        &self.view.board_view,
                        &get_context(main).unwrap_throw(),
                    );
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
                    let state_dirty = board_controller.on_mousemove(
                        event,
                        self.player_id,
                        &self.view.board_view,
                        &get_context(main).unwrap_throw(),
                    );
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
    pub fn on_keydown(&mut self, event: &web_sys::KeyboardEvent, _main: &web_sys::Element) {
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
        if let GameState::InGame(ref mut _conn_state) = self.state {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("broadcasting"));
            let sender = &mut _conn_state.sender;
            let state = &mut _conn_state.state;
            let state = state.read().expect("Failed to lock state");
            let message = Message::State(state.clone());
            sender.send(message);
        }
    }

    fn curr_class(&self) -> &'static str {
        match self.state {
            GameState::MainMenu => "main-menu",
            GameState::ConnectMenu => "connect-menu",
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                match *state {
                    NetGameState::Lobby(_) => "lobby",
                    NetGameState::Active(_) => "active",
                    NetGameState::GameOver(_) => "game-over",
                    NetGameState::Error(_) => "error",
                }
            }
            GameState::HardError(_) => "hard-error",
            GameState::Options(_) => "options",
        }
    }

    fn build_dom(&mut self, main: &web_sys::Element) {
        let old_class = main.class_name();
        let curr_class = self.curr_class();

        // deferring is complicated, preventing default is complicated
        macro_rules! listen {
            ($target:expr, $evt:expr, self.$e:ident($( $a:ident ),*)) => {{
                let target = $target;
                $(let $a = $a.clone();)*
                let options = EventListenerOptions::enable_prevent_default();
                let actions = self.actions.clone();
                let listener = EventListener::new_with_options(
                    target,
                    $evt,
                    options,
                    move |event| {
                        $(let $a = $a.clone();)*
                        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("handling an event"));
                        event.prevent_default();
                        let mut actions = actions.lock().unwrap_throw();
                        actions.push(Box::new(move |x: &mut Self| x.$e($($a),*)));
                    }
                );
                self.listeners.push(listener);
            }}
        }

        // get ready to make some elements
        let document = main.owner_document().unwrap_throw();

        // if the UI doesn't need to be rebuilt from scratch...
        if old_class == curr_class {
            // apply updates incrementally
            if let GameState::InGame(ref conn_state) = self.state {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                match *state {
                    NetGameState::Lobby(ref info) => {
                        let players = main.query_selector("ul").unwrap_throw().unwrap_throw();
                        for player_info in info.players_ref() {
                            let is_local = player_info.lives_with(self.player_id);
                            let existing_player = players.query_selector(&format!("#player-{}", player_info.id))
                                .map_err(|e| web_sys::console::error_1(&e)).ok().flatten();
                            match existing_player {
                                Some(player) => {
                                    if !is_local {
                                        let name = player.query_selector("span:first-child").unwrap_throw().unwrap_throw();
                                        let name = name.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                                        if name.inner_text() != player_info.name {
                                            name.set_inner_text(&player_info.name);
                                        }
                                        let color = player.query_selector("span:last-child").unwrap_throw().unwrap_throw();
                                        let color = color.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                                        if color.style().get_property_value("background-color").unwrap_throw() != player_info.color.hex() {
                                            color.style().set_property("background-color", &player_info.color.hex()).unwrap_throw();
                                        }
                                    }
                                }
                                None => {
                                    let player = document.create_element("li").unwrap_throw();
                                    player.set_id(&format!("player-{}", player_info.id));
                                    if is_local {
                                        let name_box = document.create_element("input").unwrap_throw();
                                        let name_box = name_box.dyn_ref::<web_sys::HtmlInputElement>().unwrap_throw();
                                        name_box.set_attribute("value", &player_info.name).unwrap_throw();
                                        player.append_with_node_1(&name_box).unwrap_throw();
                                        let id = player_info.id;
                                        listen!(&name_box, "input", self.set_name(name_box, id));
                                        player.append_with_node_1(&name_box).unwrap_throw();
                                        let color = document.create_element("input").unwrap_throw();
                                        let color = color.dyn_ref::<web_sys::HtmlInputElement>().unwrap_throw();
                                        color.set_type("color");
                                        color.set_value(&player_info.color.hex());
                                        listen!(&color, "input", self.set_color(color, id));
                                        player.append_with_node_1(&color).unwrap_throw();
                                    } else {
                                        let name = document.create_element("span").unwrap_throw();
                                        let name = name.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                                        name.set_inner_text(&player_info.name);
                                        player.append_with_node_1(&name).unwrap_throw();
                                        let color = document.create_element("span").unwrap_throw();
                                        let color = color.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                                        color.set_inner_html("&nbsp;");
                                        color.style().set_property("background-color", &player_info.color.hex()).unwrap_throw();
                                        player.append_with_node_1(&color).unwrap_throw();
                                    }
                                    players.append_with_node_1(&player).unwrap_throw();
                                }
                            }
                        }
                    }
                    NetGameState::Active(_) => {
                        let canvas = main.query_selector("canvas").unwrap_throw().unwrap_throw();
                        let canvas = canvas
                            .dyn_ref::<web_sys::HtmlCanvasElement>()
                            .unwrap_throw();
                        let window = web_sys::window().unwrap_throw();
                        let inner_width = window.inner_width().unwrap_throw().as_f64().unwrap_throw() as u32;
                        let inner_height = window.inner_height().unwrap_throw().as_f64().unwrap_throw() as u32;
                        canvas.set_width(inner_width);
                        canvas.set_height(inner_height);
                    }
                    _ => {}
                }
            }
            return;
        }
        // if there's a wrong UI already...
        if old_class != "" {
            // nuke everything from orbit
            main.set_inner_html("");
            self.listeners = vec![];
        }
        // give it the right class
        main.set_class_name(curr_class);

        match self.state {
            GameState::MainMenu => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("DynaMaze");
                main.append_with_node_1(&header).unwrap_throw();

                let tutorial = document.create_element("button").unwrap_throw();
                let tutorial = tutorial.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                tutorial.set_inner_text("Tutorial");
                main.append_with_node_1(&tutorial).unwrap_throw();
                listen!(&tutorial, "click", self.tutorial());

                let host = document.create_element("button").unwrap_throw();
                let host = host.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                host.set_inner_text("Host Game");
                main.append_with_node_1(&host).unwrap_throw();
                listen!(&host, "click", self.host());

                let connect = document.create_element("button").unwrap_throw();
                let connect = connect.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                connect.set_inner_text("Join Game");
                main.append_with_node_1(&connect).unwrap_throw();
                listen!(&connect, "click", self.connect());

                let options = document.create_element("button").unwrap_throw();
                let options = options.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                options.set_inner_text("Options");
                main.append_with_node_1(&options).unwrap_throw();
                listen!(&options, "click", self.enter_options());
            }
            GameState::ConnectMenu => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("Connect to Game");
                main.append_with_node_1(&header).unwrap_throw();

                let main_menu = document.create_element("button").unwrap_throw();
                let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                main_menu.set_inner_text("Main Menu");
                main.append_with_node_1(&main_menu).unwrap_throw();
                listen!(&main_menu, "click", self.main_menu());

                let connect_form = document.create_element("form").unwrap_throw();
                let connect_form = connect_form
                    .dyn_ref::<web_sys::HtmlFormElement>()
                    .unwrap_throw();
                main.append_with_node_1(&connect_form).unwrap_throw();

                let connect_text = document.create_element("input").unwrap_throw();
                let connect_text = connect_text
                    .dyn_ref::<web_sys::HtmlElement>()
                    .unwrap_throw();
                connect_form
                    .append_with_node_1(&connect_text)
                    .unwrap_throw();

                let connect = document.create_element("button").unwrap_throw();
                let connect = connect.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                connect.set_inner_text("Connect");
                connect_form.append_with_node_1(&connect).unwrap_throw();

                listen!(&connect_form, "submit", self.do_connect(connect_form));
            }
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                let is_host = state.is_host(self.player_id);
                match *state {
                    NetGameState::Lobby(ref info) => {
                        let status = if is_host {
                            let game_id = info.id;
                            format!("Hosting lobby\n{}", game_id)
                        } else {
                            "Connected to lobby".into()
                        };
                        let header = document.create_element("h1").unwrap_throw();
                        let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        header.set_inner_text(&status);
                        main.append_with_node_1(&header).unwrap_throw();

                        let main_menu = document.create_element("button").unwrap_throw();
                        let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        main_menu.set_inner_text("Main Menu");
                        main.append_with_node_1(&main_menu).unwrap_throw();
                        listen!(&main_menu, "click", self.main_menu());

                        let players = document.create_element("ul").unwrap_throw();
                        main.append_with_node_1(&players).unwrap_throw();

                        for player_info in info.players_ref() {
                            let player = document.create_element("li").unwrap_throw();
                            player.set_id(&format!("player-{}", player_info.id));
                            let is_local = player_info.lives_with(self.player_id);
                            if is_local {
                                let name_box = document.create_element("input").unwrap_throw();
                                let name_box = name_box.dyn_ref::<web_sys::HtmlInputElement>().unwrap_throw();
                                name_box.set_value(&player_info.name);
                                player.append_with_node_1(&name_box).unwrap_throw();
                                let id = player_info.id;
                                listen!(&name_box, "input", self.set_name(name_box, id));
                                player.append_with_node_1(&name_box).unwrap_throw();
                                let color = document.create_element("input").unwrap_throw();
                                let color = color.dyn_ref::<web_sys::HtmlInputElement>().unwrap_throw();
                                color.set_type("color");
                                color.set_value(&player_info.color.hex());
                                listen!(&color, "input", self.set_color(color, id));
                                player.append_with_node_1(&color).unwrap_throw();
                            } else {
                                let name = document.create_element("span").unwrap_throw();
                                let name = name.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                                name.set_inner_text(&player_info.name);
                                player.append_with_node_1(&name).unwrap_throw();
                                let color = document.create_element("span").unwrap_throw();
                                let color = color.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                                color.set_inner_html("&nbsp;");
                                color.style().set_property("background-color", &player_info.color.hex()).unwrap_throw();
                                player.append_with_node_1(&color).unwrap_throw();
                            }
                            players.append_with_node_1(&player).unwrap_throw();
                        }

                        let new_local = document.create_element("button").unwrap_throw();
                        let new_local = new_local.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        new_local.set_inner_text("New Local Player");
                        main.append_with_node_1(&new_local).unwrap_throw();
                        listen!(&new_local, "click", self.new_local_player());

                        if is_host {
                            let start = document.create_element("button").unwrap_throw();
                            let start = start.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                            start.set_inner_text("Begin Game");
                            main.append_with_node_1(&start).unwrap_throw();
                            listen!(&start, "click", self.start_hosted_game());
                        }
                    }
                    NetGameState::Active(_) => {
                        let canvas = document.create_element("canvas").unwrap_throw();
                        let canvas = canvas
                            .dyn_ref::<web_sys::HtmlCanvasElement>()
                            .unwrap_throw();
                        main.append_with_node_1(&canvas).unwrap_throw();
                    }
                    NetGameState::GameOver(ref info) => {
                        let text = format!("{} wins!", info.winner.name);
                        let header = document.create_element("h1").unwrap_throw();
                        let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        header.set_inner_text(&text);
                        main.append_with_node_1(&header).unwrap_throw();

                        let main_menu = document.create_element("button").unwrap_throw();
                        let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        main_menu.set_inner_text("Main Menu");
                        main.append_with_node_1(&main_menu).unwrap_throw();
                        listen!(&main_menu, "click", self.main_menu());
                    }
                    NetGameState::Error(ref text) => {
                        let header = document.create_element("h1").unwrap_throw();
                        let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        header.set_inner_text("Error");
                        main.append_with_node_1(&header).unwrap_throw();

                        let body = document.create_element("p").unwrap_throw();
                        let body = body.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        body.set_inner_text(text);
                        main.append_with_node_1(&body).unwrap_throw();

                        let main_menu = document.create_element("button").unwrap_throw();
                        let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                        main_menu.set_inner_text("Main Menu");
                        main.append_with_node_1(&main_menu).unwrap_throw();
                        listen!(&main_menu, "click", self.main_menu());
                    }
                }
            }
            GameState::HardError(ref text) => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("Error");
                main.append_with_node_1(&header).unwrap_throw();

                let body = document.create_element("p").unwrap_throw();
                let body = body.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                body.set_inner_text(text);
                main.append_with_node_1(&body).unwrap_throw();

                let main_menu = document.create_element("button").unwrap_throw();
                let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                main_menu.set_inner_text("Main Menu");
                main.append_with_node_1(&main_menu).unwrap_throw();
                listen!(&main_menu, "click", self.main_menu());
            }
            GameState::Options(ref mut _curr_options) => {
                let header = document.create_element("h1").unwrap_throw();
                let header = header.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                header.set_inner_text("Options");
                main.append_with_node_1(&header).unwrap_throw();

                // TODO slider for music level with poke_options

                // TODO slider for sound level with poke_options

                let save_button = document.create_element("button").unwrap_throw();
                let save_button = save_button.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                save_button.set_inner_text("Save");
                main.append_with_node_1(&save_button).unwrap_throw();
                listen!(&save_button, "click", self.save_options());

                let main_menu = document.create_element("button").unwrap_throw();
                let main_menu = main_menu.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
                main_menu.set_inner_text("Main Menu");
                main.append_with_node_1(&main_menu).unwrap_throw();
                listen!(&main_menu, "click", self.main_menu());
            }
        }
    }
}

impl Default for GameController {
    fn default() -> Self {
        Self::new()
    }
}
