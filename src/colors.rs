#![allow(clippy::unnecessary_cast)]

use quicksilver::graphics::{Background, Color as Qcolor};
use rand::distributions::{Distribution, Standard};
use rand::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Color(pub f32, pub f32, pub f32);

impl Into<Qcolor> for Color {
    fn into(self) -> Qcolor {
        Qcolor { r: self.0, g: self.1, b: self.2, a: 1.0 }
    }
}

impl Into<Background<'_>> for Color {
    fn into(self) -> Background<'static> {
        Background::Col(self.into())
    }
}

impl Distribution<Color> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Color {
        let r = rng.gen_range(0.0, 1.0);
        let g = rng.gen_range(0.0, 1.0);
        let b = rng.gen_range(0.0, 1.0);
        Color(r, g, b)
    }
}

macro_rules! color {
    ($r: expr, $g: expr, $b: expr) => {Color(($r as f32) / 255.0, ($g as f32) / 255.0, ($b as f32) / 255.0)};
}

pub const DARK: Color = color!(0x30, 0x29, 0x2F);
pub const LIGHT: Color = color!(0x82, 0xAE, 0xB1);
pub const PURPLE: Color = color!(0x5F, 0x5A, 0xA2);
pub const BLUE: Color = color!(0x35, 0x56, 0x91);
pub const TEAL: Color = color!(0x66, 0x85, 0x86);
