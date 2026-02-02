//! Hardware interface abstraction
//!
//! This module provides the [`DisplayInterface`] trait and the [`Interface`] struct
//! for communicating with the SSD1677 controller over SPI.
//!
//! ## Hardware Requirements
//!
//! The SSD1677 requires:
//! - SPI bus (MOSI + SCK)
//! - 3 GPIO pins:
//!   - **DC**: Data/Command select (output)
//!   - **RST**: Reset (output, active low)
//!   - **BUSY**: Busy status (input, active high)
//!
//! ## Example
//!
//! ```rust,no_run
//! use embedded_hal::delay::DelayNs;
//! use embedded_hal::digital::{InputPin, OutputPin};
//! use embedded_hal::spi::{Operation, SpiDevice};
//! use ssd1677::{DisplayInterface, Interface};
//! # use core::convert::Infallible;
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
//! # let mut delay = MockDelay;
//! // Create interface with SPI and GPIO pins
//! let mut interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
//!
//! // Send command
//! let _ = interface.send_command(0x12); // Soft reset
//!
//! // Send data
//! let _ = interface.send_data(&[0xFF, 0x00, 0xFF]);
//!
//! // Wait for display ready
//! let _ = interface.busy_wait(&mut delay);
//! ```

use core::fmt::Debug;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

type InterfaceResult<T, E> = core::result::Result<T, E>;

/// Trait for hardware interface to SSD1677 controller
///
/// This trait abstracts over different hardware implementations,
/// allowing the [`Display`](crate::display::Display) to work with any
/// SPI + GPIO implementation that satisfies embedded-hal traits.
///
/// ## Implementing
///
/// For most cases, use the provided [`Interface`] struct. If you need
/// custom behavior (e.g., different pin polarities, additional CS control),
/// implement this trait on your own type.
pub trait DisplayInterface {
    /// Error type for interface operations
    ///
    /// Must implement [`Debug`] for error reporting.
    type Error: Debug;
    /// Send a command byte to the controller
    ///
    /// The implementation must:
    /// 1. Set DC pin low (command mode)
    /// 2. Send the command byte over SPI
    ///
    /// # Errors
    ///
    /// Returns an error if SPI communication or GPIO fails.
    #[allow(clippy::type_complexity)]
    fn send_command(&mut self, command: u8) -> InterfaceResult<(), Self::Error>;

    /// Send data bytes to the controller
    ///
    /// The implementation must:
    /// 1. Set DC pin high (data mode)
    /// 2. Send the data bytes over SPI
    ///
    /// # Arguments
    ///
    /// * `data` - Slice of bytes to send
    ///
    /// # Errors
    ///
    /// Returns an error if SPI communication or GPIO fails.
    #[allow(clippy::type_complexity)]
    fn send_data(&mut self, data: &[u8]) -> InterfaceResult<(), Self::Error>;

    /// Perform hardware reset
    ///
    /// The implementation must:
    /// 1. Set RST pin low
    /// 2. Wait at least 10ms
    /// 3. Set RST pin high
    /// 4. Wait at least 10ms
    ///
    /// # Arguments
    ///
    /// * `delay` - Delay implementation for timing
    fn reset<D: DelayNs>(&mut self, delay: &mut D);

    /// Wait for busy pin to go low (with timeout)
    ///
    /// Polls the BUSY pin until it goes low (display ready) or timeout occurs.
    /// BUSY is active high - when high, the display is processing a command.
    ///
    /// # Arguments
    ///
    /// * `delay` - Delay implementation for polling interval
    ///
    /// # Errors
    ///
    /// Returns [`InterfaceError::Timeout`] if BUSY doesn't go low within
    /// the implementation-specific timeout period.
    #[allow(clippy::type_complexity)]
    fn busy_wait<D: DelayNs>(&mut self, delay: &mut D) -> InterfaceResult<(), Self::Error>;
}

/// Errors that can occur at the interface level
///
/// Generic over SPI and GPIO error types.
#[derive(Debug)]
pub enum InterfaceError<SpiErr, PinErr> {
    /// SPI communication error
    Spi(SpiErr),
    /// GPIO pin error
    Pin(PinErr),
    /// Timeout waiting for busy pin
    Timeout,
}

