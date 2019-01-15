//! Item logic

use rand::prelude::*;
use rand::distributions::{Distribution, Standard};

/// An item a tile can have
#[allow(missing_docs)]
#[derive(Clone)]
pub enum Item {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}

const ITEM_LIST: [Item; 26] = [
    Item::A,
    Item::B,
    Item::C,
    Item::D,
    Item::E,
    Item::F,
    Item::G,
    Item::H,
    Item::I,
    Item::J,
    Item::K,
    Item::L,
    Item::M,
    Item::N,
    Item::O,
    Item::P,
    Item::Q,
    Item::R,
    Item::S,
    Item::T,
    Item::U,
    Item::V,
    Item::W,
    Item::X,
    Item::Y,
    Item::Z,
];

impl Distribution<Item> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Item {
        ITEM_LIST.choose(rng).unwrap().clone()
    }
}

impl Item {
    /// Get the character for the given item
    pub fn char(&self) -> char {
        match self {
            &Item::A => 'A',
            &Item::B => 'B',
            &Item::C => 'C',
            &Item::D => 'D',
            &Item::E => 'E',
            &Item::F => 'F',
            &Item::G => 'G',
            &Item::H => 'H',
            &Item::I => 'I',
            &Item::J => 'J',
            &Item::K => 'K',
            &Item::L => 'L',
            &Item::M => 'M',
            &Item::N => 'N',
            &Item::O => 'O',
            &Item::P => 'P',
            &Item::Q => 'Q',
            &Item::R => 'R',
            &Item::S => 'S',
            &Item::T => 'T',
            &Item::U => 'U',
            &Item::V => 'V',
            &Item::W => 'W',
            &Item::X => 'X',
            &Item::Y => 'Y',
            &Item::Z => 'Z',
        }
    }
}
