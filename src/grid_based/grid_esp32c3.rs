use core::{
    default,
    marker::PhantomData,
    ops::{
        Deref,
        DerefMut,
    },
    slice::IterMut,
};

use esp_hal::{
    Blocking,
    aes::dma::AesTransfer,
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

use crate::{
    grid_based::grid_trait::{
        Grid,
        LedGrid,
    },
    strip_based::{
        Esp32c3StripError,
        LedStrip,
        LedStripEsp32C3,
        Rgb,
        SignalPeriod,
        StripResult,
        duration_to_ticks,
        min_length_times_24_plus_one,
    },
};

// struct BasicArrayBasedGrid<'a,
//     const HEIGHT: usize,
//     const WIDTH: usize,
//     T: Copy + Sized + Default,
// >(&'a mut [T]);

// type BasicArrayBasedGrid<'a,
//     const HEIGHT: usize,
//     const WIDTH: usize,
//     T: Copy + Sized + Default,
// > = &'a mut [T];

pub struct LedGridEsp32c3<
    'strip_lifetime,
    const HEIGHT: usize,
    const WIDTH: usize,
    const GRID_SIZE: usize,
    const GRID_SIZE_TIMES_24_PLUS_1: usize,
    const DEPTH: usize,
    const SIZE: usize,
    P: PhyisicalGridLayout<HEIGHT, WIDTH>,
> {
    grid: Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, Rgb>,
    tx: Option<Channel<'strip_lifetime, Blocking, Tx>>,
    tick_rate: Rate,
    marker: PhantomData<P>,
}

pub trait PhyisicalGridLayout<const HEIGHT: usize, const WIDTH: usize> {
    fn get_index_in_strip_from_x_y(x: usize, y: usize) -> usize;
    fn get_x_y_in_strip_from_index(index: usize) -> (usize, usize);
}
pub struct RowsSameDirection<const HEIGHT: usize, const WIDTH: usize>;
impl<const HEIGHT: usize, const WIDTH: usize> PhyisicalGridLayout<HEIGHT, WIDTH>
    for RowsSameDirection<HEIGHT, WIDTH>
{
    fn get_index_in_strip_from_x_y(x: usize, y: usize) -> usize {
        assert!(
            x < WIDTH,
            concat!("X must be less than ", stringify!(WIDTH))
        );
        assert!(
            y < HEIGHT,
            concat!("Y must be less than ", stringify!(HEIGHT))
        );
        y * WIDTH + x
    }
    fn get_x_y_in_strip_from_index(index: usize) -> (usize, usize) {
        assert!(
            index < { WIDTH * HEIGHT },
            concat!(
                "index must be less than ",
                stringify!(WIDTH),
                "*",
                stringify!(HEIGHT)
            )
        );

        let x = index % WIDTH;
        let y = index / WIDTH;
        (x, y)
    }
}

impl<
    'strip_lifetime,
    const HEIGHT: usize,
    const WIDTH: usize,
    const GRID_SIZE: usize,
    const GRID_SIZE_TIMES_24_PLUS_1: usize,
    const DEPTH: usize,
    const SIZE: usize,
    P: PhyisicalGridLayout<HEIGHT, WIDTH>,
>
    LedGridEsp32c3<
        'strip_lifetime,
        HEIGHT,
        WIDTH,
        GRID_SIZE,
        GRID_SIZE_TIMES_24_PLUS_1,
        DEPTH,
        SIZE,
        P,
    >
{
    pub fn new(
        led_pin: impl PeripheralOutput<'strip_lifetime>,
        rmt: RMT<'strip_lifetime>,
    ) -> StripResult<Self> {
        const {
            assert!(
                GRID_SIZE_TIMES_24_PLUS_1 == GRID_SIZE * 24 + 1,
                "Buffer size must be at least 24 x LENGTH +1!"
            );
            assert!(
                GRID_SIZE == HEIGHT * WIDTH,
                "GRID_SIZE should be HEIGHT * WIDTH"
            );
            assert!(
                SIZE == HEIGHT * WIDTH * DEPTH,
                "SIZE should be HEIGHT * WIDTH * DEPTH"
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
        Ok(LedGridEsp32c3 {
            tick_rate,
            tx: Some(tx),
            grid: Grid3d::default(),
            marker: PhantomData::<P> {},
        })
    }

    fn get_strip_interface<'s, 't>(
        &'t mut self,
    ) -> StripInterfaceForInterface<
        's,
        'strip_lifetime,
        HEIGHT,
        WIDTH,
        GRID_SIZE,
        GRID_SIZE_TIMES_24_PLUS_1,
        DEPTH,
        SIZE,
        P,
    >
    where
        't: 's,
    {
        (&mut self.tx, self.tick_rate, &self.grid, PhantomData {})
    }

    fn strip(
        &mut self,
    ) -> impl LedStrip<SIZE, GRID_SIZE_TIMES_24_PLUS_1, Rgb, Error = Esp32c3StripError> {
        self.get_strip_interface()
    }

    pub fn refresh(&mut self) -> Result<(), crate::strip_based::Esp32c3StripError> {
        self.strip().refresh()
    }
}

type StripInterfaceForInterface<
    'obj_lifetime,
    'strip_lifetime,
    const HEIGHT: usize,
    const WIDTH: usize,
    const GRID_SIZE: usize,
    const GRID_SIZE_TIMES_24_PLUS_1: usize,
    const DEPTH: usize,
    const SIZE: usize,
    P: PhyisicalGridLayout<HEIGHT, WIDTH>,
> where
    'obj_lifetime: 'strip_lifetime,
= (
    &'obj_lifetime mut Option<Channel<'strip_lifetime, Blocking, Tx>>,
    Rate,
    &'obj_lifetime Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, Rgb>,
    PhantomData<P>,
);

impl<
    'obj_lifetime,
    'strip_lifetime,
    const HEIGHT: usize,
    const WIDTH: usize,
    const GRID_SIZE: usize,
    const GRID_SIZE_TIMES_24_PLUS_1: usize,
    const DEPTH: usize,
    const SIZE: usize,
    P: PhyisicalGridLayout<HEIGHT, WIDTH>,
> LedStrip<GRID_SIZE, GRID_SIZE_TIMES_24_PLUS_1, Rgb>
    for StripInterfaceForInterface<
        'obj_lifetime,
        'strip_lifetime,
        HEIGHT,
        WIDTH,
        GRID_SIZE,
        GRID_SIZE_TIMES_24_PLUS_1,
        DEPTH,
        SIZE,
        P,
    >
{
    type Error = Esp32c3StripError;
    type SignalPeriodType = PulseCode;

    fn _zero_out_index_unchecked(&mut self, _index: usize) {
        unimplemented!("This is deliberatly not implemented, used the grid methods")
    }

    fn _set_led_unchecked(&mut self, _index: usize, _color: Rgb) {
        unimplemented!("This is deliberatly not implemented, used the grid methods")
    }

    fn _get_led_unchecked(&self, index: usize) -> Rgb {
        let (x, y) = P::get_x_y_in_strip_from_index(index);
        self.2.composite_for_position(x, y)
    }

    fn _convert_from_signal_period(
        &self,
        signal_period: SignalPeriod,
    ) -> core::result::Result<Self::SignalPeriodType, Self::Error> {
        let tick_rate = self.1;
        Ok(PulseCode::new(
            Level::High,
            duration_to_ticks(tick_rate, &signal_period.high())?,
            Level::Low,
            duration_to_ticks(tick_rate, &signal_period.low())?,
        ))
    }

    fn _transmit_signal(
        &mut self,
        mut signal: heapless::Vec<Self::SignalPeriodType, GRID_SIZE_TIMES_24_PLUS_1>,
    ) -> core::result::Result<(), Self::Error> {
        signal
            .push(PulseCode::end_marker())
            // TODO this should probably panic as there is already a const assert that should make this impossible
            .map_err(|_| Esp32c3StripError::SignalVectorTooSmall)?;

        let tx = self.0.take();
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
        self.0.replace(tx);

        Ok(())
    }
}

impl<
    'strip_lifetime,
    const HEIGHT: usize,
    const WIDTH: usize,
    const GRID_SIZE: usize,
    const GRID_SIZE_TIMES_24_PLUS_1: usize,
    const DEPTH: usize,
    const SIZE: usize,
    P: PhyisicalGridLayout<HEIGHT, WIDTH>,
> Deref
    for LedGridEsp32c3<
        'strip_lifetime,
        HEIGHT,
        WIDTH,
        GRID_SIZE,
        GRID_SIZE_TIMES_24_PLUS_1,
        DEPTH,
        SIZE,
        P,
    >
{
    type Target = Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, Rgb>;

    fn deref(&self) -> &Self::Target {
        &self.grid
    }
}

impl<
    'strip_lifetime,
    const HEIGHT: usize,
    const WIDTH: usize,
    const GRID_SIZE: usize,
    const GRID_SIZE_TIMES_24_PLUS_1: usize,
    const DEPTH: usize,
    const SIZE: usize,
    P: PhyisicalGridLayout<HEIGHT, WIDTH>,
> DerefMut
    for LedGridEsp32c3<
        'strip_lifetime,
        HEIGHT,
        WIDTH,
        GRID_SIZE,
        GRID_SIZE_TIMES_24_PLUS_1,
        DEPTH,
        SIZE,
        P,
    >
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.grid
    }
}

pub trait Visibility {
    fn is_visible(&self) -> bool;
    fn invisible() -> Self;
}

impl Visibility for Rgb {
    fn is_visible(&self) -> bool {
        !self.is_off()
    }

    fn invisible() -> Self {
        Self::default()
    }
}

pub struct Grid3d<
    const HEIGHT: usize,
    const WIDTH: usize,
    const DEPTH: usize,
    const SIZE: usize,
    T: Default + Copy + Visibility,
>([T; SIZE]);

impl<
    const HEIGHT: usize,
    const WIDTH: usize,
    const DEPTH: usize,
    const SIZE: usize,
    T: Default + Copy + Visibility,
> Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, T>
{
    // 0 is top
    pub fn get_z_mut(&mut self, z_index: usize) -> Grid2dMut<'_, HEIGHT, WIDTH, T> {
        let start_index = z_index * HEIGHT * WIDTH;
        let end_index = (start_index + HEIGHT * WIDTH);
        Grid2dMut(&mut self.0[start_index..end_index])
    }

    // 0 is top
    pub fn get_z(&self, z_index: usize) -> Grid2d<'_, HEIGHT, WIDTH, T> {
        let start_index = z_index * HEIGHT * WIDTH;
        let end_index = (start_index + HEIGHT * WIDTH);
        Grid2d(&self.0[start_index..end_index])
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.0.iter_mut()
    }

    fn composite_for_position(&self, x: usize, y: usize) -> T {
        for z_index in 0..DEPTH {
            let t = self.get_z(z_index).get_x_y((x, y));
            if t.is_visible() {
                return t;
            }
        }
        T::invisible()
    }
}
impl<
    const HEIGHT: usize,
    const WIDTH: usize,
    const DEPTH: usize,
    const SIZE: usize,
    T: Default + Copy + Visibility,
