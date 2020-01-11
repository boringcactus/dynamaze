//! Menu / Game view

use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d as Context;

use crate::{BoardView, BoardViewSettings, colors, GameController};
use crate::menu::{GameState, NetGameState};

/// Stores visual information about the game
pub struct GameView {
    /// Stores board view settings
    pub board_view: BoardView,
}

impl GameView {
    /// Create a new GameView
    pub fn new() -> GameView {
        GameView {
            board_view: BoardView::new(BoardViewSettings::new()),
        }
    }

    /// Draw game
    pub fn draw(&self, controller: &GameController, ctx: &Context) {
        ctx.save();
        ctx.set_fill_style(&colors::LIGHT.into());
        ctx.fill_rect(0.0, 0.0, 10000.0, 10000.0); // TODO don't do this
        ctx.restore();
        match controller.state {
            GameState::MainMenu => {}
            GameState::ConnectMenu => {}
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to acquire state mutex");
                match *state {
                    NetGameState::Lobby(ref info) => {
                        ctx.save();
                        let mut y = 150.0;
                        for player in info.players_ref() {
                            // TODO make this more robust
                            let is_me = player.id == controller.player_id;
                            let x_offset = if is_me { 20.0 } else { 0.0 };
                            ctx.begin_path();
                            ctx.set_fill_style(&player.color.into());
                            ctx.ellipse(
                                x_offset,
                                y - 15.0,
                                15.0,
                                15.0,
                                0.0,
                                0.0,
                                ::std::f64::consts::PI * 2.0,
                            )
                                .unwrap_throw();
                            ctx.fill();
                            ctx.set_fill_style(&colors::DARK.into());
                            ctx.set_font("15pt sans-serif");
                            ctx.fill_text(&player.name, x_offset + 20.0, y)
                                .unwrap_throw();
                            y += 30.0;
                        }
                        ctx.restore();
                    }
                    NetGameState::Active(ref board_controller) => {
                        self.board_view
                            .draw(board_controller, controller.player_id, ctx);
                    }
                    NetGameState::GameOver(_) => {}
                    NetGameState::Error(_) => {}
                }
            }
            GameState::HardError(_) => {}
            GameState::Options(_) => {}
        }
    }
}

impl Default for GameView {
    fn default() -> Self {
        Self::new()
    }
}
