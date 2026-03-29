#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

// #![feature(min_generic_const_args)]

use embassy_executor::Spawner;
use embassy_time::{
    Duration,
    Timer,
};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    timer::timg::TimerGroup,
};
use log::info;
use ws2812_driver::{
    generate_grid_definition,
    grid_based::RowsSameDirection,
    strip_based::{
        Color24bit,
        LedStrip,
        LedStripEsp32C3,
        Rgb,
        SignalPeriod,
        min_length_times_24_plus_one,
    },
};

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

macro_rules! error_then_panic {
    // This pattern captures the format string ($fmt) and any arguments ($arg)
    ($fmt:expr $(, $($arg:tt)*)?) => {
        {
            // We use a block to keep the scope clean
            let file = file!();
            let line = line!();

            // Log it first
            log::error!(concat!("[{}:{}] ", $fmt), file, line $(, $($arg)*)?);

            // Panic with the same formatted string
            panic!(concat!("[{}:{}] ", $fmt), file, line $(, $($arg)*)?);
        }
    };
}

macro_rules! panic_escape {
    ($result:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => {
                // Passes the error to your previous macro
                // This will include the file and line of the panic_escape! call
                error_then_panic!("Error: {:#?}", e);
            }
        }
    };
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    // fix printing problem in wokwi
    logger::init_custom_logger(log::LevelFilter::Info);
    // esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");
    // led_strip_example(peripherals.GPIO6, peripherals.RMT).await;
    led_grid_example(peripherals.GPIO6, peripherals.RMT).await;

    loop {}
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}

#[allow(clippy::large_stack_frames)]
pub async fn led_grid_example<'a>(
    led_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'a>,
    rmt: esp_hal::peripherals::RMT<'a>,
) -> ! {
    // use ws2812_driver::grid_based::GridDimensions;
    // use ws2812_driver::grid_based::LedGrid8x8;
    use ws2812_driver::grid_based::LedGridEsp32c3;

    generate_grid_definition!(MyGrid, 8, 8, 1);

    let mut grid = panic_escape!(MyGrid::create(led_pin, rmt, RowsSameDirection));

    let color = Rgb::CYBER_PURPLE;
    let mut index = 0;

    fn comparison((x, y): &(usize, usize), index: u8) -> bool {
        let index = index as usize;
        match index % 4 {
            0 => *x == (0 + (index / 4)) % 4,
            1 => *y == (0 + (index / 4)) % 4,
            2 => *x == MyGrid::WIDTH - (1 + (0 + (index / 4)) % 4),
            _ => *y == MyGrid::HEIGHT - (1 + (0 + (index / 4)) % 4),
        }
    }

    loop {
        grid.get_z_mut(0).clear();

        grid.get_z_mut(0)
            .iter_mut()
            .filter(|(p, _)| comparison(p, index))
            .for_each(|(_, rgb)| {
                rgb.update_self(&color);
            });
        panic_escape!(grid.refresh());

        index += 1;
        // index %= 4;
        Timer::after(Duration::from_millis(250)).await;

        // Timer::after(Duration::from_millis(250)).await;
        // grid.get_z_mut(0)
        //     .iter_mut()
        //     .filter(|((_, y), _)| *y == 0)
        //     .for_each(|(_, rgb)| {
        //         rgb.update_self(&color);
        //     });
        // grid.refresh();

        // Timer::after(Duration::from_millis(250)).await;
        // grid.get_z_mut(0)
        //     .iter_mut()
        //     .filter(|((x, _), _)| *x == MyGrid::WIDTH - 1)
        //     .for_each(|(_, rgb)| {
        //         rgb.update_self(&color);
        //     });
        // grid.refresh();

        // Timer::after(Duration::from_millis(250)).await;
        // grid.get_z_mut(0)
        //     .iter_mut()
        //     .filter(|((_, y), _)| *y == MyGrid::HEIGHT - 1)
        //     .for_each(|(_, rgb)| {
        //         rgb.update_self(&color);
        //     });
        // grid.refresh();
    }
}