> Default for Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, T>
{
    fn default() -> Self {
        Self([Default::default(); SIZE])
    }
}

pub struct Grid2dMut<'a, const HEIGHT: usize, const WIDTH: usize, T: Default + Copy + Visibility>(
    &'a mut [T],
);
pub struct Grid2d<'a, const HEIGHT: usize, const WIDTH: usize, T: Default + Copy + Visibility>(
    &'a [T],
);

impl<'a, const HEIGHT: usize, const WIDTH: usize, T: Default + Copy + Visibility>
    Grid2d<'a, WIDTH, HEIGHT, T>
{
    fn get_index_from_x_y(x: usize, y: usize) -> usize {
        assert!(x < WIDTH, "X must be less than {}", WIDTH);
        assert!(y < HEIGHT, "Y must be less than {}", HEIGHT);
        let index = y * WIDTH + x;
        assert!(index < HEIGHT * WIDTH);
        index
    }
    fn get_x_y_from_index(index: usize) -> (usize, usize) {
        let x = index % WIDTH;
        let y = index / WIDTH;
        (x, y)
    }
    pub fn get_x_y(&self, (x, y): (usize, usize)) -> T {
        self.0[Self::get_index_from_x_y(x, y)]
    }
}

impl<'a, const HEIGHT: usize, const WIDTH: usize, T: Default + Copy + Visibility>
    Grid2dMut<'a, WIDTH, HEIGHT, T>
{
    pub fn get_x_y(&mut self, (x, y): (usize, usize)) -> &mut T {
        &mut self.0[Grid2d::<'a, WIDTH, HEIGHT, T>::get_index_from_x_y(x, y)]
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = ((usize, usize), &mut T)> {
        self.0
            .iter_mut()
            .enumerate()
            .map(|(index, a)| (Grid2d::<HEIGHT, WIDTH, T>::get_x_y_from_index(index), a))
    }
    pub fn clear(&mut self) {
        self.iter_mut().for_each(|(_, r)| *r = T::invisible());
    }
}

impl<'a, const HEIGHT: usize, const WIDTH: usize, T: Default + Copy + Visibility>
    From<&'a Grid2dMut<'a, WIDTH, HEIGHT, T>> for Grid2d<'a, WIDTH, HEIGHT, T>
{
    fn from(value: &'a Grid2dMut<'a, WIDTH, HEIGHT, T>) -> Grid2d<'a, WIDTH, HEIGHT, T> {
        Grid2d::<'a, WIDTH, HEIGHT, T>(value.0)
    }
}

pub use paste;
#[macro_export]
macro_rules! generate_grid_definition {
($name:ident, $height:expr, $width:expr, $depth:expr) => {
$crate::grid_based::paste::paste!{
    // #[allow(non_upper_case_globals)]
    // const [<__ $name  _ HEIGHT>]: usize = { $height };
    // #[allow(non_upper_case_globals)]
    // const [<__ $name  _ WIDTH>]:  usize = { $width };
    // #[allow(non_upper_case_globals)]
    // const [<__ $name  _ GRID_SIZE>]: usize = { $height * $width };
    // #[allow(non_upper_case_globals)]
    // const [<__ $name  _ GRID_SIZE_TIMES_24_PLUS_1>]: usize = { $height * $width * 24 + 1 };
    // #[allow(non_upper_case_globals)]
    // const [<__ $name  _ DEPTH>]: usize = { $depth };
    // #[allow(non_upper_case_globals)]
    // const [<__ $name  _ SIZE>]: usize = { $height * $width * $depth };
    pub struct $name;


    impl $name {

        pub const HEIGHT: usize = { $height };
        pub const WIDTH: usize = { $width };
        pub const GRID_SIZE: usize = { $height * $width };
        pub const GRID_SIZE_TIMES_24_PLUS_1: usize = { $height * $width * 24 + 1 };
        pub const DEPTH: usize = { $depth };
        pub const SIZE: usize = { $height * $width * $depth };

        pub fn create<'strip_lifetime, P: $crate::grid_based::PhyisicalGridLayout<{Self::HEIGHT}, { Self::WIDTH}>>(
            led_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'strip_lifetime>,
            rmt: esp_hal::peripherals::RMT<'strip_lifetime>,
            grid_layout: P,
        ) -> $crate::strip_based::StripResult<
            $crate::grid_based::LedGridEsp32c3<
                'strip_lifetime,
                {Self::HEIGHT},
                { Self::WIDTH},
                { Self::GRID_SIZE },
                { Self::GRID_SIZE_TIMES_24_PLUS_1 },
                { Self::DEPTH},
                { Self::SIZE },
                P,
            >,
        > {
            let _ = grid_layout;
            $crate::grid_based::LedGridEsp32c3::new(led_pin, rmt)
        }

    }
}
}
}

