//! Menu / Game view

use web_sys::CanvasRenderingContext2d as Context;

use crate::{BoardView, BoardViewSettings, GameController};
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
        ctx.clear_rect(0.0, 0.0, 10000.0, 10000.0); // TODO don't do this
        ctx.restore();
        match controller.state {
            GameState::MainMenu => {}
            GameState::ConnectMenu => {}
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to acquire state mutex");
                match *state {
                    NetGameState::Connecting => {}
                    NetGameState::Lobby(_) => {}
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
