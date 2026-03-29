use core::{
    num::TryFromIntError,
    time::Duration,
};

use esp_hal::{
    Blocking,
    gpio::{
        Level,
        interconnect::PeripheralOutput,
    },
    peripherals::RMT,
    rmt::{
        Channel,
        PulseCode,
        Rmt,
        Tx,
        TxChannelConfig,
        TxChannelCreator,
    },
    time::Rate,
};
use heapless::Vec;
// use crate::neopixel::board_ctrl::BoardSizeType;
use thiserror::Error;

use super::{
    LedStripTraitError,
    SignalPeriod,
    rgb::Rgb,
    strip_trait::LedStrip,
};

#[derive(Error, Debug)]
pub enum Esp32c3StripError {
    #[error("overflow when calculating ticks")]
    TickOverflowError,
    #[error("failed to convert ticks to u16")]
    FailedToConvertTickError(#[from] TryFromIntError),
    #[error("failed to configure rmt: {0}")]
    FailedToConfigureRMT(#[source] esp_hal::rmt::Error),
    #[error("index {index} out of range  of length {length}")]
    IndexOutOfRangeOfStrip { length: usize, index: usize },
    #[error("signal vector was too small for transmission")]
    SignalVectorTooSmall,
    #[error("failed to transmit on rmt: {0}")]
    FailedToTransmit(#[source] esp_hal::rmt::Error),
    #[error("failed to wait on rmt transmition: {0}")]
    FailedToWait(#[source] esp_hal::rmt::Error),
}

impl From<LedStripTraitError> for Esp32c3StripError {
    fn from(e: LedStripTraitError) -> Self {
        match e {
            LedStripTraitError::IndexOutOfRangeOfStrip { length, index } => {
                Self::IndexOutOfRangeOfStrip { length, index }
            }
            LedStripTraitError::SignalVectorTooSmall => Self::SignalVectorTooSmall,
        }
    }
}

pub type StripResult<T = ()> = core::result::Result<T, Esp32c3StripError>;

pub fn duration_to_ticks(ticks: Rate, duration: &Duration) -> StripResult<u16> {
    let ticks = duration
        .as_nanos()
        .checked_mul(ticks.as_hz() as u128)
        .ok_or(Esp32c3StripError::TickOverflowError)?
        / 1_000_000_000;

    Ok(u16::try_from(ticks)?)
}

pub const fn min_length_times_24_plus_one(length: usize) -> usize {
    length * 24 + 1
}

#[derive(Debug)]
pub struct LedStripEsp32C3<'a, const LENGTH: usize, const LENGTH_TIMES_24_PLUS_1: usize> {
    tx: Option<Channel<'a, Blocking, Tx>>,
    data: [Rgb; LENGTH],
    tick_rate: Rate,
}

impl<'a, const LENGTH: usize, const LENGTH_TIMES_24_PLUS_1: usize>
    LedStripEsp32C3<'a, LENGTH, LENGTH_TIMES_24_PLUS_1>
{
    pub fn new(led_pin: impl PeripheralOutput<'a>, rmt: RMT<'a>) -> StripResult<Self> {
        const {
            assert!(
                LENGTH_TIMES_24_PLUS_1 == LENGTH * 24 + 1,
                "Buffer size must be at least 24 x LENGTH +1!"
            );
        }

        // ESP32C3 can only do RMT at 80MHz
        let tick_rate = Rate::from_mhz(80);

        let rmt = Rmt::new(rmt, tick_rate).unwrap();

        let config: TxChannelConfig = TxChannelConfig::default().with_clk_divider(1);

        let tx: Channel<'_, Blocking, Tx> = rmt
            .channel0
            .configure_tx(led_pin, config)
            .map_err(Esp32c3StripError::FailedToConfigureRMT)?;

        // let tx: Channel<Async, ConstChannelAccess<Tx, 0>> =   channel.configure_tx(
        //     led_pin,
        //     config,
        // ).map_err(|e|anyhow!("Error creating rmt tx channel: {:?}", e))?;
        Ok(LedStripEsp32C3 {
            tick_rate,
            tx: Some(tx),
            data: [Rgb::new(0, 0, 0); LENGTH],
        })
    }

    pub fn clear(&mut self) {
        for i in 0..LENGTH {
            self.data[i].zero_out();
        }
    }

    pub fn set_led(&mut self, index: usize, rgb: Rgb) -> StripResult<()> {
        if index >= LENGTH {
            return Err(Esp32c3StripError::IndexOutOfRangeOfStrip {
                length: LENGTH,
                index,
            });
        }
        self.data[index] = rgb;

        Ok(())
    }
}



impl<'a, const LENGTH: usize, const LENGTH_TIMES_24_PLUS_1: usize>
    LedStrip<LENGTH, LENGTH_TIMES_24_PLUS_1, Rgb>
    for LedStripEsp32C3<'a, LENGTH, LENGTH_TIMES_24_PLUS_1>
{
    type Error = Esp32c3StripError;
    type SignalPeriodType = PulseCode;

    fn _zero_out_index_unchecked(&mut self, index: usize) {
        self.data[index].zero_out();
    }

    fn _set_led_unchecked(&mut self, index: usize, color: Rgb) {
        self.data[index] = color;
    }
       fn _get_led_unchecked(&self, index: usize) -> Rgb {
        self.data[index]
    }


    fn _convert_from_signal_period(
        &self,
        signal_period: SignalPeriod,
    ) -> core::result::Result<Self::SignalPeriodType, Self::Error> {
        let tick_rate = self.tick_rate;
        Ok(PulseCode::new(
                Level::High,
                duration_to_ticks(tick_rate, &signal_period.high())?,
                Level::Low,
                duration_to_ticks(tick_rate, &signal_period.low())?,
            ))
    }

    fn _transmit_signal(
        &mut self,
        mut signal: heapless::Vec<Self::SignalPeriodType, LENGTH_TIMES_24_PLUS_1>,
    ) -> core::result::Result<(), Self::Error> {
        signal
            .push(PulseCode::end_marker())
            // TODO this should probably panic as there is already a const assert that should make this impossible
            .map_err(|_| Esp32c3StripError::SignalVectorTooSmall)?;

        let tx = self.tx.take();
        assert!(
            tx.is_some(),
            "TX should always be some unless an error has occured"
        );
        let tx = tx.expect("TX should always be some unless an error has occured");

        let transaction = tx
            .transmit(&signal)
            .map_err(Esp32c3StripError::FailedToTransmit)?;
        let tx = transaction
            .wait()
            // TODO: maybe hand the tx and self back in an error case for graceful recovery?
            .map_err(|(err, _)| Esp32c3StripError::FailedToWait(err))?;
        self.tx = Some(tx);

        Ok(())
    }
    
 
}
