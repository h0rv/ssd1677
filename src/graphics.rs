//! Graphics support via embedded-graphics
//!
//! This module provides the [`GraphicDisplay`] struct which wraps [`Display`]
//! and implements the [`DrawTarget`](embedded_graphics_core::draw_target::DrawTarget) trait from
//! the embedded-graphics ecosystem.
//!
//! ## Features
//!
//! - 2D graphics primitives (lines, rectangles, circles, text, etc.)
//! - Image support via embedded-graphics image modules
//! - Rotation support
//! - Efficient pixel buffer management
//!
//! ## Example
//!
//! ```rust,no_run
//! use embedded_graphics::{
//!     mono_font::{ascii::FONT_6X10, MonoTextStyle},
//!     prelude::*,
//!     primitives::{Circle, Rectangle, PrimitiveStyle},
//!     text::Text,
//! };
//! use ssd1677::{Color, GraphicDisplay};
//! # use core::convert::Infallible;
//! # use embedded_hal::delay::DelayNs;
//! # use embedded_hal::digital::{InputPin, OutputPin};
//! # use embedded_hal::spi::{Operation, SpiDevice};
//! # use ssd1677::{Builder, Dimensions, Display, Interface};
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
//! # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
//! # let dims = match Dimensions::new(480, 800) {
//! #     Ok(dims) => dims,
//! #     Err(_) => return,
//! # };
//! # let config = match Builder::new().dimensions(dims).build() {
//! #     Ok(config) => config,
//! #     Err(_) => return,
//! # };
//! # let display_driver = Display::new(interface, config);
//! # let buffer_size = dims.buffer_size();
//! # let black_buffer = vec![0xFFu8; buffer_size];
//! # let red_buffer = vec![0x00u8; buffer_size];
//! # let mut delay = MockDelay;
//! // Create graphic display with buffers
//! let mut display = GraphicDisplay::new(display_driver, black_buffer, red_buffer);
//!
//! // Clear to white
//! display.clear(Color::White);
//!
//! // Draw shapes
//! let _ = Rectangle::new(Point::new(10, 10), Size::new(50, 30))
//!     .into_styled(PrimitiveStyle::with_fill(Color::Black))
//!     .draw(&mut display);
//!
//! let _ = Circle::new(Point::new(100, 50), 40)
//!     .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
//!     .draw(&mut display);
//!
//! // Draw text
//! let _ = Text::new(
//!     "Hello, E-Paper!",
//!     Point::new(10, 100),
//!     MonoTextStyle::new(&FONT_6X10, Color::Black),
//! )
//! .draw(&mut display);
//!
//! // Update physical display
//! let _ = display.update(&mut delay);
//! ```

use core::convert::Infallible;
use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    prelude::Pixel,
};
use embedded_hal::delay::DelayNs;

use crate::color::Color;
use crate::display::Display;
use crate::error::Error;
use crate::interface::DisplayInterface;
use crate::rotation::apply_rotation;

/// Display with graphics buffers
///
/// This wrapper around [`Display`] provides embedded-graphics support
/// and manages the pixel buffers for black/white and red planes.
///
/// ## Type Parameters
///
/// * `I` - Interface type implementing [`DisplayInterface`]
/// * `B1` - Buffer type implementing `AsMut<[u8]>` for the black/white buffer
/// * `B2` - Buffer type implementing `AsMut<[u8]>` for the red buffer
///
/// ## Example
///
/// ```rust,no_run
/// use ssd1677::GraphicDisplay;
/// # use core::convert::Infallible;
/// # use embedded_hal::digital::{InputPin, OutputPin};
/// # use embedded_hal::spi::{Operation, SpiDevice};
/// # use ssd1677::{Builder, Dimensions, Display, Interface};
/// # struct MockSpi;
/// # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
/// # impl SpiDevice for MockSpi {
/// #     fn transaction(
/// #         &mut self,
/// #         _operations: &mut [Operation<'_, u8>],
/// #     ) -> Result<(), Self::Error> {
/// #         Ok(())
/// #     }
/// # }
/// # struct MockPin;
/// # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
/// # impl OutputPin for MockPin {
/// #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # impl InputPin for MockPin {
/// #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
/// #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
/// # }
/// # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
/// # let dims = match Dimensions::new(480, 800) {
/// #     Ok(dims) => dims,
/// #     Err(_) => return,
/// # };
/// # let config = match Builder::new().dimensions(dims).build() {
/// #     Ok(config) => config,
/// #     Err(_) => return,
/// # };
/// # let display = Display::new(interface, config);
/// # let buffer_size = dims.buffer_size();
/// let mut graphic_display = GraphicDisplay::new(
///     display,
///     vec![0u8; buffer_size],  // Black buffer
///     vec![0u8; buffer_size],  // Red buffer
/// );
///
/// // Use with embedded-graphics...
/// ```
pub struct GraphicDisplay<I, B1, B2>
where
    I: DisplayInterface,
    B1: AsMut<[u8]>,
    B2: AsMut<[u8]>,
{
    /// The underlying display driver
    display: Display<I>,
    /// Buffer for black/white pixels
    black_buffer: B1,
    /// Buffer for red pixels
    red_buffer: B2,
}

