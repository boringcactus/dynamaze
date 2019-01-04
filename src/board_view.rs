//! Board view

use std::ops;

use graphics::types::Color;
use graphics::{Context, Graphics};
use graphics::character::CharacterCache;

use crate::BoardController;
use crate::Direction;

#[derive(Clone)]
struct Extents {
    north: f64,
    south: f64,
    east: f64,
    west: f64,
}

impl ops::Sub<f64> for Extents {
    type Output = Extents;

    fn sub(self, rhs: f64) -> Extents {
        Extents {
            north: self.north + rhs,
            south: self.south - rhs,
            east: self.east - rhs,
            west: self.west + rhs,
        }
    }
}

/// Stores board view settings
pub struct BoardViewSettings {
    /// Position from top left corner
    pub position: [f64; 2],
    /// Width of board
    pub width: f64,
    /// Height of board
    pub height: f64,
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
    /// Tile wall width as percentage of tile size
    pub wall_width: f64,
    /// Insert guide color
    pub insert_guide_color: Color,
}

impl BoardViewSettings {
    /// Creates new board view settings
    pub fn new() -> BoardViewSettings {
        BoardViewSettings {
            position: [0.0; 2],
            width: 640.0,
            height: 480.0,
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
            wall_width: 0.3,
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

    /// Gets the size of an individual tile and the x and y padding values
    fn tile_padding(&self, controller: &BoardController) -> (f64, f64, f64) {
        let ref settings = self.settings;
        let cell_max_height = settings.height / (controller.board.height() as f64 + 2.0);
        let cell_max_width = settings.width / (controller.board.width() as f64 + 2.0);
        if cell_max_height < cell_max_width {
            let space_used_x = cell_max_height * (controller.board.width() as f64 + 2.0);
            (cell_max_height, (settings.width - space_used_x) / 2.0, 0.0)
        } else {
            let space_used_y = cell_max_width * (controller.board.height() as f64 + 2.0);
            (cell_max_width, 0.0, (settings.height - space_used_y) / 2.0)
        }
    }

    /// Gets the extents of the game and board
    fn extents(&self, controller: &BoardController) -> (Extents, Extents) {
        let ref settings = self.settings;
        let (cell_size, x_padding, y_padding) = self.tile_padding(controller);
        let game = Extents {
            west: settings.position[0] + x_padding,
            east: settings.position[0] + settings.width - x_padding,
            north: settings.position[1] + y_padding,
            south: settings.position[1] + settings.height - y_padding,
        };
        let board = game.clone() - cell_size;
        (game, board)
    }

    /// Draw board
    pub fn draw<G: Graphics, C>(
        &self, controller: &BoardController,
        glyphs: &mut C,
        c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::{Line, Rectangle};

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        let ref settings = self.settings;
        let (cell_size, _, _) = self.tile_padding(controller);

        // draw board
        let (game, board) = self.extents(controller);
        let board_width = cell_size * board_tile_width as f64;
        let board_height = cell_size * board_tile_height as f64;
        let board_rect = [board.west, board.north, board_width, board_height];
        Rectangle::new(settings.background_color)
            .draw(board_rect, &c.draw_state, c.transform, g);

        // draw the tiles
        self.draw_tiles(controller, glyphs, c, g);

        // draw tile edges
        let cell_edge = Line::new(settings.cell_edge_color, settings.cell_edge_radius);
        for i in 0..board_tile_width {
            let x = board.west + i as f64 * cell_size;
            let vline = [x, board.north, x, board.south];
            cell_edge.draw(vline, &c.draw_state, c.transform, g);
        }
        for j in 0..board_tile_height {
            let y = game.north + (j + 1) as f64 * cell_size;
            let hline = [board.west, y, board.east, y];
            cell_edge.draw(hline, &c.draw_state, c.transform, g);
        }

        // draw board edge
        Rectangle::new_border(settings.board_edge_color, settings.board_edge_radius)
            .draw(board_rect, &c.draw_state, c.transform, g);

        // draw insert guides
        self.draw_insert_guides(controller, glyphs, c, g);
    }

    fn draw_tiles<G: Graphics, C>(
        &self, controller: &BoardController,
        _glyphs: &mut C,
        c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::Rectangle;

        let ref settings = self.settings;

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();
        let (cell_size, _, _) = self.tile_padding(controller);
        let (_, board) = self.extents(controller);
        let wall_width = cell_size * settings.wall_width;

        let wall_rect = Rectangle::new(settings.wall_color);
        for j in 0..board_tile_height {
            for i in 0..board_tile_width {
                let north = board.north + j as f64 * cell_size;
                let south = north + cell_size;
                let south_ish = south - wall_width;
                let west = board.west + i as f64 * cell_size;
                let east = west + cell_size;
                let east_ish = east - wall_width;

                wall_rect.draw([west, north, wall_width, wall_width], &c.draw_state, c.transform, g);
                wall_rect.draw([east_ish, north, wall_width, wall_width], &c.draw_state, c.transform, g);
                wall_rect.draw([west, south_ish, wall_width, wall_width], &c.draw_state, c.transform, g);
                wall_rect.draw([east_ish, south_ish, wall_width, wall_width], &c.draw_state, c.transform, g);

                let walled_directions = controller.board.get([i, j]).walls();

                for d in walled_directions {
                    let rect = match d {
                        Direction::North => [west, north, cell_size, wall_width],
                        Direction::South => [west, south_ish, cell_size, wall_width],
                        Direction::East => [east_ish, north, wall_width, cell_size],
                        Direction::West => [west, north, wall_width, cell_size],
                    };
                    wall_rect.draw(rect, &c.draw_state, c.transform, g);
                }
            }
        }
    }

    fn draw_insert_guides<G: Graphics, C>(
        &self, controller: &BoardController,
        glyphs: &mut C,
        c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::Polygon;

        let ref settings = self.settings;

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();
        let (cell_size, _, _) = self.tile_padding(controller);
        let (game, board) = self.extents(controller);
        let wall_width = cell_size * settings.wall_width;

        let insert_guide = Polygon::new(settings.insert_guide_color);
        for i in 0..(board_tile_width / 2) {
            let north = game.north + wall_width;
            let north_ish = board.north - wall_width;
            let south = game.south - wall_width;
            let south_ish = board.south + wall_width;

            let offset = (2 * i + 1) as f64 * cell_size;
            let early_offset = offset + wall_width;
            let mid_offset = offset + cell_size / 2.0;
            let late_offset = offset + cell_size - wall_width;

            let early_x = board.west + early_offset;
            let mid_x = board.west + mid_offset;
            let late_x = board.west + late_offset;

            // draw north edge
            insert_guide.draw(&[[early_x, north], [mid_x, north_ish], [late_x, north]], &c.draw_state, c.transform, g);
            // draw south edge
            insert_guide.draw(&[[early_x, south], [mid_x, south_ish], [late_x, south]], &c.draw_state, c.transform, g);
        }
        for j in 0..(board_tile_height / 2) {
            let west = game.west + wall_width;
            let west_ish = board.west - wall_width;
            let east = game.east - wall_width;
            let east_ish = board.east + wall_width;

            let offset = (2 * j + 1) as f64 * cell_size;
            let early_offset = offset + wall_width;
            let mid_offset = offset + cell_size / 2.0;
            let late_offset = offset + cell_size - wall_width;

            let early_y = board.north + early_offset;
            let mid_y = board.north + mid_offset;
            let late_y = board.north + late_offset;

            // draw east edge
            insert_guide.draw(&[[east, early_y], [east_ish, mid_y], [east, late_y]], &c.draw_state, c.transform, g);
            // draw west edge
            insert_guide.draw(&[[west, early_y], [west_ish, mid_y], [west, late_y]], &c.draw_state, c.transform, g);
        }
    }
}
