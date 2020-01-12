#![allow(clippy::unnecessary_cast)]

use rand::distributions::{Distribution, Standard};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Color(pub f32, pub f32, pub f32);

impl Color {
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", (self.0 * 255.0) as u8, (self.1 * 255.0) as u8, (self.2 * 255.0) as u8)
    }
}

impl Into<JsValue> for Color {
    fn into(self) -> JsValue {
        JsValue::from_str(&format!(
            "rgb({}%, {}%, {}%)",
            self.0 * 100.0,
            self.1 * 100.0,
            self.2 * 100.0
        ))
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
    ($r: expr, $g: expr, $b: expr) => {
        Color(
            ($r as f32) / 255.0,
            ($g as f32) / 255.0,
            ($b as f32) / 255.0,
        )
    };
}

pub const DARK: Color = color!(0x30, 0x29, 0x2F);
pub const LIGHT: Color = color!(0x82, 0xAE, 0xB1);
pub const PURPLE: Color = color!(0x5F, 0x5A, 0xA2);
pub const BLUE: Color = color!(0x35, 0x56, 0x91);
pub const TEAL: Color = color!(0x66, 0x85, 0x86);