impl<SpiErr: Debug, PinErr: Debug> core::fmt::Display for InterfaceError<SpiErr, PinErr> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Spi(e) => write!(f, "SPI error: {e:?}"),
            Self::Pin(e) => write!(f, "Pin error: {e:?}"),
            Self::Timeout => write!(f, "Timeout waiting for display"),
        }
    }
}

impl<SpiErr: Debug, PinErr: Debug> core::error::Error for InterfaceError<SpiErr, PinErr> {}

/// Default timeout for busy-wait in milliseconds
pub const DEFAULT_BUSY_TIMEOUT_MS: u32 = 30_000;

/// Hardware interface implementation for SSD1677
///
/// Implements [`DisplayInterface`] for embedded-hal v1.0 SPI and GPIO traits.
///
/// ## Type Parameters
///
/// * `SPI` - SPI device implementing [`SpiDevice`]
/// * `DC` - Data/Command pin implementing [`OutputPin`]
/// * `RST` - Reset pin implementing [`OutputPin`]
/// * `BUSY` - Busy pin implementing [`InputPin`]
///
/// ## Example
///
/// ```rust,no_run
/// use ssd1677::{Builder, Dimensions, Display, Interface};
/// # use core::convert::Infallible;
/// # use embedded_hal::digital::{InputPin, OutputPin};
/// # use embedded_hal::spi::{Operation, SpiDevice};
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
/// let interface = Interface::new(
///     MockSpi,  // SpiDevice
///     MockPin,  // OutputPin
///     MockPin,  // OutputPin
///     MockPin,  // InputPin
/// );
///
/// // Use with Display
/// # let dims = match Dimensions::new(480, 800) {
/// #     Ok(dims) => dims,
/// #     Err(_) => return,
/// # };
/// # let config = match Builder::new().dimensions(dims).build() {
/// #     Ok(config) => config,
/// #     Err(_) => return,
/// # };
/// let _display = Display::new(interface, config);
/// ```
pub struct Interface<SPI, DC, RST, BUSY> {
    /// SPI device for communication
    spi: SPI,
    /// Data/Command select pin (low=command, high=data)
    dc: DC,
    /// Reset pin (active low)
    rst: RST,
    /// Busy pin (active high)
    busy: BUSY,
    /// Timeout for busy-wait in milliseconds
    busy_timeout_ms: u32,
    /// Busy pin polarity (true = active high, false = active low)
    busy_active_high: bool,
}

impl<SPI, DC, RST, BUSY> Interface<SPI, DC, RST, BUSY>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin,
    BUSY: InputPin,
{
    /// Create a new Interface
    ///
    /// # Arguments
    ///
    /// * `spi` - SPI device (must implement [`SpiDevice`])
    /// * `dc` - Data/Command pin (output, low=command, high=data)
    /// * `rst` - Reset pin (output, active low)
    /// * `busy` - Busy pin (input, active high)
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// use ssd1677::{DisplayInterface, Interface};
    /// # use core::convert::Infallible;
    /// # use embedded_hal::digital::{InputPin, OutputPin};
    /// # use embedded_hal::spi::{Operation, SpiDevice};
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
    /// let _interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
    /// ```
    pub fn new(spi: SPI, dc: DC, rst: RST, busy: BUSY) -> Self {
        Self {
            spi,
            dc,
            rst,
            busy,
            busy_timeout_ms: DEFAULT_BUSY_TIMEOUT_MS,
            busy_active_high: true,
        }
    }

    /// Set the busy-wait timeout in milliseconds
    ///
    /// Default is 30,000ms (30 seconds). Set to 0 to disable timeout.
    pub fn set_busy_timeout(&mut self, timeout_ms: u32) -> &mut Self {
        self.busy_timeout_ms = timeout_ms;
        self
    }

    /// Get the current busy-wait timeout in milliseconds
    pub fn busy_timeout(&self) -> u32 {
        self.busy_timeout_ms
    }

    /// Set busy pin polarity
    ///
    /// Default is active-high. Set to false for active-low panels.
    pub fn set_busy_active_high(&mut self, active_high: bool) -> &mut Self {
        self.busy_active_high = active_high;
        self
    }

    /// Get busy pin polarity (true = active high)
    pub fn busy_active_high(&self) -> bool {
        self.busy_active_high
    }
}

