//! Menu / global state controller
extern crate clipboard;

use std::collections::VecDeque;
use std::ffi::CString;
use std::sync::{Arc, RwLock};

use clipboard::{ClipboardContext, ClipboardProvider};
use discord_game_sdk::event::activities::Join;
use discord_game_sdk::event::lobbies::MemberConnect;
use piston::input::{GenericEvent, Key};
use rand::prelude::*;

use crate::{BoardController, BoardSettings, GameView, Player, PlayerID};
use crate::anim;
use crate::colors;
use crate::demo;
use crate::discord::{Activity, DiscordHandle};
use crate::menu::{ConnectedState, GameOverInfo, GameState, LobbyInfo, NetGameState};
use crate::net::{Message, MessageCtrl};
use crate::options;
use crate::sound;
use crate::tutorial;

widget_ids! {
    pub struct Ids {
        canvas,
        menu_header,
        tutorial_button,
        host_button,
        connect_button,
        options_button,
        ip_box,
        lobby_name,
        color_button,
        name_box,
        start_button,
        color_demo,
        new_local_button,
        copy_secret_button,
        main_menu_button,
        error_text,
        music_slider,
        sound_slider,
        save_button,
    }
}

/// Handles events for DynaMaze game
pub struct GameController {
    /// Game state
    pub state: GameState,
    /// Current player ID
    pub player_id: PlayerID,
    /// Whether or not the shift key is currently pressed
    pub shift: bool,
    /// Whether or not the ctrl key is currently pressed
    pub ctrl: bool,
    /// Active player ID the last time the state was checked for a notification
    pub last_player: Option<PlayerID>,
}

impl GameController {
    /// Creates a new GameController
    pub fn new() -> GameController {
        if demo::is_demo() {
            return demo::new_controller();
        }
        let player_id = random();
        sound::SOUND.play_music(sound::Music::Menu);
        GameController {
            state: GameState::MainMenu,
            player_id,
            shift: false,
            ctrl: false,
            last_player: None,
        }
    }

    fn tutorial(&mut self) {
        self.state = GameState::InGame(tutorial::new_conn_state(self.player_id));
    }

    fn host(&mut self, discord: &mut DiscordHandle) {
        let state = NetGameState::Lobby(LobbyInfo::new(self.player_id, discord.my_name()));
        let state = Arc::new(RwLock::new(state));
        discord.create_lobby();
        let conn_state = ConnectedState {
            state,
            outbox: VecDeque::new(),
        };
        self.state = GameState::InGame(conn_state);
    }

    fn connect(&mut self) {
        self.state = GameState::ConnectMenu("".into());
    }

    fn enter_options(&mut self) {
        self.state = GameState::Options(options::HANDLE.fetch().clone());
    }

    fn do_connect(&mut self, discord: &mut DiscordHandle) {
        if let GameState::ConnectMenu(ref address) = self.state {
            let state = NetGameState::Error("Connecting...".to_string());
            let state = Arc::new(RwLock::new(state));
            discord.join_lobby(address.clone());
            let player = Player::new(discord.my_name(), random(), self.player_id);
            let mut outbox = VecDeque::new();
            outbox.push_back(Message::JoinLobby(player).into());
            let conn_state = ConnectedState {
                state,
                outbox,
            };
            self.state = GameState::InGame(conn_state);
        }
    }

    /// Handle an incoming Join event from Discord
    pub fn handle_join(&mut self, join: Join, discord: &mut DiscordHandle) {
        self.state = GameState::ConnectMenu(join.secret);
        self.do_connect(discord);
    }

    /// Handle a member connection event from Discord
    pub fn handle_connect(&mut self, _connect: MemberConnect, _discord: &mut DiscordHandle) {
        self.broadcast_state();
    }

    fn save_options(&mut self) {
        if let GameState::Options(ref opts) = self.state {
            options::HANDLE.save(opts);
            self.state = GameState::MainMenu;
            sound::SOUND.fetch_volume();
        }
    }

