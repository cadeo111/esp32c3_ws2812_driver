use core::fmt::{
    Display,
    Formatter,
};

use super::{
    Color24bit,
    rgb_to_packed,
};

mod color {

    use super::*;

    #[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash, Default)]
    pub struct Rgb(u32);
    impl Rgb {
        pub const fn raw(r: u8, g: u8, b: u8) -> Self {
            Rgb(rgb_to_packed(r, g, b))
        }

        #[inline(always)]
        pub fn update_self(&mut self, new_state: &Self) -> &mut Rgb {
            self.0 = new_state.0;
            self
        }
    }

    impl Color24bit for Rgb {
        #[inline(always)]
        fn as_24_bit_color_u32(&self) -> u32 {
            self.0
        }

        #[inline(always)]
        fn green(&self) -> u8 {
            ((self.0 >> 16) & 0xFF) as u8
        }

        #[inline(always)]
        fn red(&self) -> u8 {
            ((self.0 >> 8) & 0xFF) as u8
        }

        #[inline(always)]
        fn blue(&self) -> u8 {
            (self.0 & 0xFF) as u8
        }
    }

    impl Display for Rgb {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            let (r, g, b) = (self.red(), self.green(), self.blue());
            write!(f, "({r}, {g}, {b})")
        }
    }
    impl From<Rgb> for u32 {
        fn from(c: Rgb) -> Self {
            c.0
        }
    }
    impl From<u32> for Rgb {
        fn from(i: u32) -> Self {
            Rgb(i)
        }
    }
}
pub use color::Rgb;

impl Rgb {
    // --- Primaries & Secondaries ---
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0);
    pub const CYAN: Self = Self::new(0, 255, 255);
    pub const MAGENTA: Self = Self::new(255, 0, 255);

    // --- Nature & Earth Tones ---
    pub const FOREST_GREEN: Self = Self::new(34, 139, 34);
    pub const SKY_BLUE: Self = Self::new(135, 206, 235);
    pub const OCEAN_BLUE: Self = Self::new(0, 105, 148);
    pub const SUNSET_ORANGE: Self = Self::new(255, 69, 0);
    pub const GOLDENROD: Self = Self::new(218, 165, 32);
    pub const AUTUMN_LEAF: Self = Self::new(139, 69, 19);
    pub const OLIVE_DRAB: Self = Self::new(107, 142, 35);
    pub const SPRING_GREEN: Self = Self::new(0, 255, 127);

    // --- Pastels & Soft Tones ---
    pub const LAVENDER: Self = Self::new(230, 190, 255);
    pub const MINT: Self = Self::new(170, 255, 195);
    pub const PEACH: Self = Self::new(255, 218, 185);
    pub const BABY_POWDER_PINK: Self = Self::new(255, 182, 193);
    pub const SOFT_ICE: Self = Self::new(200, 230, 255);
    pub const WARM_SAND: Self = Self::new(245, 222, 179);

    // --- Modern & Tech Colors ---
    pub const CYBER_PURPLE: Self = Self::new(180, 0, 255);
    pub const ELECTRIC_LIME: Self = Self::new(200, 255, 0);
    pub const DEEP_TEAL: Self = Self::new(0, 128, 128);
    pub const NEON_PINK: Self = Self::new(255, 20, 147);
    pub const SLATE: Self = Self::new(112, 128, 144);

    // --- Temperature Whites (Using raw/no gamma) ---
    pub const CANDLELIGHT: Self = Self::raw(255, 147, 41);
    pub const WARM_WHITE: Self = Self::raw(255, 244, 229);
    pub const NEUTRAL_WHITE: Self = Self::raw(255, 255, 255);
    pub const COOL_WHITE: Self = Self::raw(201, 226, 255);
    pub const MOONLIGHT: Self = Self::raw(150, 150, 200);

    // --- Your Specific Requested Defaults ---
    pub const OFF: Self = Self::raw(0, 0, 0);
    pub const WHITE: Self = Self::new(40, 40, 40);
}

impl Rgb {
    pub fn rainbow_progression(index: u16, total_steps: u16) -> Self {
        let hue = (index as f32 / total_steps as f32) * 360.0;
        let hue = hue as u32;
        Self::from_hsv(hue, 100, 70)
    }
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb::gamma_corrected(r, g, b)
    }
    /// Converts hue, saturation, value to RGB
    pub fn from_hsv(h: u32, s: u32, v: u32) -> Self {
        assert!(h <= 360, "The given H in HSV values are not in valid range");
        assert!(s <= 100, "The given S in HSV values are not in valid range");
        assert!(v <= 100, "The given V in HSV values are not in valid range");

        let s = s as f64 / 100.0;
        let v = v as f64 / 100.0;
        let c = s * v;
        let x = c * (1.0 - (((h as f64 / 60.0) % 2.0) - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match h {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self::gamma_corrected(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }

    pub const fn gamma_corrected(r: u8, g: u8, b: u8) -> Self {
        Self::raw(GAMMA8[r as usize], GAMMA8[g as usize], GAMMA8[b as usize])
    }

    pub fn is_off(&self) -> bool {
        self.red() == 0 && self.green() == 0 && self.blue() == 0
    }

    pub fn zero_out(&mut self) {
        self.update_self(&Rgb::OFF);
    }
}

/// used to correct to the right color/brigthness
const GAMMA8: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5,
    5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 24, 24, 25, 25, 26, 27,
    27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72,
    73, 74, 75, 77, 78, 79, 81, 82, 83, 85, 86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104,
    105, 107, 109, 110, 112, 114, 115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137,
    138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175,
    177, 180, 182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
    223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
];