generate_grid_definition!(__test__GRID8x8x8, 8, 8, 8);

// const GRID_SIZE_8x8: usize = 8;
// const GRID_DEPTH_8x8: usize = 3;
// pub type LedGrid8x8<'a> = LedGridEsp32c3<
//     'a,
//     GRID_SIZE_8x8,
//     GRID_SIZE_8x8,
//     { GRID_SIZE_8x8 * GRID_SIZE_8x8 },
//     { (GRID_SIZE_8x8 * GRID_SIZE_8x8 * 24 + 1) },
//     3,
//     { GRID_SIZE_8x8 * GRID_SIZE_8x8 * GRID_DEPTH_8x8 },
// >;

// pub trait GridDimensions {
//     #[type_const]
//     const HEIGHT: usize;
//     #[type_const]
//     const WIDTH: usize;
//     #[type_const]
//     const GRID_SIZE: usize;
//     #[type_const]
//     const GRID_SIZE_TIMES_24_PLUS_1: usize;
//     #[type_const]
//     const DEPTH: usize;
//     #[type_const]
//     const SIZE: usize;
// }

// // #[macro_export]
// macro_rules! generate_grid_definition {
//     ($name:ident, $height:expr, $width:expr, $depth:expr) => {
//         struct $name;
//         impl $crate::grid_based::GridDimensions for $name {
//             #[type_const]
//             const HEIGHT: usize = { $height };
//             #[type_const]
//             const WIDTH: usize = { $width };
//             #[type_const]
//             const GRID_SIZE: usize = { $height * $width };
//             #[type_const]
//             const GRID_SIZE_TIMES_24_PLUS_1: usize = { $height * $width * 24 + 1 };
//             #[type_const]
//             const DEPTH: usize = { $depth };
//             #[type_const]
//             const SIZE: usize = { $height * $width * $depth };
//         }
//     };
// }