    fn randomize_color(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            let outbox = &mut conn_state.outbox;
            if let NetGameState::Lobby(ref mut info) = *state {
                let player = info.player_mut(&self.player_id);
                player.color = random();
                let message = Message::EditPlayer(self.player_id, player.clone());
                outbox.push_back(message.into());
            }
        }
    }

    fn set_own_name(&mut self, new_name: &str) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let outbox = &mut conn_state.outbox;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                let player = info.player_mut(&self.player_id);
                player.name = new_name.to_string();
                let message = Message::EditPlayer(self.player_id, player.clone());
                outbox.push_back(message.into());
            }
        }
    }

    fn new_local_player(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let outbox = &mut conn_state.outbox;
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                if let Some(me) = info.player(&self.player_id) {
                    let child = Player::new_child(me.name.clone(), me.color, random(), me.id);
                    info.players.push(child.clone());
                    outbox.push_back(Message::JoinLobby(child).into());
                }
            }
        }
    }

    fn start_hosted_game(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let state = &mut conn_state.state;
            let mut state = state.write().expect("Failed to lock state");
            if let NetGameState::Lobby(ref mut info) = *state {
                let players = info.players.clone();
                // TODO edit these
                let settings = BoardSettings {
                    width: 7,
                    height: 7,
                    score_limit: 10,
                };
                let board_controller = BoardController::new(settings, players);
                let net_state = NetGameState::Active(board_controller);
                *state = net_state;
                drop(state);
                self.broadcast_state();
            }
        }
    }

    fn copy_secret(&self) {
        if let GameState::InGame(ref conn_state) = self.state {
            let state = &conn_state.state;
            let state = state.read().expect("Failed to lock state");
            if let NetGameState::Lobby(ref info) = *state {
                let secret = &info.join_secret;
                if let Some(ref secret) = secret {
                    let mut ctx: ClipboardContext = ClipboardProvider::new().expect("Failed to copy");
                    ctx.set_contents(secret.clone()).expect("Failed to copy");
                }
            }
        }
    }

    fn main_menu(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let outbox = &mut conn_state.outbox;
            outbox.push_back(MessageCtrl::Disconnect);
            println!("Attempted to disconnect");
        }
        sound::SOUND.fetch_volume();
        self.state = GameState::MainMenu;
    }

    /// Retrieves current game state for Discord
    pub fn activity(&self, discord: &mut DiscordHandle) -> Activity {
        let mut result = Activity::empty();
        match self.state {
            GameState::MainMenu | GameState::Options(_) | GameState::ConnectMenu(_) | GameState::HardError(_) => {
                let state = CString::new("In Menus").unwrap();
                result.with_state(&state);
            }
            GameState::InGame(ref state) => {
                let state = state.state.read().unwrap();
                match *state {
                    NetGameState::Lobby(ref lobby) => {
                        let state = CString::new("In Lobby").unwrap();
                        result.with_state(&state);
                        result.with_party_amount((lobby.players.len()) as i32);
                        result.with_party_capacity(crate::MAX_PLAYERS as i32);

                        if let Some((id, secret)) = discord.get_id_and_secret() {
                            let id = format!("{}", id);
                            let id = CString::new(id).unwrap();
                            result.with_party_id(&id);
                            let secret = CString::new(secret).unwrap();
                            result.with_join_secret(&secret);
                        }
                    }
                    NetGameState::Active(ref board) => {
                        let state = CString::new("In Game").unwrap();
                        result.with_state(&state);
                        result.with_party_amount(board.players.len() as i32);
                        result.with_party_capacity(board.players.len() as i32);
                    }
                    NetGameState::Error(_) | NetGameState::GameOver(_) => {
                        let state = CString::new("In Menus").unwrap();
                        result.with_state(&state);
                    }
                }
            }
        }
        result
    }

    /// Handles events
    pub fn event<E: GenericEvent>(&mut self, view: &GameView, e: &E, discord: &mut DiscordHandle) {
        use piston::input::Button;

        if let Some(args) = e.update_args() {
            anim::STATE.write().unwrap().advance_by(args.dt);

            if let GameState::InGame(ref state) = self.state {
                let mut state = state.state.write().unwrap();
                if let NetGameState::Lobby(ref mut lobby) = *state {
                    lobby.join_secret = discord.get_id_and_secret().map(|(_, x)| x);
                }
            }
        }

        // TODO only do this when a turn actually ends
        if e.update_args().is_some() {
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
            sound::SOUND.play_music(music);

            if old_last_player != self.last_player && self.last_player == Some(self.player_id) {
                sound::SOUND.play_sound(sound::Sound::YourTurn);
            }
        }

        if let Some(state) = e.button_args() {
            if state.button == Button::Keyboard(Key::LShift) || state.button == Button::Keyboard(Key::RShift) {
                use piston::input::ButtonState;
                self.shift = state.state == ButtonState::Press;
            }
            if state.button == Button::Keyboard(Key::LCtrl) || state.button == Button::Keyboard(Key::RCtrl) {
                use piston::input::ButtonState;
                self.ctrl = state.state == ButtonState::Press;
            }
        }

        match self.state {
            GameState::MainMenu => {}
            GameState::ConnectMenu(ref mut address) => {
                // shout out to conrod for not supporting Ctrl-V in text boxes, what the fuck
                if let Some(Button::Keyboard(Key::V)) = e.press_args() {
                    if self.ctrl {
                        let mut ctx: ClipboardContext = ClipboardProvider::new().expect("Failed to paste");
                        *address = ctx.get_contents().expect("Failed to paste");
                    }
                }
            }
            GameState::InGame(ref mut conn_state) => {
                let outbox = &mut conn_state.outbox;
                let state = &mut conn_state.state;
                let (broadcast, new_state, new_net_state) = {
                    let mut state = state.write().expect("Failed to lock state");
                    match *state {
                        NetGameState::Lobby(_) => {
                            (false, None, None)
                        }
                        NetGameState::Active(ref mut board_controller) => {
                            let state_dirty = board_controller.event(&view.board_view, e, self.player_id, outbox);
                            if state_dirty {
                                if let Some(winner) = board_controller.winner() {
                                    let info = GameOverInfo {
                                        winner: winner.clone(),
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
                            (false, None, None)
                        }
                        NetGameState::Error(_) => {
                            (false, None, None)
                        }
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
            GameState::HardError(_) => {}
            GameState::Options(_) => {}
        }
    }

    /// Handles an incoming network message from the given user ID
    pub fn handle_incoming(&mut self, message: Message) {
        if let GameState::InGame(ref mut state) = self.state {
            let outbox = &mut state.outbox;
            let mut state = state.state.write().expect("Failed to acquire state");
            match message {
                Message::JoinLobby(player) => {
                    if let NetGameState::Lobby(ref mut lobby_info) = *state {
                        lobby_info.players.push(player);
                        outbox.push_back(MessageCtrl::send(Message::State(state.clone())));
                    }
                }
                Message::EditPlayer(id, player) => {
                    if let NetGameState::Lobby(ref mut lobby_info) = *state {
                        lobby_info.players.iter_mut().filter(|p| p.id == id).for_each(|p| *p = player.clone());
                    }
                }
                Message::State(new_state) => {
                    // TODO only accept state from active player, probably by connecting player ID to source SocketAddr
                    *state = new_state;
                }
                Message::Anim(sync) => {
                    anim::STATE.write().unwrap().apply(sync);
                }
            }
        }
    }

    /// Sends all pending messages through the given Discord handle
    pub fn send_all(&mut self, discord: &mut DiscordHandle) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let outbox = &mut conn_state.outbox;
            discord.drain_messages(outbox);
        }
    }

    fn broadcast_state(&mut self) {
        if let GameState::InGame(ref mut conn_state) = self.state {
            let outbox = &mut conn_state.outbox;
            let state = &mut conn_state.state;
            let state = state.read().expect("Failed to lock state");
            let message = Message::State(state.clone());
            outbox.push_back(message.into());
        }
    }

    /// Run Conrod GUI
    pub fn gui(&mut self, ui: &mut conrod_core::UiCell, ids: &Ids, discord: &mut DiscordHandle) {
        use conrod_core::{widget, Colorable, Labelable, Positionable, Sizeable, Widget};

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
                    self.host(discord);
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
                            self.do_connect(discord);
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
                    self.do_connect(discord);
                }
            }
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to lock state");
                match *state {
                    NetGameState::Lobby(ref info) => {
                        widget::Text::new("Connected to lobby")
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

                        let no_name = "loading...".to_string();
                        let my_name = me.map(|x| &x.name).unwrap_or(&no_name);
                        let name_box = widget::TextBox::new(&my_name)
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

                        let my_color = me.map(|x| x.color).unwrap_or(colors::DARK);
                        widget::Circle::fill(MARGIN / 2.0)
                            .color(my_color.into())
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

                        let copy_secret_button = widget::Button::new()
                            .label("Copy Join Secret")
                            .color(conrod_core::color::WHITE.with_alpha(0.4))
                            .label_color(colors::DARK.into())
                            .wh(BUTTON_DIMENSIONS)
                            .align_right_of(ids.name_box)
                            .down_from(ids.new_local_button, MARGIN)
                            .set(ids.copy_secret_button, ui);
                        for _press in copy_secret_button {
                            defer!(self.copy_secret());
                        }

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
