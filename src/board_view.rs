//! Board view

use graphics::types::Color;
use graphics::{Context, Graphics};
use graphics::character::CharacterCache;

use crate::board;
use crate::BoardController;
use crate::Direction;

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
    /// Wall color
    pub wall_color: Color,
    /// Tile wall width
    pub wall_width: f64,
    /// Insert guide color
    pub insert_guide_color: Color,
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
            wall_color: [0.2, 0.2, 0.3, 1.0],
            wall_width: 15.0,
            insert_guide_color: [0.6, 0.2, 0.6, 1.0],
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
        use graphics::{Line, Rectangle, Polygon};

        let ref settings = self.settings;
        let cell_size = settings.size / (board::SIZE as f64 + 2.0);

        // draw board
        let board_rect = [
            settings.position[0] + cell_size, settings.position[1] + cell_size,
            settings.size - 2.0 * cell_size, settings.size - 2.0 * cell_size
        ];
        Rectangle::new(settings.background_color)
            .draw(board_rect, &c.draw_state, c.transform, g);

        // draw the tiles
        let wall_rect = Rectangle::new(settings.wall_color);
        for j in 0..board::SIZE {
            for i in 0..board::SIZE {
                let north = settings.position[1] + (j + 1) as f64 * cell_size;
                let south = north + cell_size;
                let south_ish = south - settings.wall_width;
                let west = settings.position[0] + (i + 1) as f64 * cell_size;
                let east = west + cell_size;
                let east_ish = east - settings.wall_width;

                wall_rect.draw([west, north, settings.wall_width, settings.wall_width], &c.draw_state, c.transform, g);
                wall_rect.draw([east_ish, north, settings.wall_width, settings.wall_width], &c.draw_state, c.transform, g);
                wall_rect.draw([west, south_ish, settings.wall_width, settings.wall_width], &c.draw_state, c.transform, g);
                wall_rect.draw([east_ish, south_ish, settings.wall_width, settings.wall_width], &c.draw_state, c.transform, g);

                let walled_directions = controller.board.get([i, j]).walls();

                for d in walled_directions {
                    let rect = match d {
                        Direction::North => [west, north, cell_size, settings.wall_width],
                        Direction::South => [west, south_ish, cell_size, settings.wall_width],
                        Direction::East => [east_ish, north, settings.wall_width, cell_size],
                        Direction::West => [west, north, settings.wall_width, cell_size],
                    };
                    wall_rect.draw(rect, &c.draw_state, c.transform, g);
                }
            }
        }

        // draw tile edges
        let cell_edge = Line::new(settings.cell_edge_color, settings.cell_edge_radius);
        for i in 0..board::SIZE {
            let x = settings.position[0] + (i + 1) as f64 / (board::SIZE as f64 + 2.0) * settings.size;
            let y = settings.position[1] + (i + 1) as f64 / (board::SIZE as f64 + 2.0) * settings.size;
            let x2 = settings.position[0] + settings.size - cell_size;
            let y2 = settings.position[1] + settings.size - cell_size;

            let vline = [x, settings.position[1] + cell_size, x, y2];
            cell_edge.draw(vline, &c.draw_state, c.transform, g);

            let hline = [settings.position[0] + cell_size, y, x2, y];
            cell_edge.draw(hline, &c.draw_state, c.transform, g);
        }

        // draw board edge
        Rectangle::new_border(settings.board_edge_color, settings.board_edge_radius)
            .draw(board_rect, &c.draw_state, c.transform, g);

        // draw insert guides
        let insert_guide = Polygon::new(settings.insert_guide_color);
        for i in 0..3 {
            let north = settings.position[1];
            let north_ish = north + cell_size;
            let west = settings.position[0];
            let west_ish = west + cell_size;
            let east = west + settings.size;
            let east_ish = east - cell_size;
            let south = north + settings.size;
            let south_ish = south - cell_size;

            let early_offset = (i as f64 + 1.0) * 2.0 / (board::SIZE as f64 + 2.0) * settings.size;
            let mid_offset = early_offset + cell_size / 2.0;
            let late_offset = early_offset + cell_size;

            let early_x = settings.position[0] + early_offset;
            let mid_x = settings.position[0] + mid_offset;
            let late_x = settings.position[0] + late_offset;

            let early_y = settings.position[1] + early_offset;
            let mid_y = settings.position[1] + mid_offset;
            let late_y = settings.position[1] + late_offset;

            // draw north edge
            insert_guide.draw(&[[early_x, north], [mid_x, north_ish], [late_x, north]], &c.draw_state, c.transform, g);
            // draw east edge
            insert_guide.draw(&[[east, early_y], [east_ish, mid_y], [east, late_y]], &c.draw_state, c.transform, g);
            // draw south edge
            insert_guide.draw(&[[early_x, south], [mid_x, south_ish], [late_x, south]], &c.draw_state, c.transform, g);
            // draw west edge
            insert_guide.draw(&[[west, early_y], [west_ish, mid_y], [west, late_y]], &c.draw_state, c.transform, g);
        }
    }
}
