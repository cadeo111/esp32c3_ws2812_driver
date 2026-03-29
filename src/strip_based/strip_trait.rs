use core::{
    marker::PhantomData,
    time::Duration,
};

use heapless::Vec;
use smart_leds::RGB8;
use thiserror::Error;

/// save RGB as u32 color value (24bit) representation for neopixel
///
/// e.g. rgb: (1,2,4)
/// G        R        B
/// 7      0 7      0 7      0
/// 00000010 00000001 00000100
pub const fn rgb_to_packed(r: u8, g: u8, b: u8) -> u32 {
    ((g as u32) << 16) | ((r as u32) << 8) | b as u32
}

pub trait Color24bit: Sized {
    fn as_24_bit_color_u32(&self) -> u32 {
        rgb_to_packed(self.red(), self.green(), self.blue())
    }
    fn red(&self) -> u8;
    fn green(&self) -> u8;
    fn blue(&self) -> u8;
    fn to_rgb8(&self) -> RGB8 {
        RGB8::new(self.red(), self.green(), self.blue())
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self;
    fn from_other_color<C: Color24bit>(c: C) -> Self {
        Self::from_rgb(c.red(), c.green(), c.blue())
    }
}

#[derive(Error, Debug)]
pub enum LedStripTraitError {
    #[error("index {index} out of range  of length {length}")]
    IndexOutOfRangeOfStrip { length: usize, index: usize },
    #[error("signal vector was too small for transmission")]
    SignalVectorTooSmall,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub struct SignalPeriod {
    high: Duration,
    low: Duration,
}
impl SignalPeriod {
    pub const fn new(high: Duration, low: Duration) -> Self {
        Self { high, low }
    }
    pub fn high(&self) -> Duration {
        self.high
    }
    pub fn low(&self) -> Duration {
        self.low
    }
}

#[allow(dead_code)]
pub const fn times_24(length: usize) -> usize {
    length * 24 + 1
}

/// MIN_SIGNAL_LENGTH should be at least LENGTH *24
/// _methods are internal, should be avoided
pub trait LedStrip<const LENGTH: usize, const MIN_SIGNAL_LENGTH: usize, C: Color24bit>:
    Sized
{
    type Error: core::error::Error + From<LedStripTraitError>;
    type SignalPeriodType;

    const LOGIC_0: SignalPeriod =
        SignalPeriod::new(Duration::from_nanos(350), Duration::from_nanos(800));
    const LOGIC_1: SignalPeriod =
        SignalPeriod::new(Duration::from_nanos(700), Duration::from_nanos(600));

    fn clear(&mut self) {
        for i in 0..LENGTH {
            self._zero_out_index_unchecked(i);
        }
    }
    fn _zero_out_index_unchecked(&mut self, index: usize);
    fn _set_led_unchecked(&mut self, index: usize, color: C);
    fn set_led<C24B: Color24bit>(
        &mut self,
        index: usize,
        color: C24B,
    ) -> core::result::Result<(), Self::Error> {
        if index >= LENGTH {
            Err(LedStripTraitError::IndexOutOfRangeOfStrip {
                length: LENGTH,
                index,
            })?;
        }
        self._set_led_unchecked(index, C::from_other_color(color));
        Ok(())
    }

    fn get_led(&self, index: usize) -> core::result::Result<C, Self::Error> {
        if index >= LENGTH {
            Err(LedStripTraitError::IndexOutOfRangeOfStrip {
                length: LENGTH,
                index,
            })?;
        }
        Ok(self._get_led_unchecked(index))
    }

    fn _get_led_unchecked(&self, index: usize) -> C;

    fn _get_periods(
        &mut self,
    ) -> core::result::Result<heapless::Vec<Self::SignalPeriodType, MIN_SIGNAL_LENGTH>, Self::Error>
    {
        use const_format::formatcp;

        const {
            assert!(
                MIN_SIGNAL_LENGTH >= LENGTH * 24,
                concat!("Buffer size must be at least 24 x LENGTH (the number of leds)!")
            );
        }

        let mut signal = Vec::<Self::SignalPeriodType, MIN_SIGNAL_LENGTH>::new();

        let mut s = [SignalPeriod::default(); 24];

        for index in 0..LENGTH {
            let c = self.get_led(index)?;
            let color: u32 = c.as_24_bit_color_u32();
            for i in (0..24).rev() {
                let p = 2_u32.pow(i);
                let bit: bool = (p & color) != 0;
                s[(23 - i) as usize] = if bit { Self::LOGIC_1 } else { Self::LOGIC_0 };
            }
            for item in s {
                signal
                    .push(self._convert_from_signal_period(item)?)
                    .map_err(|_| LedStripTraitError::SignalVectorTooSmall)?;
            }
        }
        Ok(signal)
    }

    fn _convert_from_signal_period(
        &self,
        p: SignalPeriod,
    ) -> core::result::Result<Self::SignalPeriodType, Self::Error>;

    fn _transmit_signal(
        &mut self,
        signal: heapless::Vec<Self::SignalPeriodType, MIN_SIGNAL_LENGTH>,
    ) -> core::result::Result<(), Self::Error>;

    fn refresh(&mut self) -> core::result::Result<(), Self::Error> {
        let signal = self._get_periods()?;
        self._transmit_signal(signal)
    }

    /// createsd to mimic the smart LEDs interface
    /// https://github.com/smart-leds-rs/smart-leds-trait/tree/master
    fn write_all<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Color24bit,
    {
        for (index, color) in iterator.into_iter().enumerate() {
            self.set_led(index, color)?
        }
        self.refresh()
    }
}
