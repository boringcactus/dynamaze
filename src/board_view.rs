//! Board view

use std::cmp;
use std::ops;

use graphics::{Context, Graphics};
use graphics::character::CharacterCache;
use graphics::types::{Color, Rectangle};

use crate::{BoardController, colors, Direction, PlayerID, Tile};
use crate::board_controller::TurnState;

#[derive(Clone, Debug)]
struct Extents {
    north: f64,
    south: f64,
    east: f64,
    west: f64,
}

impl Extents {
    #[allow(dead_code)]
    fn center(&self) -> [f64; 2] {
        [(self.west + self.east) / 2.0, (self.north + self.south) / 2.0]
    }
}

impl ops::Sub<f64> for Extents {
    type Output = Extents;

    #[allow(clippy::suspicious_arithmetic_impl)]
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

impl Into<Rectangle> for Extents {
    fn into(self) -> Rectangle {
        let x = self.west;
        let y = self.north;
        let w = self.east - self.west;
        let h = self.south - self.north;
        [x, y, w, h]
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
    /// Reachable background color
    pub reachable_background_color: Color,
    /// Border color
    pub border_color: Color,
    /// Edge color around the whole board
    pub board_edge_color: Color,
    /// Edge color between cells
    pub cell_edge_color: Color,
    /// Edge radius around the whole board
    pub board_edge_radius: f64,
    /// Edge radius between cells
    pub cell_edge_radius: f64,
    /// Text color
    pub text_color: Color,
    /// Wall color
    pub wall_color: Color,
    /// Tile wall width as percentage of tile size
    pub wall_width: f64,
    /// Insert guide color
    pub insert_guide_color: Color,
    /// UI margin size, south pane
    pub ui_margin_south: f64,
    /// UI margin size, east pane
    pub ui_margin_east: f64,
    /// Font size
    pub font_size: u32,
}

impl BoardViewSettings {
    /// Creates new board view settings
    pub fn new(size: [f64; 2]) -> BoardViewSettings {
        let [width, height] = size;
        BoardViewSettings {
            position: [0.0; 2],
            width,
            height,
            background_color: colors::TEAL.into(),
            reachable_background_color: colors::LIGHT.into(),
            border_color: colors::DARK.into(),
            board_edge_color: colors::DARK.into(),
            cell_edge_color: colors::DARK.into(),
            board_edge_radius: 3.0,
            cell_edge_radius: 1.0,
            text_color: colors::DARK.into(),
            wall_color: colors::BLUE.into(),
            wall_width: 0.3,
            insert_guide_color: colors::PURPLE.into(),
            ui_margin_south: 100.0,
            ui_margin_east: 300.0,
            font_size: 25,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum DrawMode {
    All,
    OnlySelf,
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
        let settings = &self.settings;
        let cell_max_height = (settings.height - settings.ui_margin_south) / (controller.board.height() as f64 + 2.0);
        let cell_max_width = (settings.width - settings.ui_margin_east) / (controller.board.width() as f64 + 2.0);
        if cell_max_height < cell_max_width {
            let space_used_x = cell_max_height * (controller.board.width() as f64 + 2.0) + settings.ui_margin_east;
            (cell_max_height, (settings.width - space_used_x) / 2.0, 0.0)
        } else {
            let space_used_y = cell_max_width * (controller.board.height() as f64 + 2.0) + settings.ui_margin_south;
            (cell_max_width, 0.0, (settings.height - space_used_y) / 2.0)
        }
    }

    /// Gets the extents of the game and board
    fn game_extents(&self, controller: &BoardController) -> (Extents, Extents) {
        let settings = &self.settings;
        let (cell_size, x_padding, y_padding) = self.tile_padding(controller);
        let game = Extents {
            west: settings.position[0] + x_padding,
            east: settings.position[0] + settings.width - x_padding - settings.ui_margin_east,
            north: settings.position[1] + y_padding,
            south: settings.position[1] + settings.height - y_padding - settings.ui_margin_south,
        };
        let board = game.clone() - cell_size;
        (game, board)
    }

    /// Gets the extents of the south and east UI panels
    fn ui_extents(&self) -> (Extents, Extents) {
        let settings = &self.settings;
        let global = Extents {
            north: settings.position[1],
            south: settings.position[1] + settings.height,
            west: settings.position[0],
            east: settings.position[0] + settings.width,
        };
        let south = Extents {
            north: global.south - settings.ui_margin_south,
            south: global.south,
            west: global.west,
            east: global.east,
        };
        let east = Extents {
            north: global.north,
            south: south.north,
            west: global.east - settings.ui_margin_east,
            east: global.east,
        };
        (south, east)
    }

    /// Draw board
    pub fn draw<G: Graphics, C>(
        &self, controller: &BoardController, local_id: PlayerID,
        glyphs: &mut C, c: &Context, g: &mut G,
    ) where C: CharacterCache<Texture=G::Texture> {
        use graphics::{Line, Rectangle};

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        let settings = &self.settings;
        let (cell_size, _, _) = self.tile_padding(controller);

        // draw board
        let (game, board) = self.game_extents(controller);
        let board_width = cell_size * board_tile_width as f64;
        let board_height = cell_size * board_tile_height as f64;
        let board_rect = [board.west, board.north, board_width, board_height];

        // draw the tiles
        self.draw_tiles(controller, local_id, glyphs, c, g);

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
        self.draw_insert_guides(controller, local_id, glyphs, c, g);

        // draw player tokens
        self.draw_player_tokens(DrawMode::All, controller, local_id, glyphs, c, g);

        // draw own token on top of others
        self.draw_player_tokens(DrawMode::OnlySelf, controller, local_id, glyphs, c, g);

        // draw UI
        self.draw_ui(controller, local_id, glyphs, c, g);
    }

    fn tile_extents(&self, controller: &BoardController, row: usize, col: usize) -> Extents {
        let (cell_size, _, _) = self.tile_padding(controller);
        let (_, board) = self.game_extents(controller);
        let north = board.north + row as f64 * cell_size;
        let south = north + cell_size;
        let west = board.west + col as f64 * cell_size;
        let east = west + cell_size;
        Extents {
            north,
            south,
            east,
            west,
        }
    }

    /// Checks if a given position is within a tile, and returns that tile's (row, col)
    pub fn in_tile(&self, pos: &[f64; 2], controller: &BoardController) -> Option<(usize, usize)> {
        // TODO don't do this dumb thing

        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        for j in 0..board_tile_height {
            for i in 0..board_tile_width {
                let cell = self.tile_extents(controller, j, i);
                if pos < &cell {
                    return Some((j, i));
                }
            }
        }
        None
    }

    fn draw_tiles<G: Graphics, C>(
        &self, controller: &BoardController, local_id: PlayerID,
        _glyphs: &mut C, c: &Context, g: &mut G,
    ) where C: CharacterCache<Texture=G::Texture> {
        let board_tile_width = controller.board.width();
        let board_tile_height = controller.board.height();

        let current_player_pos = controller.board.player_pos(local_id);
        let reachable = controller.board.reachable_coords(current_player_pos);

        for j in 0..board_tile_height {
            for i in 0..board_tile_width {
                let cell = self.tile_extents(controller, j, i);
                let color = if reachable.contains(&(j, i)) {
                    self.settings.reachable_background_color
                } else {
                    self.settings.background_color
                };
                let is_highlighted = controller.highlighted_tile == (j, i);
                self.draw_tile(controller.board.get([i, j]), cell, color, is_highlighted, controller, _glyphs, c, g);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_tile<G: Graphics, C>(
        &self, tile: &Tile, outer: Extents, background_color: Color, draw_border: bool,
        controller: &BoardController, _glyphs: &mut C, c: &Context, g: &mut G,
    ) where C: CharacterCache<Texture=G::Texture> {
        use graphics::{Rectangle, Polygon};

        let settings = &self.settings;

        let (cell_size, _, _) = self.tile_padding(controller);
        let wall_width = cell_size * settings.wall_width;
        let inner = outer.clone() - wall_width;

        Rectangle::new(background_color)
            .draw([outer.west, outer.north, cell_size, cell_size], &c.draw_state, c.transform, g);

        // TODO make this less goofy
        if let Some(whose_target) = tile.whose_target {
            let color = controller.players[&whose_target].color;

            // TODO tilt based on something so less reliant on color
            // TODO animate stripes for current player

            let markers = (0..=6).map(|x| cell_size * (f64::from(x) / 6.0));
            let Extents { north, east, south, west } = outer;
            // x increments
            let x = markers.clone().map(|x| outer.west + x).collect::<Vec<_>>();
            // y increments
            let y = markers.clone().map(|y| outer.north + y).collect::<Vec<_>>();

            let poly = Polygon::new(color.into());
            let stripes = [
                [[x[1], north], [west, y[1]], [west, y[2]], [x[2], north]],
                [[x[3], north], [west, y[3]], [west, y[4]], [x[4], north]],
                [[x[5], north], [west, y[5]], [west, y[6]], [x[6], north]],
                [[east, y[1]], [x[1], south], [x[2], south], [east, y[2]]],
                [[east, y[3]], [x[3], south], [x[4], south], [east, y[4]]],
                [[east, y[5]], [x[5], south], [east, south], [east, south]]
            ];
            for stripe in &stripes {
                poly.draw(stripe, &c.draw_state, c.transform, g);
            }
        }

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

        if draw_border {
            let border_width = wall_width / 3.0;
            let inner = outer.clone() - border_width;
            let border_rect = Rectangle::new(settings.text_color);
            border_rect.draw([outer.west, outer.north, cell_size, border_width], &c.draw_state, c.transform, g);
            border_rect.draw([outer.west, inner.south, cell_size, border_width], &c.draw_state, c.transform, g);
            border_rect.draw([inner.east, outer.north, border_width, cell_size], &c.draw_state, c.transform, g);
            border_rect.draw([outer.west, outer.north, border_width, cell_size], &c.draw_state, c.transform, g);
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
        &self, controller: &BoardController, _local_id: PlayerID,
        _glyphs: &mut C, c: &Context, g: &mut G,
    ) where C: CharacterCache<Texture=G::Texture> {
        use graphics::Polygon;

        let settings = &self.settings;

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

    fn draw_player_tokens<G: Graphics, C>(
        &self, mode: DrawMode, controller: &BoardController, local_id: PlayerID,
        _glyphs: &mut C, c: &Context, g: &mut G,
    ) where C: CharacterCache<Texture=G::Texture> {
        use graphics::Ellipse;

        let settings = &self.settings;

        let (cell_size, _, _) = self.tile_padding(controller);
        let wall_width = cell_size * settings.wall_width;

        for token in controller.board.player_tokens.values() {
            let (row, col) = token.position;
            let player = match controller.players.get(&token.player_id) {
                Some(x) => x,
                None => continue,
            };
            let tile = self.tile_extents(controller, row, col);
            let token_rect = tile - wall_width;

            let should = mode == DrawMode::All || token.player_id == local_id;
            if should {
                let token_ellipse = Ellipse::new(player.color.into());
                token_ellipse.draw(token_rect.clone(), &c.draw_state, c.transform, g);
                if token.player_id == local_id {
                    let token_core = Ellipse::new([0.0, 0.0, 0.0, 1.0]);
                    token_core.draw(token_rect - wall_width / 2.0, &c.draw_state, c.transform, g);
                }
            }
        }
    }

    fn draw_ui<G: Graphics, C>(
        &self, controller: &BoardController, local_id: PlayerID,
        glyphs: &mut C, c: &Context, g: &mut G,
    ) where C: CharacterCache<Texture=G::Texture> {
        use graphics::Transformed;
        // draw loose tile
        {
            let cell = self.loose_tile_extents(controller);
            self.draw_tile(&controller.board.loose_tile, cell, self.settings.background_color, false, controller, glyphs, c, g);
        }

        // draw player target
        {
            let (cell_size, _, _) = self.tile_padding(controller);
            let (south_panel, _) = self.ui_extents();
            let my_turn = local_id == controller.active_player_id();
            let turn_status = if my_turn { "is" } else { "is not" };
            let text = format!("It {} your turn", turn_status);
            let west = south_panel.west + cell_size * 1.5;
            let north = south_panel.north + 20.0;
            let transform = c.transform.trans(west, north);
            graphics::text(self.settings.text_color, 20, &text, glyphs, transform, g).ok().expect("Failed to draw text");
            if my_turn {
                let text = match controller.turn_state {
                    TurnState::InsertTile => "Right-click at a triangle to rotate, left-click to insert",
                    TurnState::MoveToken => "Click on a reachable tile, or yourself to not move",
                };
                let north = north + 30.0;
                let transform = c.transform.trans(west, north);
                graphics::text(self.settings.text_color, 20, &text, glyphs, transform, g).ok().expect("Failed to draw text");
            }
        }

        // draw player list
        {
            let (_, east_panel) = self.ui_extents();

            let west = east_panel.west;
            let mut north = east_panel.north + 20.0;
            for player_id in &controller.turn_order {
                let player = &controller.players[player_id];
                let token = &controller.board.player_tokens[player_id];

                let transform = c.transform.trans(west, north);
                graphics::text(self.settings.text_color, 15, &player.name, glyphs, transform, g).ok().expect("Failed to draw text");
                north += 10.0;

                graphics::ellipse(player.color.into(), [west, north, 15.0, 15.0], c.transform, g);
                let text = format!("score: {}", token.score);
                let transform = c.transform.trans(west + 20.0, north + 10.0);
                graphics::text(self.settings.text_color, 15, &text, glyphs, transform, g).ok().expect("Failed to draw text");
                north += 40.0;
            }
        }
    }
}
