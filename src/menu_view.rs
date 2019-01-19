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
                let text = "Left-click to host game, right-click to connect to game";
                let transform = c.transform.trans(0.0, 60.0);
                graphics::text(black, 20, text, glyphs, transform, g).ok().expect("Failed to draw text");
            },
            GameState::ConnectMenu(ref address) => {
                // TODO don't do this
                let black = [0.0, 0.0, 0.0, 1.0];
                let text = format!("Type an address, left-click to connect: {}", address);
                let transform = c.transform.trans(0.0, 60.0);
                graphics::text(black, 20, &text, glyphs, transform, g).ok().expect("Failed to draw text");
            },
            GameState::InGame(ref conn_state) => {
                let ref state = conn_state.state;
                match state {
                    NetGameState::Lobby(ref info) => {
                        // TODO don't do this
                        let black = [0.0, 0.0, 0.0, 1.0];
                        let port = conn_state.connection.socket.local_addr().unwrap().port();
                        let status = if state.is_host(&controller.player_id) {
                            format!("Hosting on port {}, ", port)
                        } else {
                            "Connected to lobby".to_string()
                        };
                        let transform = c.transform.trans(0.0, 60.0);
                        graphics::text(black, 20, &status, glyphs, transform, g).ok().expect("Failed to draw text");
                        let transform = transform.trans(0.0, 30.0);
                        let header = if state.is_host(&controller.player_id) {
                            "Left-click to randomize your color, type to edit your name, right-click to start game"
                        } else {
                            "Left-click to randomize your color, type to edit your name"
                        };
                        graphics::text(black, 20, header, glyphs, transform, g).ok().expect("Failed to draw text");
                        let transform = transform.trans(0.0, 30.0);
                        // TODO edit name
                        graphics::text(black, 20, &info.name, glyphs, transform, g).ok().expect("Failed to draw text");
                        let mut transform = transform.trans(0.0, 30.0);
                        for player in info.players_ref() {
                            // TODO don't do this
                            let is_me = player.id == controller.player_id;
                            let x_offset = if is_me { 20.0 } else { 0.0 };
                            graphics::ellipse(player.color, [0.0, -15.0, 15.0, 15.0], transform.trans(x_offset, 0.0), g);
                            graphics::text(black, 15, &player.name, glyphs, transform.trans(x_offset + 20.0, 0.0), g).ok().expect("Failed to draw text");
                            transform = transform.trans(0.0, 30.0);
                        }
                    },
                    NetGameState::Active(ref board_controller) => {
                        self.board_view.draw(board_controller, &controller.player_id, glyphs, c, g);
                    }
                    NetGameState::GameOver(ref info) => {
                        // TODO don't do this
                        let black = [0.0, 0.0, 0.0, 1.0];
                        let text = format!("{} wins! Click to return to main menu", info.winner.name);
                        let transform = c.transform.trans(0.0, 60.0);
                        graphics::text(black, 20, &text, glyphs, transform, g).ok().expect("Failed to draw text");
                    },
                }
            }
        }
    }
}
