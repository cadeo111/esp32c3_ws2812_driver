# WS2812 Driver

A `no_std` Rust driver for WS2812 (NeoPixel) addressable LED strips, designed specifically for embedded environments.

## Features

* **`no_std` Support**: Built from the ground up for bare-metal embedded targets.
* **Hardware Agnostic Trait**: Core logic is abstracted behind the flexible `LedStrip` trait (`strip_trait.rs`), allowing easy implementation for new hardware.
* **ESP32-C3 Native Support**: Includes a built-in implementation (`LedStripEsp32C3`) leveraging the ESP32-C3's RMT peripheral for precise 80MHz signal timing.
* **Rich Color Utilities**: Provides an `Rgb` type featuring:
  * Built-in gamma correction for accurate brightness scaling.
  * HSV-to-RGB conversion (e.g., `rainbow_progression`).
  * Extensive built-in color palettes (Primaries, Pastels, Nature Tones, Temperature Whites).
* **Smart-LEDs Compatibility**: Implements mimicking interfaces (`write_all`) to integrate smoothly with the broader `smart-leds` ecosystem.

## Examples

This repository includes examples demonstrating how to use the driver across different microcontrollers:

* **`esp32c3`**: Demonstrates driving a 64-LED strip using the ESP32-C3's native RMT peripheral. Features an asynchronous looping rainbow animation powered by `esp-hal`, `esp-rtos`, and `embassy`.
* **`rp2040`**: Demonstrates driving LEDs using the Raspberry Pi Pico's PIO state machines with `embassy-rp` and PIO WS2812 DMA.

## Quick Start (ESP32-C3)

You can easily initialize the built-in driver for an ESP32-C3 device:

```rust
use ws2812_driver::neopixel::{LedStrip, LedStripEsp32C3, Rgb, min_length_times_24_plus_one};

// Initialize your peripherals
let peripherals = esp_hal::init(esp_hal::Config::default());

// Define the length of your LED strip
const LED_LENGTH: usize = 64;

// Create the LED strip handler using GPIO6 and the RMT peripheral
let mut strip: LedStripEsp32C3<'_, LED_LENGTH, { min_length_times_24_plus_one(LED_LENGTH) }> = 
    LedStripEsp32C3::new(peripherals.GPIO6, peripherals.RMT).unwrap();

// Set an LED to a color and refresh the strip
strip.set_led(0, Rgb::RED).unwrap();
strip.refresh().unwrap();
```

## License

Dual-licensed under either the MIT license or the Apache License, Version 2.0 at your option.