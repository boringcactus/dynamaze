//! Menu / Game view

use crate::{BoardView, BoardViewSettings};
use crate::GameController;
use crate::menu::{GameState, NetGameState};

use graphics::{Context, Graphics};
use graphics::character::CharacterCache;

/// Stores visual information about the game
pub struct GameView {
    /// Stores board view settings
    pub board_view: BoardView,
}

impl GameView {
    /// Create a new GameView for a screen with the given \[width, height\]
    pub fn new(size: [f64; 2]) -> GameView {
        GameView {
            board_view: BoardView::new(BoardViewSettings::new(size)),
        }
    }

    /// Draw game
    pub fn draw<G: Graphics, C>(
        &self, controller: &GameController,
        glyphs: &mut C, c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::Transformed;
        match controller.state {
            GameState::MainMenu => {
                // TODO don't do this
                let black = [0.0, 0.0, 0.0, 1.0];
                let text = "Left-click to host local game, right-click to connect to local game";
                let transform = c.transform.trans(0.0, 60.0);
                graphics::text(black, 20, text, glyphs, transform, g).ok().expect("Failed to draw text");
            },
            GameState::ConnectMenu(ref address) => {
                // TODO don't do this
                let black = [0.0, 0.0, 0.0, 1.0];
                let text = format!("Left-click to connect to {}", address);
                let transform = c.transform.trans(0.0, 60.0);
                graphics::text(black, 20, &text, glyphs, transform, g).ok().expect("Failed to draw text");
            },
            GameState::InGame(NetGameState::Lobby(ref info)) => {
                // TODO don't do this
                let black = [0.0, 0.0, 0.0, 1.0];
                let text = format!("Hosting on {}, click anywhere to start game", info.host.address);
                let transform = c.transform.trans(0.0, 60.0);
                graphics::text(black, 20, &text, glyphs, transform, g).ok().expect("Failed to draw text");
            },
            GameState::InGame(NetGameState::GameOver(_)) => unimplemented!("Game over isn't real yet"),
            GameState::InGame(NetGameState::Active(ref board_controller)) => {
                self.board_view.draw(board_controller, glyphs, c, g);
            }
        }
    }
}
