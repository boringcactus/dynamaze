//! Tile logic

use std::convert::TryFrom;
use std::f64::consts;
use std::ops;

use rand::distributions::{Distribution, Standard};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

use crate::PlayerID;

/// Cardinal directions
#[derive(Eq, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Direction {
    /// Up
    North,
    /// Down
    South,
    /// Right
    East,
    /// Left
    West,
}

impl Direction {
    /// Gets the radian rotation of this direction
    pub fn rad(self) -> f64 {
        match self {
            Direction::North => 0.0,
            Direction::East => consts::PI / 2.0,
            Direction::South => consts::PI,
            Direction::West => consts::PI * 3.0 / 2.0,
        }
    }

    /// Gets a list of all Directions
    pub fn all() -> &'static [Direction] {
        &[Direction::North, Direction::East, Direction::South, Direction::West]
    }
}

impl ops::Add<Direction> for (usize, usize) {
    type Output = (usize, usize);

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn add(self, rhs: Direction) -> (usize, usize) {
        let (j, i) = self;
        match rhs {
            Direction::North => (j - 1, i),
            Direction::South => (j + 1, i),
            Direction::East => (j, i + 1),
            Direction::West => (j, i - 1),
        }
    }
}

impl ops::Mul<Direction> for Direction {
    type Output = Direction;

    fn mul(self, rhs: Direction) -> Direction {
        use self::Direction::*;
        match (self, rhs) {
            (North, a) => a,
            (East, North) => East,
            (East, East) => South,
            (East, South) => West,
            (East, West) => North,
            (South, North) => South,
            (South, East) => West,
            (South, South) => North,
            (South, West) => East,
            (West, North) => West,
            (West, East) => North,
            (West, South) => East,
            (West, West) => South,
        }
    }
}

impl ops::MulAssign for Direction {
    fn mul_assign(&mut self, rhs: Direction) {
        *self = *self * rhs;
    }
}

impl ops::Mul<Direction> for [f32; 2] {
    type Output = [f32; 2];

    fn mul(self, rhs: Direction) -> Self::Output {
        let [x, y] = self;
        match rhs {
            Direction::North => [x, y],
            Direction::South => [-x, -y],
            Direction::East => [-y, x],
            Direction::West => [y, -x],
        }
    }
}

impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0, 4) {
            0 => Direction::North,
            1 => Direction::South,
            2 => Direction::East,
            3 => Direction::West,
            _ => panic!("Invalid direction generated")
        }
    }
}

/// Tile shapes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Shape {
    /// Two connections, 90 degree angle (canonically North / East)
    L,
    /// Two connections, 180 degree angle (canonically North / South)
    I,
    /// Three connections (canonically North / East / South)
    T,
}

impl Shape {
    fn paths(&self) -> Vec<Direction> {
        match *self {
            Shape::L => vec![Direction::North, Direction::East],
            Shape::I => vec![Direction::North, Direction::South],
            Shape::T => vec![Direction::North, Direction::East, Direction::South],
        }
    }
    fn walls(&self) -> Vec<Direction> {
        match *self {
            Shape::L => vec![Direction::South, Direction::West],
            Shape::I => vec![Direction::East, Direction::West],
            Shape::T => vec![Direction::West],
        }
    }
}

impl Distribution<Shape> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Shape {
        match rng.gen_range(0, 3) {
            0 => Shape::L,
            1 => Shape::I,
            2 => Shape::T,
            _ => panic!("Invalid shape generated")
        }
    }
}

/// Contents of a tile
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tile {
    /// Shape of the tile
    pub shape: Shape,
    /// Orientation of the tile
    pub orientation: Direction,
    /// Player whose target is this tile
    pub whose_target: Option<PlayerID>,
}

impl Tile {
    /// Get the directions which are valid connections on this tile
    pub fn paths(&self) -> Vec<Direction> {
        self.shape.paths().iter().map(|d| *d * self.orientation).collect()
    }
    /// Get the directions which are blocked on this tile
    pub fn walls(&self) -> Vec<Direction> {
        self.shape.walls().iter().map(|d| *d * self.orientation).collect()
    }

    /// Rotate this tile clockwise
    pub fn rotate(&mut self, direction: Direction) {
        self.orientation = direction * self.orientation;
    }
}

impl Distribution<Tile> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Tile {
        let shape = rng.gen();
        let orientation = rng.gen();
        Tile {
            shape,
            orientation,
            whose_target: None,
        }
    }
}

impl TryFrom<char> for Tile {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        use Direction::*;
        use Shape::*;
        let (shape, dir) = match value {
            '│' => (I, North),
            '─' => (I, East),
            '└' => (L, North),
            '┌' => (L, East),
            '┐' => (L, South),
            '┘' => (L, West),
            '├' => (T, North),
            '┬' => (T, East),
            '┤' => (T, South),
            '┴' => (T, West),
            _ => return Err(())
        };
        Ok(Tile {
            shape,
            orientation: dir,
            whose_target: None,
        })
    }
}
