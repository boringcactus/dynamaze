//! Menu / Game view

use graphics::{Context, Graphics};
use graphics::character::CharacterCache;

use crate::{BoardView, BoardViewSettings, colors, GameController};
use crate::menu::{GameState, NetGameState};

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
        glyphs: &mut C, c: &Context, g: &mut G,
    ) where C: CharacterCache<Texture=G::Texture> {
        use graphics::Transformed;
        match controller.state {
            GameState::MainMenu => {}
            GameState::ConnectMenu(_) => {}
            GameState::InGame(ref conn_state) => {
                let ref state = conn_state.state;
                let state = state.read().expect("Failed to acquire state mutex");
                match *state {
                    NetGameState::Lobby(ref info) => {
                        let transform = c.transform.trans(0.0, 120.0);
                        // TODO edit lobby name
                        graphics::text(colors::DARK.into(), 20, &info.name, glyphs, transform, g).ok().expect("Failed to draw text");
                        let mut transform = transform.trans(0.0, 30.0);
                        for player in info.players_ref() {
                            // TODO don't do this
                            let is_me = player.id == controller.player_id;
                            let x_offset = if is_me { 20.0 } else { 0.0 };
                            graphics::ellipse(player.color.into(), [0.0, -15.0, 15.0, 15.0], transform.trans(x_offset, 0.0), g);
                            graphics::text(colors::DARK.into(), 15, &player.name, glyphs, transform.trans(x_offset + 20.0, 0.0), g).ok().expect("Failed to draw text");
                            transform = transform.trans(0.0, 30.0);
                        }
                    }
                    NetGameState::Active(ref board_controller) => {
                        self.board_view.draw(board_controller, &controller.player_id, glyphs, c, g);
                    }
                    NetGameState::GameOver(_) => {}
                    NetGameState::Error(_) => {}
                }
            }
        }
    }

    /// Grabs a Conrod theme
    pub fn theme(&self) -> conrod_core::Theme {
        use conrod_core::position::{Align, Padding, Position, Relative};
        let light = colors::LIGHT.into();
        conrod_core::Theme {
            name: "DynaMaze Theme".to_string(),
            padding: Padding::none(),
            x_position: Position::Relative(Relative::Align(Align::Start), None),
            y_position: Position::Relative(Relative::Align(Align::Start), None),
            background_color: light,
            shape_color: light.highlighted(),
            border_color: colors::BLUE.into(),
            border_width: 0.0,
            label_color: colors::DARK.into(),
            font_id: None,
            font_size_large: 26,
            font_size_medium: 18,
            font_size_small: 12,
            widget_styling: conrod_core::theme::StyleMap::default(),
            mouse_drag_threshold: 0.0,
            double_click_threshold: std::time::Duration::from_millis(500),
        }
    }
}
