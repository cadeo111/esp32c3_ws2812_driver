use core::{
    default,
    slice::IterMut,
};

use esp_hal::{gpio::interconnect::PeripheralOutput, peripherals::RMT};

use crate::{
    grid_based::grid_trait::{
        Grid,
        LedGrid,
    },
    strip_based::{
        LedStripEsp32C3, Rgb, StripResult, min_length_times_24_plus_one
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
> {
    grid: Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, Rgb>,
    strip: LedStripEsp32C3<'strip_lifetime, GRID_SIZE, GRID_SIZE_TIMES_24_PLUS_1>,
}

pub struct Grid3d<
    const HEIGHT: usize,
    const WIDTH: usize,
    const DEPTH: usize,
    const SIZE: usize,
    T: Default,
>([T; SIZE]);

impl<
    const HEIGHT: usize,
    const WIDTH: usize,
    const DEPTH: usize,
    const SIZE: usize,
    T: Default + Copy,
> Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, T>
{
    pub fn get_z(&mut self, z: usize) -> Grid2d<'_, HEIGHT, WIDTH, T> {
        let start_index = z * HEIGHT * WIDTH;
        let end_index = (start_index + HEIGHT * WIDTH) - 1;
        Grid2d(&mut self.0[start_index..end_index])
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.0.iter_mut()
    }
}
impl<
    const HEIGHT: usize,
    const WIDTH: usize,
    const DEPTH: usize,
    const SIZE: usize,
    T: Default + Copy,
> Default for Grid3d<HEIGHT, WIDTH, DEPTH, SIZE, T>
{
    fn default() -> Self {
        Self([Default::default(); SIZE])
    }
}

pub struct Grid2d<'a, const HEIGHT: usize, const WIDTH: usize, T: Default>(&'a mut [T]);

impl<'a, const HEIGHT: usize, const WIDTH: usize, T: Default> Grid2d<'a, WIDTH, HEIGHT, T> {
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.0.iter_mut()
    }
}

pub fn create_8x8<
    'strip_lifetime,
    const HEIGHT: usize,
    const WIDTH: usize,
    const GRID_SIZE: usize,
    const GRID_SIZE_TIMES_24_PLUS_1: usize,
    const DEPTH: usize,
    const SIZE: usize,
>(led_pin: impl PeripheralOutput<'strip_lifetime>, rmt: RMT<'strip_lifetime>)
-> StripResult<LedGridEsp32c3<'strip_lifetime, HEIGHT, WIDTH, GRID_SIZE, GRID_SIZE_TIMES_24_PLUS_1, DEPTH, SIZE>>
{
    let strip = LedStripEsp32C3::<'_, GRID_SIZE, GRID_SIZE_TIMES_24_PLUS_1>::new(led_pin,rmt)?;
    Ok(LedGridEsp32c3 { grid: Grid3d::default(), strip })
}
