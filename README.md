# SSD1677 E-Paper Display Driver

[![Crates.io](https://img.shields.io/crates/v/ssd1677)](https://crates.io/crates/ssd1677)
[![Docs.rs](https://docs.rs/ssd1677/badge.svg)](https://docs.rs/ssd1677)
[![License](https://img.shields.io/crates/l/ssd1677)](LICENSE)

A `no_std` driver for the [**SSD1677**](https://www.solomon-systech.com/product/ssd1677/) e-paper display controller, supporting displays up to **960x680 pixels** (datasheet max) with tri-color (black/white/red) support.

## Features

- `no_std` compatible - suitable for bare-metal embedded systems
- `embedded-hal` v1.0 support
- `embedded-graphics` integration (optional, enabled by default)
- Full and fast refresh modes
- Custom Look-Up Table (LUT) support for custom waveforms
- Display rotation support (0°, 90°, 180°, 270°)
- Type-safe configuration builder
- Efficient buffer management

## Supported Displays

The SSD1677 controller supports various e-paper display sizes, including:

- 4.2" (400x300)
- 5.83" (648x480)
- 7.5" (800x480)
- 9.7" (960x680)

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
ssd1677 = "0.1.0"
embedded-hal = "1.0.0"
```

Basic usage example:

```rust
use ssd1677::{Builder, Dimensions, Display, Interface, Rotation};
use embedded_hal::delay::DelayNs;

// Create hardware interface (SPI + GPIO pins)
let interface = Interface::new(spi, dc_pin, rst_pin, busy_pin);

// Configure display dimensions and rotation
// Note: Dimensions::new(rows, cols) == (height, width)
if let Ok(dims) = Dimensions::new(480, 800) {
    if let Ok(config) = Builder::new().dimensions(dims).rotation(Rotation::Rotate0).build() {
        // Create display driver and initialize
        let mut display = Display::new(interface, config);
        let _ = display.reset(&mut delay);

        // Update display with buffers
        let black_buffer = vec![0xFF; buffer_size]; // All white
        let red_buffer = vec![0x00; buffer_size];   // No red
        let _ = display.update(&black_buffer, &red_buffer, &mut delay);
    }
}
```

### Using with embedded-graphics

Enable the `graphics` feature (enabled by default):

```toml
[dependencies]
ssd1677 = { version = "0.1.0", features = ["graphics"] }
embedded-graphics = "0.8"
```

```rust
use ssd1677::{Builder, Dimensions, Display, Interface, Rotation, GraphicDisplay, Color};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    prelude::*,
    text::Text,
};

// Setup display...
let display = Display::new(interface, config);

// Create graphic display with buffers
let mut graphic_display = GraphicDisplay::new(
    display,
    vec![0u8; buffer_size],  // Black buffer
    vec![0u8; buffer_size],  // Red buffer
);

// Draw using embedded-graphics
Text::new("Hello, E-Paper!", Point::new(10, 20), 
    MonoTextStyle::new(&FONT_6X10, Color::Black))
    .draw(&mut graphic_display)?;

// Update display
graphic_display.update(&mut delay)?;
```

## Hardware Interface

The SSD1677 requires:

### Pin Connections

| SSD1677 Pin | MCU Pin | Description |
|------------|---------|-------------|
| VCC | 3.3V | Power supply |
| GND | GND | Ground |
| DIN | MOSI | SPI Data In |
| CLK | SCK | SPI Clock |
| CS | GPIO (CS) | SPI Chip Select |
| DC | GPIO | Data/Command select |
| RST | GPIO | Hardware reset |
| BUSY | GPIO (Input) | Busy status (active high) |

### Wiring Diagram

```
       MCU                    SSD1677 Display
    ┌─────────┐             ┌───────────────┐
    │         │             │               │
    │    MOSI ├─────────────┤ DIN           │
    │    SCK  ├─────────────┤ CLK           │
    │    CS   ├─────────────┤ CS            │
    │    GPIO ├─────────────┤ DC            │
    │    GPIO ├─────────────┤ RST           │
    │    GPIO ├─────────────┤ BUSY          │
    │         │             │               │
    │    3.3V ├─────────────┤ VCC           │
    │    GND  ├─────────────┤ GND           │
    │         │             │               │
    └─────────┘             └───────────────┘
```

## Development

### Setup

This project uses [just](https://github.com/casey/just) for task running. To set up the development environment:

```bash
# Install just (if not already installed)
cargo install just

# Setup rust components (rustfmt, clippy)
just setup

# Run all checks
just all
```

### Available Commands

```bash
just all              # Run all checks (format, lint, type-check, test, doc)
just ci               # Full CI simulation locally
just format           # Format code with rustfmt
just lint             # Run clippy lints
just type-check       # Type check the code
just test             # Run tests
just doc              # Build documentation
just doc-open         # Build and open documentation
just clean            # Clean build artifacts
just publish-dry      # Dry run publish check
```

### Required Rust Components

The `rust-toolchain.toml` file specifies required components:
- `rustfmt` - Code formatting
- `clippy` - Linting
- `rust-docs` - Documentation

These are automatically installed when you run `just setup` or when rustup detects the toolchain file.

## Configuration

### Display Dimensions

Dimensions must meet these constraints:
- Rows: 1 to 680 (height)
- Columns: 8 to 960, must be multiple of 8 (width)

```rust
use ssd1677::Dimensions;

// 7.5" display (800x480) -> rows=480, cols=800
let dims = Dimensions::new(480, 800)?;

// 5.83" display (648x480) -> rows=480, cols=648
let dims = Dimensions::new(480, 648)?;

// 4.2" display (400x300) -> rows=300, cols=400
let dims = Dimensions::new(300, 400)?;
```

### Panel-Specific Settings

Some SSD1677 parameters are panel-dependent (booster soft-start, gate scanning,
data entry mode, RAM Y inversion, update control values, and clear values). The
defaults aim to be reasonable, but many panels require tuning.

Example (800x480 panel configuration):

```rust
use ssd1677::{Builder, Dimensions};

let config = Builder::new()
    .dimensions(Dimensions::new(480, 800)?)
    .gate_scanning(0x02)
    .data_entry_mode(0x01)   // X increment, Y decrement
    .ram_y_inverted(true)
    .clear_bw_value(0xF7)
    .clear_red_value(0xF7)
    .display_update_ctrl2_full(0x34)
    .display_update_ctrl2_partial(0xD4)
    .display_update_ctrl2_fast(0x1C)
    .build()?;
```

### Advanced Configuration

```rust
use ssd1677::Builder;

let config = Builder::new()
    .dimensions(dims)
    .rotation(Rotation::Rotate90)
    .vcom(0x3C)                    // VCOM voltage
    .border_waveform(0x01)         // Border waveform
    .booster_soft_start([0xAE, 0xC7, 0xC3, 0xC0, 0x40])
    .build()?;
```

### Refresh Modes

```rust
// Full refresh - clears entire display, best quality
fn full_refresh<D: DelayNs>(&mut self, delay: &mut D) -> Result<(), Error<I>>;

// Partial update from buffers
fn update<D: DelayNs>(
    &mut self,
    black_buffer: &[u8],
    red_buffer: &[u8],
    delay: &mut D
) -> Result<(), Error<I>>;
```

### Custom LUT

For custom waveforms (e.g., grayscale or fast refresh):

```rust
// Load custom 112-byte LUT
const CUSTOM_LUT: [u8; 112] = [/* your waveform data */];
display.load_lut(&CUSTOM_LUT)?;
```

## Examples

See the [examples/](examples/) directory for complete examples including:
- Basic initialization and display update
- Graphics drawing with embedded-graphics
- Custom LUT usage
- Rotation handling

## Resources

- [SSD1677 Datasheet](https://www.solumco.com/files/SSD1677.pdf)
- [embedded-hal](https://github.com/rust-embedded/embedded-hal) - Hardware abstraction layer
- [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) - 2D graphics library

## License

- MIT license ([LICENSE](LICENSE))