type GraphicsResult<I> = core::result::Result<(), Error<I>>;
type GraphicsNewResult<I, T> = core::result::Result<T, Error<I>>;

impl<I, B1, B2> GraphicDisplay<I, B1, B2>
where
    I: DisplayInterface,
    B1: AsMut<[u8]>,
    B2: AsMut<[u8]>,
{
    /// Create a new GraphicDisplay
    ///
    /// # Arguments
    ///
    /// * `display` - The [`Display`] driver instance
    /// * `black_buffer` - Buffer for black/white pixels (must be at least `dimensions.buffer_size()` bytes)
    /// * `red_buffer` - Buffer for red pixels (must be at least `dimensions.buffer_size()` bytes)
    ///
    /// # Panics
    ///
    /// Panics if either buffer is smaller than the required size based on
    /// **physical** (unrotated) dimensions. The buffer size is always calculated
    /// from physical dimensions regardless of rotation setting.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use ssd1677::{Display, GraphicDisplay};
    /// # use core::convert::Infallible;
    /// # use embedded_hal::digital::{InputPin, OutputPin};
    /// # use embedded_hal::spi::{Operation, SpiDevice};
    /// # use ssd1677::{Builder, Dimensions, Interface};
    /// # struct MockSpi;
    /// # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
    /// # impl SpiDevice for MockSpi {
    /// #     fn transaction(
    /// #         &mut self,
    /// #         _operations: &mut [Operation<'_, u8>],
    /// #     ) -> Result<(), Self::Error> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// # struct MockPin;
    /// # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
    /// # impl OutputPin for MockPin {
    /// #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl InputPin for MockPin {
    /// #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
    /// #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
    /// # }
    /// # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
    /// # let dims = match Dimensions::new(480, 800) {
    /// #     Ok(dims) => dims,
    /// #     Err(_) => return,
    /// # };
    /// # let config = match Builder::new().dimensions(dims).build() {
    /// #     Ok(config) => config,
    /// #     Err(_) => return,
    /// # };
    /// # let display = Display::new(interface, config);
    /// # let buffer_size = dims.buffer_size();
    /// let graphic_display = GraphicDisplay::new(
    ///     display,
    ///     vec![0u8; buffer_size],
    ///     vec![0u8; buffer_size],
    /// );
    /// ```
    pub fn new(display: Display<I>, mut black_buffer: B1, mut red_buffer: B2) -> Self {
        let required = display.dimensions().buffer_size();
        assert!(
            black_buffer.as_mut().len() >= required,
            "black_buffer too small: required {} bytes, got {}",
            required,
            black_buffer.as_mut().len()
        );
        assert!(
            red_buffer.as_mut().len() >= required,
            "red_buffer too small: required {} bytes, got {}",
            required,
            red_buffer.as_mut().len()
        );
        Self {
            display,
            black_buffer,
            red_buffer,
        }
    }

    /// Try to create a new GraphicDisplay, returning an error if buffers are too small
    ///
    /// This is the fallible version of [`new`](Self::new).
    ///
    /// # Arguments
    ///
    /// * `display` - The [`Display`] driver instance
    /// * `black_buffer` - Buffer for black/white pixels
    /// * `red_buffer` - Buffer for red pixels
    ///
    /// # Errors
    ///
    /// Returns `Error::BufferTooSmall` if either buffer is smaller than the required
    /// size based on **physical** (unrotated) dimensions.
    pub fn try_new(
        display: Display<I>,
        mut black_buffer: B1,
        mut red_buffer: B2,
    ) -> GraphicsNewResult<I, Self> {
        let required = display.dimensions().buffer_size();
        if black_buffer.as_mut().len() < required {
            return Err(Error::BufferTooSmall {
                required,
                provided: black_buffer.as_mut().len(),
            });
        }
        if red_buffer.as_mut().len() < required {
            return Err(Error::BufferTooSmall {
                required,
                provided: red_buffer.as_mut().len(),
            });
        }
        Ok(Self {
            display,
            black_buffer,
            red_buffer,
        })
    }

    /// Clear buffers to a color
    ///
    /// Fills both buffers with the appropriate values to display the given color
    /// across the entire screen.
    ///
    /// # Arguments
    ///
    /// * `color` - The color to clear to ([`Color::Black`], [`Color::White`], or [`Color::Red`])
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use ssd1677::{Color, GraphicDisplay};
    /// # use core::convert::Infallible;
    /// # use embedded_hal::digital::{InputPin, OutputPin};
    /// # use embedded_hal::spi::{Operation, SpiDevice};
    /// # use ssd1677::{Builder, Dimensions, Display, Interface};
    /// # struct MockSpi;
    /// # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
    /// # impl SpiDevice for MockSpi {
    /// #     fn transaction(
    /// #         &mut self,
    /// #         _operations: &mut [Operation<'_, u8>],
    /// #     ) -> Result<(), Self::Error> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// # struct MockPin;
    /// # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
    /// # impl OutputPin for MockPin {
    /// #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl InputPin for MockPin {
    /// #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
    /// #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
    /// # }
    /// # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
    /// # let dims = match Dimensions::new(480, 800) {
    /// #     Ok(dims) => dims,
    /// #     Err(_) => return,
    /// # };
    /// # let config = match Builder::new().dimensions(dims).build() {
    /// #     Ok(config) => config,
    /// #     Err(_) => return,
    /// # };
    /// # let display = Display::new(interface, config);
    /// # let buffer_size = dims.buffer_size();
    /// # let mut graphic_display = GraphicDisplay::new(
    /// #     display,
    /// #     vec![0u8; buffer_size],
    /// #     vec![0u8; buffer_size],
    /// # );
    /// // Clear to white
    /// graphic_display.clear(Color::White);
    ///
    /// // Clear to black
    /// graphic_display.clear(Color::Black);
    /// ```
    pub fn clear(&mut self, color: Color) {
        let (bw, red) = (color.bw_byte(), color.red_byte());

        for byte in self.black_buffer.as_mut().iter_mut() {
            *byte = bw;
        }
        for byte in self.red_buffer.as_mut().iter_mut() {
            *byte = red;
        }
    }

    /// Update the display from buffers using full refresh
    ///
    /// Sends the current buffer contents to the display controller and triggers
    /// a refresh. The BUSY pin will go high during the refresh operation.
    ///
    /// # Arguments
    ///
    /// * `delay` - Delay implementation for busy-waiting
    ///
    /// # Errors
    ///
    /// Returns [`Error::Interface`] if there's a
    /// communication error.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use embedded_hal::delay::DelayNs;
    /// use ssd1677::GraphicDisplay;
    /// # use core::convert::Infallible;
    /// # use embedded_hal::digital::{InputPin, OutputPin};
    /// # use embedded_hal::spi::{Operation, SpiDevice};
    /// # use ssd1677::{Builder, Dimensions, Display, Interface};
    /// # struct MockSpi;
    /// # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
    /// # impl SpiDevice for MockSpi {
    /// #     fn transaction(
    /// #         &mut self,
    /// #         _operations: &mut [Operation<'_, u8>],
    /// #     ) -> Result<(), Self::Error> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// # struct MockPin;
    /// # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
    /// # impl OutputPin for MockPin {
    /// #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl InputPin for MockPin {
    /// #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
    /// #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
    /// # }
    /// # struct MockDelay;
    /// # impl DelayNs for MockDelay { fn delay_ns(&mut self, _ns: u32) {} }
    /// # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
    /// # let dims = match Dimensions::new(480, 800) {
    /// #     Ok(dims) => dims,
    /// #     Err(_) => return,
    /// # };
    /// # let config = match Builder::new().dimensions(dims).build() {
    /// #     Ok(config) => config,
    /// #     Err(_) => return,
    /// # };
    /// # let display = Display::new(interface, config);
    /// # let buffer_size = dims.buffer_size();
    /// # let mut graphic_display = GraphicDisplay::new(
    /// #     display,
    /// #     vec![0u8; buffer_size],
    /// #     vec![0u8; buffer_size],
    /// # );
    /// # let mut delay = MockDelay;
    /// // After drawing...
    /// if let Err(err) = graphic_display.update(&mut delay) {
    ///     let _ = err;
    /// }
    /// ```
    pub fn update<D: DelayNs>(&mut self, delay: &mut D) -> GraphicsResult<I> {
        self.display
            .update(self.black_buffer.as_mut(), self.red_buffer.as_mut(), delay)
    }

    /// Update the display with specified refresh mode
    ///
    /// # Arguments
    ///
    /// * `mode` - Refresh mode (Full, Partial, or Fast)
    /// * `delay` - Delay implementation for busy-waiting
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use embedded_hal::delay::DelayNs;
    /// use ssd1677::{GraphicDisplay, RefreshMode};
    /// # use core::convert::Infallible;
    /// # use embedded_hal::digital::{InputPin, OutputPin};
    /// # use embedded_hal::spi::{Operation, SpiDevice};
    /// # use ssd1677::{Builder, Dimensions, Display, Interface};
    /// # struct MockSpi;
    /// # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
    /// # impl SpiDevice for MockSpi {
    /// #     fn transaction(
    /// #         &mut self,
    /// #         _operations: &mut [Operation<'_, u8>],
    /// #     ) -> Result<(), Self::Error> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// # struct MockPin;
    /// # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
    /// # impl OutputPin for MockPin {
    /// #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl InputPin for MockPin {
    /// #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
    /// #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
    /// # }
    /// # struct MockDelay;
    /// # impl DelayNs for MockDelay { fn delay_ns(&mut self, _ns: u32) {} }
    /// # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
    /// # let dims = match Dimensions::new(480, 800) {
    /// #     Ok(dims) => dims,
    /// #     Err(_) => return,
    /// # };
    /// # let config = match Builder::new().dimensions(dims).build() {
    /// #     Ok(config) => config,
    /// #     Err(_) => return,
    /// # };
    /// # let display = Display::new(interface, config);
    /// # let buffer_size = dims.buffer_size();
    /// # let mut graphic_display = GraphicDisplay::new(
    /// #     display,
    /// #     vec![0u8; buffer_size],
    /// #     vec![0u8; buffer_size],
    /// # );
    /// # let mut delay = MockDelay;
    /// // Fast update for UI changes
    /// if let Err(err) = graphic_display.update_with_mode(RefreshMode::Fast, &mut delay) {
    ///     let _ = err;
    /// }
    ///
    /// // Full refresh periodically to clear ghosting
    /// if let Err(err) = graphic_display.update_with_mode(RefreshMode::Full, &mut delay) {
    ///     let _ = err;
    /// }
    /// ```
    pub fn update_with_mode<D: DelayNs>(
        &mut self,
        mode: crate::display::RefreshMode,
        delay: &mut D,
    ) -> GraphicsResult<I> {
        self.display.update_with_mode(
            self.black_buffer.as_mut(),
            self.red_buffer.as_mut(),
            mode,
            delay,
        )
    }

    /// Access the underlying Display
    ///
    /// Returns an immutable reference to the wrapped [`Display`].
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use ssd1677::GraphicDisplay;
    /// # use core::convert::Infallible;
    /// # use embedded_hal::digital::{InputPin, OutputPin};
    /// # use embedded_hal::spi::{Operation, SpiDevice};
    /// # use ssd1677::{Builder, Dimensions, Display, Interface};
    /// # struct MockSpi;
    /// # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
    /// # impl SpiDevice for MockSpi {
    /// #     fn transaction(
    /// #         &mut self,
    /// #         _operations: &mut [Operation<'_, u8>],
    /// #     ) -> Result<(), Self::Error> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// # struct MockPin;
    /// # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
    /// # impl OutputPin for MockPin {
    /// #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl InputPin for MockPin {
    /// #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
    /// #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
    /// # }
    /// # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
    /// # let dims = match Dimensions::new(480, 800) {
    /// #     Ok(dims) => dims,
    /// #     Err(_) => return,
    /// # };
    /// # let config = match Builder::new().dimensions(dims).build() {
    /// #     Ok(config) => config,
    /// #     Err(_) => return,
    /// # };
    /// # let display = Display::new(interface, config);
    /// # let buffer_size = dims.buffer_size();
    /// # let graphic_display = GraphicDisplay::new(
    /// #     display,
    /// #     vec![0u8; buffer_size],
    /// #     vec![0u8; buffer_size],
    /// # );
    /// let dims = graphic_display.display().dimensions();
    /// let _ = dims;
    /// ```
    pub fn display(&self) -> &Display<I> {
        &self.display
    }

    /// Access the underlying Display mutably
    ///
    /// Returns a mutable reference to the wrapped [`Display`].
    /// This can be used to access low-level operations directly.
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use ssd1677::GraphicDisplay;
    /// # use core::convert::Infallible;
    /// # use embedded_hal::digital::{InputPin, OutputPin};
    /// # use embedded_hal::spi::{Operation, SpiDevice};
    /// # use ssd1677::{Builder, Dimensions, Display, Interface};
    /// # struct MockSpi;
    /// # impl embedded_hal::spi::ErrorType for MockSpi { type Error = Infallible; }
    /// # impl SpiDevice for MockSpi {
    /// #     fn transaction(
    /// #         &mut self,
    /// #         _operations: &mut [Operation<'_, u8>],
    /// #     ) -> Result<(), Self::Error> {
    /// #         Ok(())
    /// #     }
    /// # }
    /// # struct MockPin;
    /// # impl embedded_hal::digital::ErrorType for MockPin { type Error = Infallible; }
    /// # impl OutputPin for MockPin {
    /// #     fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// #     fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) }
    /// # }
    /// # impl InputPin for MockPin {
    /// #     fn is_high(&mut self) -> Result<bool, Self::Error> { Ok(false) }
    /// #     fn is_low(&mut self) -> Result<bool, Self::Error> { Ok(true) }
    /// # }
    /// # let interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
    /// # let dims = match Dimensions::new(480, 800) {
    /// #     Ok(dims) => dims,
    /// #     Err(_) => return,
    /// # };
    /// # let config = match Builder::new().dimensions(dims).build() {
    /// #     Ok(config) => config,
    /// #     Err(_) => return,
    /// # };
    /// # let display = Display::new(interface, config);
    /// # let buffer_size = dims.buffer_size();
    /// # let mut graphic_display = GraphicDisplay::new(
    /// #     display,
    /// #     vec![0u8; buffer_size],
    /// #     vec![0u8; buffer_size],
    /// # );
    /// let custom_lut = [0u8; ssd1677::lut::LUT_SIZE];
    ///
    /// // Load a custom LUT
    /// if let Err(err) = graphic_display.display_mut().load_lut(&custom_lut) {
    ///     let _ = err;
    /// }
    /// ```
    pub fn display_mut(&mut self) -> &mut Display<I> {
        &mut self.display
    }

    /// Set a single pixel to a color
    ///
    /// Internal method used by the [`DrawTarget`] implementation.
    /// Applies rotation transformation and updates both buffers appropriately.
    fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        let dims = self.display.dimensions();
        let width = dims.cols as u32;
        let height = dims.rows as u32;

        if x >= width || y >= height {
            return;
        }

        let rotation = self.display.rotation();
        let (index, bit) = apply_rotation(x, y, width, height, rotation);

        if index >= self.black_buffer.as_mut().len() {
            return;
        }

        match color {
            Color::Black => {
                self.black_buffer.as_mut()[index] &= !bit;
                self.red_buffer.as_mut()[index] &= !bit;
            }
            Color::White => {
                self.black_buffer.as_mut()[index] |= bit;
                self.red_buffer.as_mut()[index] &= !bit;
            }
            Color::Red => {
                self.black_buffer.as_mut()[index] |= bit;
                self.red_buffer.as_mut()[index] |= bit;
            }
        }
    }
}

