use graphics::types::Color;

macro_rules! color {
    ($r: expr, $g: expr, $b: expr) => {[($r as f32) / 255.0, ($g as f32) / 255.0, ($b as f32) / 255.0, 1.0]};
}

pub const DARK: Color = color!(0x30, 0x29, 0x2F);
pub const LIGHT: Color = color!(0x82, 0xAE, 0xB1);
pub const PURPLE: Color = color!(0x5F, 0x5A, 0xA2);
pub const BLUE: Color = color!(0x35, 0x56, 0x91);
pub const TEAL: Color = color!(0x66, 0x85, 0x86);
