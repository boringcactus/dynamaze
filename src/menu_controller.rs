//! Menu / global state controller

use std::sync::{Arc, Mutex, RwLock};

use gloo::events::{EventListener, EventListenerOptions};
use rand::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d as Context;

use crate::{BoardController, GameView, Player, PlayerID};
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
            let state = NetGameState::Connecting;
            let state = Arc::new(RwLock::new(state));
            let mut sender = net::NetHandler::run(state.clone(), game, self.player_id);
            anim::STATE.write().unwrap().set_send(sender.queue());
            let player = Player::new("Guesty McGuestface".into(), random(), self.player_id);
            NetGameState::join_lobby(&mut sender, player);
            let conn_state = ConnectedState { sender, state };
            self.state = GameState::InGame(conn_state);
        }
    }

    fn set_width(&mut self, width: web_sys::HtmlInputElement) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                let settings = &mut info.settings;
                settings.width = width.value().parse().unwrap_throw();
                settings.version += 1;
                width.form().unwrap_throw().dataset().set("version", &format!("{}", settings.version)).unwrap_throw();
                let message = Message::EditSettings(settings.clone());
                sender.send(message);
            }
        }
    }

    fn set_height(&mut self, height: web_sys::HtmlInputElement) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                let settings = &mut info.settings;
                settings.height = height.value().parse().unwrap_throw();
                settings.version += 1;
                height.form().unwrap_throw().dataset().set("version", &format!("{}", settings.version)).unwrap_throw();
                let message = Message::EditSettings(settings.clone());
                sender.send(message);
            }
        }
    }

    fn set_score_limit(&mut self, score_limit: web_sys::HtmlInputElement) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                let settings = &mut info.settings;
                settings.score_limit = score_limit.value().parse().unwrap_throw();
                settings.version += 1;
                score_limit.form().unwrap_throw().dataset().set("version", &format!("{}", settings.version)).unwrap_throw();
                let message = Message::EditSettings(settings.clone());
                sender.send(message);
            }
        }
    }

    fn set_music_level(&mut self, slider: web_sys::HtmlInputElement) {
        if let GameState::Options(ref mut opts) = self.state {
            let val = slider.value();
            opts.music_level = val.parse().unwrap_throw();
            self.sound_engine.poke_options(opts);
        }
    }

    fn set_sound_level(&mut self, slider: web_sys::HtmlInputElement) {
        if let GameState::Options(ref mut opts) = self.state {
            let val = slider.value();
            opts.sound_level = val.parse().unwrap_throw();
            self.sound_engine.poke_options(opts);
        }
    }

    fn save_options(&mut self) {
        if let GameState::Options(ref opts) = self.state {
            options::HANDLE.save(opts);
            self.state = GameState::MainMenu;
            self.sound_engine.fetch_volume();
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
                    let settings = info.settings.clone();
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
        self.sound_engine.unpause();
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
        if let GameState::InGame(ref mut conn_state) = self.state {
            let sender = &mut conn_state.sender;
            let state = &mut conn_state.state;
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
                    NetGameState::Connecting => "connecting",
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
        fn query_selector<T: JsCast + Clone>(element: &web_sys::Element, selector: &str) -> T {
            let result = element.query_selector(selector).unwrap_throw().unwrap_throw();
            let result = result.dyn_ref::<T>().unwrap_throw();
            result.clone()
        }
        fn create_element<T: JsCast + Clone>(document: &web_sys::Document, tag: &str) -> T {
            let result = document.create_element(tag).unwrap_throw();
            let result = result.dyn_ref::<T>().unwrap_throw();
            result.clone()
        }
        fn create_element_with_text<T: JsCast + Clone>(document: &web_sys::Document, tag: &str, text: &str) -> T {
            let result = document.create_element(tag).unwrap_throw();
            let result = result.dyn_ref::<web_sys::HtmlElement>().unwrap_throw();
            result.set_inner_text(text);
            let result = result.dyn_ref::<T>().unwrap_throw();
            result.clone()
        }
        fn named_item<T: JsCast + Clone>(collection: &web_sys::HtmlCollection, name: &str) -> T {
            let result = collection.named_item(name).unwrap_throw();
            let result = result.dyn_ref::<T>().unwrap_throw();
            result.clone()
        }

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

        // this can't be a closure or a regular function because of ownership weirdness
        macro_rules! create_player {
            ($player_info:expr, $is_local:expr) => {{
                let player_info = $player_info;
                let is_local = $is_local;
                let player: web_sys::HtmlElement = create_element(&document, "li");
                player.set_id(&format!("player-{}", player_info.id));
                if is_local {
                    let name_box: web_sys::HtmlInputElement = create_element(&document, "input");
                    name_box.set_value(&player_info.name);
                    let id = player_info.id;
                    listen!(&name_box, "input", self.set_name(name_box, id));
                    player.append_with_node_1(&name_box).unwrap_throw();
                    let color: web_sys::HtmlInputElement = create_element(&document, "input");
                    color.set_type("color");
                    color.set_value(&player_info.color.hex());
                    listen!(&color, "input", self.set_color(color, id));
                    player.append_with_node_1(&color).unwrap_throw();
                } else {
                    let name: web_sys::HtmlElement = create_element_with_text(&document, "span", &player_info.name);
                    player.append_with_node_1(&name).unwrap_throw();
                    let color: web_sys::HtmlElement = create_element(&document, "span");
                    color.set_inner_html("&nbsp;");
                    color.style().set_property("background-color", &player_info.color.hex()).unwrap_throw();
                    player.append_with_node_1(&color).unwrap_throw();
                }
                player
            }};
        }

        // if the UI doesn't need to be rebuilt from scratch...
        if old_class == curr_class {
            // apply updates incrementally
            if let GameState::InGame(ref conn_state) = self.state {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                match *state {
                    NetGameState::Lobby(ref info) => {
                        // update players
                        let players: web_sys::HtmlElement = query_selector(main, "ul");
                        for player_info in info.players_ref() {
                            let is_local = player_info.lives_with(self.player_id);
                            let existing_player = players.query_selector(&format!("#player-{}", player_info.id))
                                .map_err(|e| web_sys::console::error_1(&e)).ok().flatten();
                            match existing_player {
                                Some(player) => {
                                    if !is_local {
                                        let name: web_sys::HtmlElement = query_selector(&player, "span:first-child");
                                        if name.inner_text() != player_info.name {
                                            name.set_inner_text(&player_info.name);
                                        }
                                        let color: web_sys::HtmlElement = query_selector(&player, "span:last-child");
                                        if color.style().get_property_value("background-color").unwrap_throw() != player_info.color.hex() {
                                            color.style().set_property("background-color", &player_info.color.hex()).unwrap_throw();
                                        }
                                    }
                                }
                                None => {
                                    let player = create_player!(player_info, is_local);
                                    players.append_with_node_1(&player).unwrap_throw();
                                }
                            }
                        }

                        // update settings
                        let settings_form: web_sys::HtmlFormElement = query_selector(main, "form");
                        let current_version: usize = settings_form.dataset().get("version").unwrap_throw().parse().unwrap_throw();
                        if current_version < info.settings.version {
                            let elements = settings_form.elements();

                            let width_field: web_sys::HtmlInputElement = named_item(&elements, "width");
                            let width = format!("{}", info.settings.width);
                            if width_field.value() != width {
                                width_field.set_value(&width);
                            }

                            let height_field: web_sys::HtmlInputElement = named_item(&elements, "height");
                            let height = format!("{}", info.settings.height);
                            if height_field.value() != height {
                                height_field.set_value(&height);
                            }

                            let score_limit_field: web_sys::HtmlInputElement = named_item(&elements, "score_limit");
                            let score_limit = format!("{}", info.settings.score_limit);
                            if score_limit_field.value() != score_limit {
                                score_limit_field.set_value(&score_limit);
                            }
                        }
                    }
                    NetGameState::Active(_) => {
                        let canvas: web_sys::HtmlCanvasElement = query_selector(main, "canvas");
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
                let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", "DynaMaze");
                main.append_with_node_1(&header).unwrap_throw();

                let tutorial: web_sys::HtmlElement = create_element_with_text(&document, "button", "Tutorial");
                main.append_with_node_1(&tutorial).unwrap_throw();
                listen!(&tutorial, "click", self.tutorial());

                let host: web_sys::HtmlElement = create_element_with_text(&document, "button", "Host Game");
                main.append_with_node_1(&host).unwrap_throw();
                listen!(&host, "click", self.host());

                let connect: web_sys::HtmlElement = create_element_with_text(&document, "button", "Join Game");
                main.append_with_node_1(&connect).unwrap_throw();
                listen!(&connect, "click", self.connect());

                let options: web_sys::HtmlElement = create_element_with_text(&document, "button", "Options");
                main.append_with_node_1(&options).unwrap_throw();
                listen!(&options, "click", self.enter_options());
            }
            GameState::ConnectMenu => {
                let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", "Connect to Game");
                main.append_with_node_1(&header).unwrap_throw();

                let main_menu: web_sys::HtmlElement = create_element_with_text(&document, "button", "Main Menu");
                main.append_with_node_1(&main_menu).unwrap_throw();
                listen!(&main_menu, "click", self.main_menu());

                let connect_form: web_sys::HtmlFormElement = create_element(&document, "form");
                main.append_with_node_1(&connect_form).unwrap_throw();

                let connect_label: web_sys::HtmlElement = create_element_with_text(&document, "label", "Lobby ID");
                connect_form.append_with_node_1(&connect_label).unwrap_throw();

                let connect_text: web_sys::HtmlElement = create_element(&document, "input");
                connect_label
                    .append_with_node_1(&connect_text)
                    .unwrap_throw();

                let connect: web_sys::HtmlElement = create_element_with_text(&document, "button", "Connect");
                connect_form.append_with_node_1(&connect).unwrap_throw();

                listen!(&connect_form, "submit", self.do_connect(connect_form));
            }
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                let is_host = state.is_host(self.player_id);
                match *state {
                    NetGameState::Connecting => {
                        let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", "Connecting...");
                        main.append_with_node_1(&header).unwrap_throw();
                    }
                    NetGameState::Lobby(ref info) => {
                        let status = if is_host {
                            "Hosting lobby"
                        } else {
                            "Connected to lobby"
                        };
                        let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", status);
                        main.append_with_node_1(&header).unwrap_throw();

                        let id = format!("Lobby ID: {}", info.id);
                        let header: web_sys::HtmlElement = create_element_with_text(&document, "h2", &id);
                        main.append_with_node_1(&header).unwrap_throw();

                        let main_menu: web_sys::HtmlElement = create_element_with_text(&document, "button", "Main Menu");
                        main.append_with_node_1(&main_menu).unwrap_throw();
                        listen!(&main_menu, "click", self.main_menu());

                        let players: web_sys::Element = create_element(&document, "ul");
                        main.append_with_node_1(&players).unwrap_throw();

                        for player_info in info.players_ref() {
                            let is_local = player_info.lives_with(self.player_id);
                            let player = create_player!(player_info, is_local);
                            players.append_with_node_1(&player).unwrap_throw();
                        }

                        let new_local: web_sys::HtmlElement = create_element_with_text(&document, "button", "New Local Player");
                        main.append_with_node_1(&new_local).unwrap_throw();
                        listen!(&new_local, "click", self.new_local_player());

                        let settings_form: web_sys::HtmlElement = create_element(&document, "form");
                        settings_form.dataset().set("version", &format!("{}", info.settings.version)).unwrap_throw();
                        main.append_with_node_1(&settings_form).unwrap_throw();

                        let width_label: web_sys::HtmlElement = create_element_with_text(&document, "label", "Board Width");
                        settings_form.append_with_node_1(&width_label).unwrap_throw();
                        let width: web_sys::HtmlInputElement = create_element(&document, "input");
                        width.set_name("width");
                        width.set_type("number");
                        width.set_min("3");
                        width.set_max("21");
                        width.set_step("2");
                        width.set_value(&format!("{}", info.settings.width));
                        listen!(&width, "input", self.set_width(width));
                        width_label.append_with_node_1(&width).unwrap_throw();

                        let height_label: web_sys::HtmlElement = create_element_with_text(&document, "label", "Board Height");
                        settings_form.append_with_node_1(&height_label).unwrap_throw();
                        let height: web_sys::HtmlInputElement = create_element(&document, "input");
                        height.set_name("height");
                        height.set_type("number");
                        height.set_min("3");
                        height.set_max("21");
                        height.set_step("2");
                        height.set_value(&format!("{}", info.settings.height));
                        listen!(&height, "input", self.set_height(height));
                        height_label.append_with_node_1(&height).unwrap_throw();

                        let score_limit_label: web_sys::HtmlElement = create_element_with_text(&document, "label", "Score Limit");
                        settings_form.append_with_node_1(&score_limit_label).unwrap_throw();
                        let score_limit: web_sys::HtmlInputElement = create_element(&document, "input");
                        score_limit.set_name("score_limit");
                        score_limit.set_type("number");
                        score_limit.set_min("1");
                        score_limit.set_max("20");
                        score_limit.set_step("1");
                        score_limit.set_value(&format!("{}", info.settings.score_limit));
                        listen!(&score_limit, "input", self.set_score_limit(score_limit));
                        score_limit_label.append_with_node_1(&score_limit).unwrap_throw();

                        if is_host {
                            let start: web_sys::HtmlElement = create_element_with_text(&document, "button", "Begin Game");
                            main.append_with_node_1(&start).unwrap_throw();
                            listen!(&start, "click", self.start_hosted_game());
                        }
                    }
                    NetGameState::Active(_) => {
                        let canvas: web_sys::HtmlCanvasElement = create_element(&document, "canvas");
                        main.append_with_node_1(&canvas).unwrap_throw();
                    }
                    NetGameState::GameOver(ref info) => {
                        let text = format!("{} wins!", info.winner.name);
                        let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", &text);
                        main.append_with_node_1(&header).unwrap_throw();

                        let main_menu: web_sys::HtmlElement = create_element_with_text(&document, "button", "Main Menu");
                        main.append_with_node_1(&main_menu).unwrap_throw();
                        listen!(&main_menu, "click", self.main_menu());
                    }
                    NetGameState::Error(ref text) => {
                        let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", "Error");
                        main.append_with_node_1(&header).unwrap_throw();

                        let body: web_sys::HtmlElement = create_element_with_text(&document, "p", text);
                        main.append_with_node_1(&body).unwrap_throw();

                        let main_menu: web_sys::HtmlElement = create_element_with_text(&document, "button", "Main Menu");
                        main.append_with_node_1(&main_menu).unwrap_throw();
                        listen!(&main_menu, "click", self.main_menu());
                    }
                }
            }
            GameState::HardError(ref text) => {
                let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", "Error");
                main.append_with_node_1(&header).unwrap_throw();

                let body: web_sys::HtmlElement = create_element_with_text(&document, "p", text);
                main.append_with_node_1(&body).unwrap_throw();

                let main_menu: web_sys::HtmlElement = create_element_with_text(&document, "button", "Main Menu");
                main.append_with_node_1(&main_menu).unwrap_throw();
                listen!(&main_menu, "click", self.main_menu());
            }
            GameState::Options(ref curr_options) => {
                let header: web_sys::HtmlElement = create_element_with_text(&document, "h1", "Options");
                main.append_with_node_1(&header).unwrap_throw();

                let music: web_sys::Element = create_element(&document, "label");
                let music_label = document.create_text_node("Music Level");
                music.append_with_node_1(&music_label).unwrap_throw();
                let music_slider: web_sys::HtmlInputElement = create_element(&document, "input");
                music_slider.set_type("range");
                music_slider.set_value(&format!("{}", curr_options.music_level));
                listen!(&music_slider, "input", self.set_music_level(music_slider));
                music.append_with_node_1(&music_slider).unwrap_throw();
                main.append_with_node_1(&music).unwrap_throw();

                let sound: web_sys::Element = create_element(&document, "label");
                let sound_label = document.create_text_node("Sound Level");
                sound.append_with_node_1(&sound_label).unwrap_throw();
                let sound_slider: web_sys::HtmlInputElement = create_element(&document, "input");
                sound_slider.set_type("range");
                sound_slider.set_value(&format!("{}", curr_options.sound_level));
                listen!(&sound_slider, "input", self.set_sound_level(sound_slider));
                sound.append_with_node_1(&sound_slider).unwrap_throw();
                main.append_with_node_1(&sound).unwrap_throw();

                let save_button: web_sys::HtmlElement = create_element_with_text(&document, "button", "Save");
                main.append_with_node_1(&save_button).unwrap_throw();
                listen!(&save_button, "click", self.save_options());

                let main_menu: web_sys::HtmlElement = create_element_with_text(&document, "button", "Main Menu");
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
