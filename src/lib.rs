//! SSD1677 E-Paper Display Driver
//!
//! A driver for the SSD1677 e-paper display controller supporting displays up to 960x680 pixels.
//!
//! ## Features
//!
//! - `no_std` compatible
//! - `embedded-hal` v1.0 support
//! - `embedded-graphics` integration (with `graphics` feature)
//! - Configurable display dimensions
//! - Full and fast refresh modes
//! - Custom LUT support
//! - Rotation support
//!
//! ## Usage
//!
//! ```rust,no_run
//! use core::convert::Infallible;
//! use embedded_hal::delay::DelayNs;
//! use embedded_hal::digital::{InputPin, OutputPin};
//! use embedded_hal::spi::{Operation, SpiDevice};
//! use ssd1677::{Builder, Dimensions, Display, Interface, Rotation};
//!
//! # struct MockSpi;
//! # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
//! # impl SpiDevice for MockSpi {
//! #     fn transaction(
//! #         &mut self,
//! #         _operations: &mut [Operation<'_, u8>],
//! #     ) -> Result<(), Self::Error> {
//! #         Ok(())
//! #     }
//! # }
//! # struct MockPin;
//! # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
//! # impl OutputPin for MockPin {
//! #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
//! #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
//! # }
//! # impl InputPin for MockPin {
//! #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
//! #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
//! # }
//! # struct MockDelay;
//! # impl DelayNs for MockDelay { fn delay_ns(&mut self, _ns: u32) {} }
//! # let spi = MockSpi;
//! # let dc = MockPin;
//! # let rst = MockPin;
//! # let busy = MockPin;
//! # let mut delay = MockDelay;
//! let interface = Interface::new(spi, dc, rst, busy);
//! let dims = match Dimensions::new(480, 800) {
//!     Ok(dims) => dims,
//!     Err(_) => return,
//! };
//! let config = match Builder::new().dimensions(dims).rotation(Rotation::Rotate0).build() {
//!     Ok(config) => config,
//!     Err(_) => return,
//! };
//!
//! let mut display = Display::new(interface, config);
//! let _ = display.reset(&mut delay);
//! ```

#![no_std]

#[cfg(any(test, feature = "alloc"))]
extern crate alloc;

/// Color types for tri-color e-paper displays
pub mod color;
/// SSD1677 command definitions
pub mod command;
/// Display configuration types and builder
pub mod config;
/// Core display operations
pub mod display;
/// Error types for the driver
pub mod error;
/// Hardware interface abstraction
pub mod interface;
/// Look-Up Tables for refresh modes
pub mod lut;
/// Coordinate rotation utilities
pub mod rotation;

/// Graphics support via embedded-graphics (requires `graphics` feature)
#[cfg(feature = "graphics")]
pub mod graphics;

pub use color::Color;
pub use config::{
    Builder, Config, Dimensions, MAX_GATE_OUTPUTS, MAX_SOURCE_OUTPUTS, RamXAddressing, Rotation,
};
pub use display::{DeepSleepMode, Display, RefreshMode, Region, UpdateRegion};
pub use error::{BuilderError, Error};
pub use interface::InterfaceError;
pub use interface::{DEFAULT_BUSY_TIMEOUT_MS, DisplayInterface, Interface};

#[cfg(feature = "graphics")]
pub use graphics::GraphicDisplay;