impl<I, B1, B2> DrawTarget for GraphicDisplay<I, B1, B2>
where
    I: DisplayInterface,
    B1: AsMut<[u8]>,
    B2: AsMut<[u8]>,
{
    type Color = Color;
    type Error = Infallible;

    fn draw_iter<Iter>(&mut self, pixels: Iter) -> Result<(), Self::Error>
    where
        Iter: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let sz = self.size();

        for Pixel(Point { x, y }, color) in pixels {
            if x < 0 || y < 0 {
                continue;
            }

            let x = x as u32;
            let y = y as u32;

            if x >= sz.width || y >= sz.height {
                continue;
            }

            self.set_pixel(x, y, color);
        }

        Ok(())
    }
}

impl<I, B1, B2> OriginDimensions for GraphicDisplay<I, B1, B2>
where
    I: DisplayInterface,
    B1: AsMut<[u8]>,
    B2: AsMut<[u8]>,
{
    fn size(&self) -> Size {
        let rotated = self.display.config().rotated_dimensions();
        Size::new(rotated.cols as u32, rotated.rows as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Builder, Dimensions, Rotation};
    use embedded_hal::delay::DelayNs;

    #[derive(Debug)]
    struct MockInterface;

    impl DisplayInterface for MockInterface {
        type Error = core::convert::Infallible;

        fn send_command(&mut self, _command: u8) -> Result<(), Self::Error> {
            Ok(())
        }

        fn send_data(&mut self, _data: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn reset<D: DelayNs>(&mut self, _delay: &mut D) {}

        fn busy_wait<D: DelayNs>(&mut self, _delay: &mut D) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn test_display(rotation: Rotation) -> Display<MockInterface> {
        let config = Builder::new()
            .dimensions(Dimensions::new(480, 480).unwrap())
            .rotation(rotation)
            .build()
            .unwrap();
        Display::new(MockInterface, config)
    }

    #[test]
    fn test_graphic_display_buffer_size_uses_physical_dimensions() {
        let display = test_display(Rotation::Rotate0);
        let required = display.dimensions().buffer_size();
        assert_eq!(required, 480 * 480 / 8);

        let black_buf = alloc::vec![0u8; required];
        let red_buf = alloc::vec![0u8; required];
        let gd = GraphicDisplay::new(display, black_buf, red_buf);
        assert_eq!(gd.size(), Size::new(480, 480));
    }

    #[test]
    fn test_graphic_display_rotated_uses_physical_buffer_size() {
        let display = test_display(Rotation::Rotate90);
        let required = display.dimensions().buffer_size();
        assert_eq!(required, 480 * 480 / 8);

        let black_buf = alloc::vec![0u8; required];
        let red_buf = alloc::vec![0u8; required];
        let gd = GraphicDisplay::new(display, black_buf, red_buf);
        assert_eq!(gd.size(), Size::new(480, 480));
    }

    #[test]
    fn test_try_new_small_black_buffer_returns_error() {
        let display = test_display(Rotation::Rotate0);
        let required = display.dimensions().buffer_size();

        let black_buf = alloc::vec![0u8; required - 1];
        let red_buf = alloc::vec![0u8; required];
        let result = GraphicDisplay::try_new(display, black_buf, red_buf);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_try_new_small_red_buffer_returns_error() {
        let display = test_display(Rotation::Rotate0);
        let required = display.dimensions().buffer_size();

        let black_buf = alloc::vec![0u8; required];
        let red_buf = alloc::vec![0u8; required - 1];
        let result = GraphicDisplay::try_new(display, black_buf, red_buf);
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_try_new_valid_buffers_succeeds() {
        let display = test_display(Rotation::Rotate0);
        let required = display.dimensions().buffer_size();

        let black_buf = alloc::vec![0u8; required];
        let red_buf = alloc::vec![0u8; required];
        let result = GraphicDisplay::try_new(display, black_buf, red_buf);
        assert!(result.is_ok());
    }

    #[test]
    #[should_panic(expected = "black_buffer too small")]
    fn test_new_panics_on_small_black_buffer() {
        let display = test_display(Rotation::Rotate0);
        let required = display.dimensions().buffer_size();

        let black_buf = alloc::vec![0u8; required - 1];
        let red_buf = alloc::vec![0u8; required];
        let _ = GraphicDisplay::new(display, black_buf, red_buf);
    }

    #[test]
    #[should_panic(expected = "red_buffer too small")]
    fn test_new_panics_on_small_red_buffer() {
        let display = test_display(Rotation::Rotate0);
        let required = display.dimensions().buffer_size();

        let black_buf = alloc::vec![0u8; required];
        let red_buf = alloc::vec![0u8; required - 1];
        let _ = GraphicDisplay::new(display, black_buf, red_buf);
    }
}
