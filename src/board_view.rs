//! Board view

use graphics::types::Color;
use graphics::{Context, Graphics};
use graphics::character::CharacterCache;

use crate::BoardController;

/// Stores board view settings
pub struct BoardViewSettings {
    /// Position from top left corner
    pub position: [f64; 2],
    /// Size of board
    pub size: f64,
    /// Background color
    pub background_color: Color,
    /// Border color
    pub border_color: Color,
    /// Edge color around the whole board
    pub board_edge_color: Color,
    /// Edge color between the 3x3 sections
    pub section_edge_color: Color,
    /// Edge color between cells
    pub cell_edge_color: Color,
    /// Edge radius around the whole board
    pub board_edge_radius: f64,
    /// Edge radius between the 3x3 sections
    pub section_edge_radius: f64,
    /// Edge radius between cells
    pub cell_edge_radius: f64,
    /// Selected cell background color
    pub selection_background_color: Color,
    /// Text color
    pub text_color: Color,
}

impl BoardViewSettings {
    /// Creates new board view settings
    pub fn new() -> BoardViewSettings {
        BoardViewSettings {
            position: [10.0; 2],
            size: 400.0,
            background_color: [0.8, 0.8, 1.0, 1.0],
            border_color: [0.0, 0.0, 0.2, 1.0],
            board_edge_color: [0.0, 0.0, 0.2, 1.0],
            section_edge_color: [0.0, 0.0, 0.2, 1.0],
            cell_edge_color: [0.0, 0.0, 0.2, 1.0],
            board_edge_radius: 3.0,
            section_edge_radius: 2.0,
            cell_edge_radius: 1.0,
            selection_background_color: [0.9, 0.9, 1.0, 1.0],
            text_color: [0.0, 0.0, 0.1, 1.0],
        }
    }
}

/// Stores visual information about a board
pub struct BoardView {
    /// Stores board view settings
    pub settings: BoardViewSettings,
}

impl BoardView {
    /// Creates a new board view
    pub fn new(settings: BoardViewSettings) -> BoardView {
        BoardView {
            settings,
        }
    }

    /// Draw board
    pub fn draw<G: Graphics, C>(
        &self, controller: &BoardController,
        glyphs: &mut C,
        c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::{Line, Rectangle, Image, Transformed};

        let ref settings = self.settings;
        let board_rect = [
            settings.position[0], settings.position[1],
            settings.size, settings.size
        ];

        Rectangle::new(settings.background_color)
            .draw(board_rect, &c.draw_state, c.transform, g);

        if let Some(ind) = controller.selection {
            let cell_size = settings.size / 9.0;
            let pos = [ind[0] as f64 * cell_size, ind[1] as f64 * cell_size];
            let cell_rect = [
                settings.position[0] + pos[0], settings.position[1] + pos[1],
                cell_size, cell_size
            ];
            Rectangle::new(settings.selection_background_color)
                .draw(cell_rect, &c.draw_state, c.transform, g);
        }

        let text_image = Image::new_color(settings.text_color);
        let cell_size = settings.size / 9.0;
        for j in 0..9 {
            for i in 0..9 {
                if let Some(ch) = controller.board.char([i, j]) {
                    let pos = [
                        settings.position[0] + i as f64 * cell_size + 15.0,
                        settings.position[1] + j as f64 * cell_size + 34.0
                    ];
                    if let Ok(character) = glyphs.character(34, ch) {
                        let ch_x = pos[0] + character.left();
                        let ch_y = pos[1] - character.top();
                        text_image.draw(character.texture,
                                        &c.draw_state,
                                        c.transform.trans(ch_x, ch_y),
                                        g);
                    }
                }
            }
        }

        let cell_edge = Line::new(settings.cell_edge_color, settings.cell_edge_radius);
        for i in 0..9 {
            if (i % 3) == 0 {
                continue;
            }

            let x = settings.position[0] + i as f64 / 9.0 * settings.size;
            let y = settings.position[1] + i as f64 / 9.0 * settings.size;
            let x2 = settings.position[0] + settings.size;
            let y2 = settings.position[1] + settings.size;

            let vline = [x, settings.position[1], x, y2];
            cell_edge.draw(vline, &c.draw_state, c.transform, g);

            let hline = [settings.position[0], y, x2, y];
            cell_edge.draw(hline, &c.draw_state, c.transform, g);
        }

        // Draw section borders.
        let section_edge = Line::new(settings.section_edge_color, settings.section_edge_radius);
        for i in 0..3 {
            // Set up coordinates.
            let x = settings.position[0] + i as f64 / 3.0 * settings.size;
            let y = settings.position[1] + i as f64 / 3.0 * settings.size;
            let x2 = settings.position[0] + settings.size;
            let y2 = settings.position[1] + settings.size;

            let vline = [x, settings.position[1], x, y2];
            section_edge.draw(vline, &c.draw_state, c.transform, g);

            let hline = [settings.position[0], y, x2, y];
            section_edge.draw(hline, &c.draw_state, c.transform, g);
        }

        // Draw board edge.
        Rectangle::new_border(settings.board_edge_color, settings.board_edge_radius)
            .draw(board_rect, &c.draw_state, c.transform, g);
    }
}