// generate_grid_definition!{LedGrid8x8_1, 8, 8, 3};

// pub struct LedGrid8x8;
// impl GridDimensions for LedGrid8x8 {
//     #[type_const]
//     const HEIGHT: usize = 1;

//     #[type_const]
//     const WIDTH: usize = 2;

//     #[type_const]
//     const GRID_SIZE: usize = 3;

//     #[type_const]
//     const GRID_SIZE_TIMES_24_PLUS_1: usize = 4;

//     #[type_const]
//     const DEPTH: usize = 5;

//     #[type_const]
//     const SIZE: usize = 6;
// }

// pub fn create_grid_with_trait<
//     'strip_lifetime,
//     D: GridDimensions,
//     P: PhyisicalGridLayout<{ D::HEIGHT }, { D::WIDTH }>,
// >(
//     led_pin: impl PeripheralOutput<'strip_lifetime>,
//     rmt: RMT<'strip_lifetime>,
//     grid_layout:P
// ) -> StripResult<
//     LedGridEsp32c3<
//         'strip_lifetime,
//         { D::HEIGHT },
//         { D::WIDTH },
//         { D::GRID_SIZE },
//         { D::GRID_SIZE_TIMES_24_PLUS_1 },
//         { D::DEPTH },
//         { D::SIZE },
//         P,
//     >,
// > {
//     let _ = grid_layout;
//     LedGridEsp32c3::new(led_pin, rmt)
// }
