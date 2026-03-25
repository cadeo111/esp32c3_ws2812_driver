# ESP32-C3 WS2812 Example

This example demonstrates how to control a WS2812 (NeoPixel) LED strip using an ESP32-C3 microcontroller.

It relies on the `ws2812-driver` crate alongside `esp-hal`, `esp-rtos`, and `embassy` to drive a smooth, asynchronous rainbow color progression.

## Hardware Setup

* **Microcontroller**: An ESP32-C3 development board.
* **Hardware**: A WS2812 / NeoPixel LED strip.
* **Wiring**:
  * Connect the Data Input (DI) of the LED strip to **GPIO 6** on the ESP32-C3.
  * Connect GND to a common ground.
  * Connect VCC to a 5V power supply. *(Warning: If driving a long strip of LEDs, do not draw power directly from the MCU's pins as the current draw may damage the board. Use an external 5V power source and a logic-level shifter if necessary).*

*Note: The code is configured to update `64` LEDs by default. You can change the `LED_LENGTH` constant in `src/main.rs` to match the length of your physical hardware.*

## Software Stack

* **`esp-hal`**: The hardware abstraction layer for the ESP32-C3. It handles configuring the `RMT` peripheral used to maintain the strict microsecond timings required by WS2812 LEDs.
* **`embassy` / `esp-rtos`**: Provides asynchronous task spawning and timers (`Timer::after`) used to throttle the animation frames smoothly.
* **`ws2812-driver`**: Provides the color generation utilities (such as `Rgb::rainbow_progression`) and the underlying driver implementation.

## How it Works

1. The program initializes the hardware, clocks, and timers.
2. An instance of `LedStripEsp32C3` is attached to `GPIO6` and the `RMT` peripheral.
3. The initial pixel at index `0` is turned red to signal successful initialization.
4. An infinite `loop` runs a rainbow progression effect that shifts down the strip over time. 

## Running the Example

To build, flash, and run the example on your connected ESP32-C3 device, use `cargo run` (ensure you have `espflash` configured and installed):

```bash
cargo run --release
```

If you are using Wokwi, the custom logger is also already configured to emit logs formatted with timings in standard out.

Enjoy the rainbow!