impl<SPI, DC, RST, BUSY, PinErr> DisplayInterface for Interface<SPI, DC, RST, BUSY>
where
    SPI: SpiDevice,
    SPI::Error: Debug,
    DC: OutputPin<Error = PinErr>,
    RST: OutputPin<Error = PinErr>,
    BUSY: InputPin<Error = PinErr>,
    PinErr: Debug,
{
    type Error = InterfaceError<SPI::Error, PinErr>;

    fn send_command(&mut self, command: u8) -> InterfaceResult<(), Self::Error> {
        self.dc.set_low().map_err(|e| InterfaceError::Pin(e))?;
        self.spi
            .write(&[command])
            .map_err(|e| InterfaceError::Spi(e))?;
        Ok(())
    }

    fn send_data(&mut self, data: &[u8]) -> InterfaceResult<(), Self::Error> {
        self.dc.set_high().map_err(|e| InterfaceError::Pin(e))?;
        self.spi.write(data).map_err(|e| InterfaceError::Spi(e))?;
        Ok(())
    }

    fn reset<D: DelayNs>(&mut self, delay: &mut D) {
        // Reset sequence: LOW -> wait 10ms -> HIGH -> wait 10ms
        let _ = self.rst.set_low();
        delay.delay_ms(10);
        let _ = self.rst.set_high();
        delay.delay_ms(10);
    }

    fn busy_wait<D: DelayNs>(&mut self, delay: &mut D) -> InterfaceResult<(), Self::Error> {
        let mut iterations = 0u32;
        let timeout_ms = self.busy_timeout_ms;

        loop {
            let is_busy = if self.busy_active_high {
                self.busy.is_high()
            } else {
                self.busy.is_low()
            };

            let is_busy = match is_busy {
                Ok(value) => value,
                Err(e) => return Err(InterfaceError::Pin(e)),
            };

            if !is_busy {
                return Ok(());
            }

            delay.delay_ms(1);
            iterations += 1;
            if timeout_ms > 0 && iterations >= timeout_ms {
                return Err(InterfaceError::Timeout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_busy_timeout() {
        assert_eq!(DEFAULT_BUSY_TIMEOUT_MS, 30_000);
    }

    #[test]
    fn test_set_busy_timeout() {
        use embedded_hal::digital::ErrorType;
        use embedded_hal::spi::ErrorType as SpiErrorType;

        #[derive(Debug)]
        struct MockSpi;
        #[derive(Debug)]
        struct MockPin;
        #[derive(Debug, Clone, Copy)]
        struct MockError;

        impl core::fmt::Display for MockError {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "mock error")
            }
        }

        impl embedded_hal::digital::Error for MockError {
            fn kind(&self) -> embedded_hal::digital::ErrorKind {
                embedded_hal::digital::ErrorKind::Other
            }
        }

        impl embedded_hal::spi::Error for MockError {
            fn kind(&self) -> embedded_hal::spi::ErrorKind {
                embedded_hal::spi::ErrorKind::Other
            }
        }

        impl SpiErrorType for MockSpi {
            type Error = MockError;
        }

        impl SpiDevice for MockSpi {
            fn transaction(
                &mut self,
                _operations: &mut [embedded_hal::spi::Operation<'_, u8>],
            ) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        impl ErrorType for MockPin {
            type Error = MockError;
        }

        impl OutputPin for MockPin {
            fn set_low(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
            fn set_high(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        impl InputPin for MockPin {
            fn is_high(&mut self) -> Result<bool, Self::Error> {
                Ok(false)
            }
            fn is_low(&mut self) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let mut interface = Interface::new(MockSpi, MockPin, MockPin, MockPin);
        assert_eq!(interface.busy_timeout(), DEFAULT_BUSY_TIMEOUT_MS);

        interface.set_busy_timeout(5_000);
        assert_eq!(interface.busy_timeout(), 5_000);

        interface.set_busy_timeout(0);
        assert_eq!(interface.busy_timeout(), 0);
    }
}
