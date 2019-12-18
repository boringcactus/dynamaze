//! Menu / Game view

use quicksilver::{
    lifecycle::Window,
};

use crate::{BoardView, BoardViewSettings, GameController};
use crate::menu::{GameState, NetGameState};

/// Stores visual information about the game
pub struct GameView {
    /// Stores board view settings
    pub board_view: BoardView,
}

impl GameView {
    /// Create a new GameView for a screen with the given \[width, height\]
    pub fn new() -> GameView {
        GameView {
            board_view: BoardView::new(BoardViewSettings::new()),
        }
    }

    /// Draw game
    pub fn draw(
        &self, controller: &GameController,
        window: &mut Window,
    ) -> quicksilver::Result<()> {
        match controller.state {
            GameState::MainMenu => {}
            GameState::ConnectMenu(_) => {}
            GameState::InGame(ref conn_state) => {
                let state = &conn_state.state;
                let state = state.read().expect("Failed to acquire state mutex");
                match *state {
                    NetGameState::Lobby(ref info) => {
//                        let mut transform = c.transform.trans(0.0, 150.0);
                        for player in info.players_ref() {
                            // TODO don't do this
                            let is_me = player.id == controller.player_id;
                            let x_offset = if is_me { 20.0 } else { 0.0 };
//                            graphics::ellipse(player.color.into(), [0.0, -15.0, 15.0, 15.0], transform.trans(x_offset, 0.0), g);
//                            graphics::text(colors::DARK.into(), 15, &player.name, glyphs, transform.trans(x_offset + 20.0, 0.0), g).ok().expect("Failed to draw text");
//                            transform = transform.trans(0.0, 30.0);
                        }
                    }
                    NetGameState::Active(ref board_controller) => {
                        self.board_view.draw(board_controller, controller.player_id, window)?;
                    }
                    NetGameState::GameOver(_) => {}
                    NetGameState::Error(_) => {}
                }
            }
            GameState::HardError(_) => {}
            GameState::Options(_) => {}
        }
        Ok(())
    }
}