#[allow(clippy::large_stack_frames)]
// clippy says this is 1168 bytes on the stack a little over the 1024 limit for the error
pub async fn led_strip_example<'a>(
    led_pin: impl esp_hal::gpio::interconnect::PeripheralOutput<'a>,
    rmt: esp_hal::peripherals::RMT<'a>,
) -> ! {
    use myrtio_light_composer::{
        Duration,
        EffectId,
        FilterProcessorConfig,
        Instant,
        IntentChannel,
        LightChangeIntent,
        LightEngineConfig,
        LightStateIntent,
        Renderer,
        Rgb as ComposerRGB,
        TransitionTimings,
        bounds::RenderingBounds,
        filter::BrightnessFilterConfig,
    };

    const LED_LENGTH: u8 = 1;
    let mut strip: LedStripEsp32C3<
        '_,
        { LED_LENGTH as usize },
        { min_length_times_24_plus_one(LED_LENGTH as usize) },
    > = LedStripEsp32C3::new(led_pin, rmt).unwrap();
    strip.set_led(0, Rgb::RED).unwrap();
    strip.refresh().unwrap();

    let mut idx = 0;
    let mut step = 0;
    let max_step = 360;
    info!("Hello world!");
    loop {
        strip.clear();
        for _ in 0..10 {
            strip
                .set_led(idx, Rgb::rainbow_progression(step, max_step))
                .unwrap();
            step += 1;
            step %= max_step;
            strip.refresh().unwrap();
            Timer::after(Duration::from_millis(5)).await;
        }
        info!("Color: {}", Rgb::rainbow_progression(step, max_step));

        // strip.refresh().unwrap();
        idx += 1;
        idx %= { LED_LENGTH as usize };
        // Timer::after(Duration::from_secs(1)).await;
        // if idx == 5 {
        //     break;
        // }
    }

    // 1. Create communication channel (static for 'static lifetime)
    static INTENTS: IntentChannel<16> = IntentChannel::new();

    const last_index: u8 = LED_LENGTH - 1;

    // 2. Configure the engine
    let config = LightEngineConfig {
        effect: EffectId::RainbowShort,
        bounds: RenderingBounds {
            start: 0,
            end: last_index,
        },
        timings: TransitionTimings {
            fade_out: Duration::from_millis(10),
            fade_in: Duration::from_millis(10),
            color_change: Duration::from_millis(10),
            brightness: Duration::from_millis(10),
        },
        filters: FilterProcessorConfig {
            brightness: BrightnessFilterConfig {
                min_brightness: 0,
                scale: 255,
                adjust: None,
            },
            color_correction: ComposerRGB::new(255, 255, 255),
        },
        brightness: 255,
        color: ComposerRGB::new(255, 180, 100),
    };

    // 3. Initialize renderer
    let receiver = INTENTS.receiver();
    let mut renderer = Renderer::<{ last_index as usize }, 16>::new(receiver, &config);

    // 4. Send commands (from anywhere - thread/interrupt safe)
    let sender = INTENTS.sender();
    let _ = sender.try_send(LightChangeIntent::State(LightStateIntent {
        brightness: Some(255),
        color: Some(ComposerRGB::new(255, 0, 0)),
        ..Default::default()
    }));

    struct WrapperRGB(ComposerRGB);
    impl Color24bit for WrapperRGB {
        fn red(&self) -> u8 {
            self.0.r
        }

        fn green(&self) -> u8 {
            self.0.g
        }

        fn blue(&self) -> u8 {
            self.0.b
        }
        fn from_rgb(r: u8, g: u8, b: u8) -> Self {
            Self(ComposerRGB::new(r, g, b))
        }
    }

    // 5. Render loop (caller provides timing)
    let mut time_ms: u64 = 0;
    loop {
        let now = Instant::from_millis(time_ms);
        let frame: &[ComposerRGB] = renderer.render(now);
        strip
            .write_all(frame.iter().map(|c| WrapperRGB(*c)))
            .unwrap();

        // Platform-specific delay (e.g., embassy Timer, std::thread::sleep, busy-wait)
        Timer::after(Duration::from_millis(10)).await;
        time_ms += 10;
    }
}

mod logger {
    use esp_println::print;
    use log::{
        LevelFilter,
        Metadata,
        Record,
    };

    // 1. Define an empty struct for your logger
    struct CustomEspLogger;

    const TIME_START: embassy_time::Instant = embassy_time::Instant::MIN;

    // 2. Implement the standard `log::Log` trait
    impl log::Log for CustomEspLogger {
        fn enabled(&self, _metadata: &Metadata) -> bool {
            // You can add more complex filtering here if you want
            true
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                // Use `print!` instead of `println!` so we can manually add \r\n
                // print!(
                //     "[{}] {}: {}\r\n",
                //     record.level(),
                //     record.target(),
                //     record.args()
                // );
                print_log_record(record)
            }
        }

        fn flush(&self) {}
    }

    const COLOR_ENABLED: bool = true;

    fn print_log_record(record: &log::Record) {
        let (color, reset) = if COLOR_ENABLED {
            const RESET: &str = "\u{001B}[0m";
            const RED: &str = "\u{001B}[31m";
            const GREEN: &str = "\u{001B}[32m";
            const YELLOW: &str = "\u{001B}[33m";
            const BLUE: &str = "\u{001B}[34m";
            const CYAN: &str = "\u{001B}[35m";

            let color = match record.level() {
                log::Level::Error => RED,
                log::Level::Warn => YELLOW,
                log::Level::Info => GREEN,
                log::Level::Debug => BLUE,
                log::Level::Trace => CYAN,
            };
            let reset = RESET;
            (color, reset)
        } else {
            ("", "")
        };

        let now = TIME_START.elapsed();
        let total_secs = now.as_secs();

        // let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let millis = now.as_millis() % 1000;

        print!(
            "{}{} ({:02}:{:02}.{:03}) - {}{}\n\r",
            color,
            record.level(),
            minutes,
            seconds,
            millis,
            record.args(),
            reset
        );
    }

    // 3. Create a static instance of the logger
    static LOGGER: CustomEspLogger = CustomEspLogger;

    // 4. Create an init function to bind it to the `log` crate
    pub fn init_custom_logger(level: LevelFilter) {
        unsafe {
            log::set_logger_racy(&LOGGER).unwrap();
            log::set_max_level_racy(level);
        }
    }
}
