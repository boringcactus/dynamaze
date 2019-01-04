//! Board view

use std::cmp;
use std::ops;

use graphics::types::Color;
use graphics::{Context, Graphics};
use graphics::character::CharacterCache;

use crate::BoardController;
use crate::Direction;
use crate::Tile;

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

impl PartialEq<Extents> for [f64; 2] {
    fn eq(&self, other: &Extents) -> bool {
        self.partial_cmp(other) == Some(cmp::Ordering::Equal)
    }
}

impl PartialOrd<Extents> for [f64; 2] {
    fn partial_cmp(&self, other: &Extents) -> Option<cmp::Ordering> {
        use std::cmp::Ordering::*;
        let [x, y] = self;
        let result = match (x.partial_cmp(&other.west), x.partial_cmp(&other.east),
            y.partial_cmp(&other.north), y.partial_cmp(&other.south)) {
            // too far west
            (Some(Less), _, _, _) => Greater,
            // too far east
            (_, Some(Greater), _, _) => Greater,
            // too far north
            (_, _, Some(Less), _) => Greater,
            // too far south
            (_, _, _, Some(Greater)) => Greater,
            // entirely within
            (Some(Greater), Some(Less), Some(Greater), Some(Less)) => Less,
            // on west edge
            (Some(Equal), _, _, _) => Equal,
            // on east edge
            (_, Some(Equal), _, _) => Equal,
            // on north edge
            (_, _, Some(Equal), _) => Equal,
            // on south edge
            (_, _, _, Some(Equal)) => Equal,
            // this really shouldn't be possible, and the rust compiler warns about an unreachable pattern!
            // thanks, rust!
            // (Some(_), Some(_), Some(_), Some(_)) => panic!("Implausible bounds check for point in extents"),
            // something is NaN or otherwise fucky
            _ => return None
        };
        Some(result)
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
    /// UI margin size
    pub ui_margin: f64,
}

impl BoardViewSettings {
    /// Creates new board view settings
    pub fn new(size: [f64; 2]) -> BoardViewSettings {
        let [width, height] = size;
        BoardViewSettings {
            position: [0.0; 2],
            width,
            height,
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
            ui_margin: 100.0,
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
        let cell_max_height = (settings.height - settings.ui_margin) / (controller.board.height() as f64 + 2.0);
        let cell_max_width = (settings.width - settings.ui_margin) / (controller.board.width() as f64 + 2.0);
        if cell_max_height < cell_max_width {
            let space_used_x = cell_max_height * (controller.board.width() as f64 + 2.0) + settings.ui_margin;
            (cell_max_height, (settings.width - space_used_x) / 2.0, 0.0)
        } else {
            let space_used_y = cell_max_width * (controller.board.height() as f64 + 2.0) + settings.ui_margin;
            (cell_max_width, 0.0, (settings.height - space_used_y) / 2.0)
        }
    }

    /// Gets the extents of the game and board
    fn game_extents(&self, controller: &BoardController) -> (Extents, Extents) {
        let ref settings = self.settings;
        let (cell_size, x_padding, y_padding) = self.tile_padding(controller);
        let game = Extents {
            west: settings.position[0] + x_padding,
            east: settings.position[0] + settings.width - x_padding - settings.ui_margin,
            north: settings.position[1] + y_padding,
            south: settings.position[1] + settings.height - y_padding - settings.ui_margin,
        };
        let board = game.clone() - cell_size;
        (game, board)
    }

    /// Gets the extents of the south and east UI panels
    fn ui_extents(&self) -> (Extents, Extents) {
        let ref settings = self.settings;
        let global = Extents {
            north: settings.position[1],
            south: settings.position[1] + settings.height,
            west: settings.position[0],
            east: settings.position[0] + settings.width,
        };
        let south = Extents {
            north: global.south - settings.ui_margin,
            south: global.south,
            west: global.west,
            east: global.east,
        };
        let east = Extents {
            north: global.north,
            south: south.north,
            west: global.east - settings.ui_margin,
            east: global.east,
        };
        (south, east)
    }

    /// Draw board
    pub fn draw<G: Graphics, C>(
        &self, controller: &BoardController,
        glyphs: &mut C, c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::{Line, Rectangle};

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        let ref settings = self.settings;
        let (cell_size, _, _) = self.tile_padding(controller);

        // draw board
        let (game, board) = self.game_extents(controller);
        let board_width = cell_size * board_tile_width as f64;
        let board_height = cell_size * board_tile_height as f64;
        let board_rect = [board.west, board.north, board_width, board_height];

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

        // draw UI
        self.draw_ui(controller, glyphs, c, g);
    }

    fn draw_tiles<G: Graphics, C>(
        &self, controller: &BoardController,
        _glyphs: &mut C, c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();
        let (cell_size, _, _) = self.tile_padding(controller);
        let (_, board) = self.game_extents(controller);

        for j in 0..board_tile_height {
            for i in 0..board_tile_width {
                let north = board.north + j as f64 * cell_size;
                let south = north + cell_size;
                let west = board.west + i as f64 * cell_size;
                let east = west + cell_size;
                let cell = Extents {
                    north,
                    south,
                    east,
                    west
                };
                self.draw_tile(controller, controller.board.get([i, j]), &cell, _glyphs, c, g);
            }
        }
    }

    fn draw_tile<G: Graphics, C>(
        &self, controller: &BoardController, tile: &Tile, outer: &Extents,
        _glyphs: &mut C, c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::Rectangle;
        let ref settings = self.settings;

        let (cell_size, _, _) = self.tile_padding(controller);
        let wall_width = cell_size * settings.wall_width;
        let inner = outer.clone() - wall_width;

        Rectangle::new(settings.background_color)
            .draw([outer.west, outer.north, cell_size, cell_size], &c.draw_state, c.transform, g);

        let wall_rect = Rectangle::new(settings.wall_color);
        wall_rect.draw([outer.west, outer.north, wall_width, wall_width], &c.draw_state, c.transform, g);
        wall_rect.draw([inner.east, outer.north, wall_width, wall_width], &c.draw_state, c.transform, g);
        wall_rect.draw([outer.west, inner.south, wall_width, wall_width], &c.draw_state, c.transform, g);
        wall_rect.draw([inner.east, inner.south, wall_width, wall_width], &c.draw_state, c.transform, g);
        let walled_directions = tile.walls();
        for d in walled_directions {
            let rect = match d {
                Direction::North => [outer.west, outer.north, cell_size, wall_width],
                Direction::South => [outer.west, inner.south, cell_size, wall_width],
                Direction::East => [inner.east, outer.north, wall_width, cell_size],
                Direction::West => [outer.west, outer.north, wall_width, cell_size],
            };
            wall_rect.draw(rect, &c.draw_state, c.transform, g);
        }
    }

    fn insert_guides(&self, controller: &BoardController) -> Vec<(Direction, Vec<Extents>)> {
        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();
        let (cell_size, _, _) = self.tile_padding(controller);
        let (game, board) = self.game_extents(controller);

        let mut result = vec![];

        let mut north = vec![];
        let mut south = vec![];
        for i in 0..(board_tile_width / 2) {
            let west = board.west + (2 * i + 1) as f64 * cell_size;
            let east = west + cell_size;

            let north_extents = Extents {
                north: game.north,
                south: board.north,
                west,
                east,
            };
            north.push(north_extents);

            let south_extents = Extents {
                north: board.south,
                south: game.south,
                east,
                west,
            };
            south.push(south_extents);
        }
        result.push((Direction::North, north));
        result.push((Direction::South, south));
        let mut east = vec![];
        let mut west = vec![];
        for j in 0..(board_tile_height / 2) {
            let north = board.north + (2 * j + 1) as f64 * cell_size;
            let south = north + cell_size;

            let west_extents = Extents {
                north,
                south,
                west: game.west,
                east: board.west,
            };
            west.push(west_extents);

            let east_extents = Extents {
                north,
                south,
                west: board.east,
                east: game.east,
            };
            east.push(east_extents);
        }
        result.push((Direction::East, east));
        result.push((Direction::West, west));
        result
    }

    fn draw_insert_guides<G: Graphics, C>(
        &self, controller: &BoardController,
        _glyphs: &mut C, c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        use graphics::Polygon;

        let ref settings = self.settings;

        let (cell_size, _, _) = self.tile_padding(controller);
        let wall_width = cell_size * settings.wall_width;

        let insert_guide = Polygon::new(settings.insert_guide_color);
        for (dir, guides) in self.insert_guides(controller) {
            for guide in guides {
                let guide = guide - wall_width;
                let mid_x = (guide.east + guide.west) / 2.0;
                let mid_y = (guide.north + guide.south) / 2.0;
                let rect = match dir {
                    Direction::North => [[guide.west, guide.north], [mid_x, guide.south], [guide.east, guide.north]],
                    Direction::South => [[guide.west, guide.south], [mid_x, guide.north], [guide.east, guide.south]],
                    Direction::West => [[guide.west, guide.north], [guide.east, mid_y], [guide.west, guide.south]],
                    Direction::East => [[guide.east, guide.north], [guide.west, mid_y], [guide.east, guide.south]],
                };
                insert_guide.draw(&rect, &c.draw_state, c.transform, g);
            }
        }
    }

    /// Checks if the given position is in an insert guide or not
    pub fn in_insert_guide(&self, pos: &[f64; 2], controller: &BoardController) -> Option<(Direction, usize)> {
        for (dir, guides) in self.insert_guides(controller) {
            for (i, guide) in guides.into_iter().enumerate() {
                if pos < &guide {
                    return Some((dir, i));
                }
            }
        }
        None
    }

    fn loose_tile_extents(&self, controller: &BoardController) -> Extents {
        if let Some((target_dir, idx)) = controller.board.loose_tile_position {
            for (dir, guides) in self.insert_guides(controller) {
                if dir == target_dir {
                    return guides[idx].clone();
                }
            }
        }
        let (cell_size, _, _) = self.tile_padding(controller);
        let (south_panel, _) = self.ui_extents();
        Extents {
            north: south_panel.north,
            south: south_panel.north + cell_size,
            west: south_panel.west,
            east: south_panel.west + cell_size,
        }
    }

    /// Check if the given position is within the loose tile area
    pub fn in_loose_tile(&self, pos: &[f64; 2], controller: &BoardController) -> bool {
        let cell = self.loose_tile_extents(controller);
        pos < &cell
    }

    fn draw_ui<G: Graphics, C>(
        &self, controller: &BoardController,
        _glyphs: &mut C, c: &Context, g: &mut G
    ) where C: CharacterCache<Texture = G::Texture> {
        // draw loose tile
        {
            let cell = self.loose_tile_extents(controller);
            self.draw_tile(controller, &controller.board.loose_tile, &cell, _glyphs, c, g);
        }
    }
}
