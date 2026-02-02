//! Error types for the driver
//!
//! This module defines error types for configuration building ([`BuilderError`])
//! and display operations ([`Error`]).
//!
//! ## Error Types
//!
//! - [`BuilderError`] - Errors during configuration construction
//! - [`Error`] - Runtime errors during display operations
//! - [`InterfaceError`](crate::interface::InterfaceError) - Low-level hardware communication errors
//!
//! ## Example
//!
//! ```
//! use ssd1677::{Builder, Dimensions, BuilderError};
//!
//! // Missing dimensions
//! let result = Builder::new().build();
//! assert!(matches!(result, Err(BuilderError::MissingDimensions)));
//!
//! // Invalid dimensions
//! let result = Dimensions::new(1000, 500); // Too large
//! assert!(result.is_err());
//! ```

use crate::interface::DisplayInterface;

/// Maximum gate outputs (rows) supported by SSD1677 controller
///
/// The SSD1677 supports up to 680 gate driver outputs.
///
/// NOTE: Some panels wire fewer gates; configure [`crate::Dimensions`] accordingly.
pub const MAX_GATE_OUTPUTS: u16 = 680;

/// Maximum source outputs (columns) supported by SSD1677 controller
///
/// The SSD1677 supports up to 960 source driver outputs.
///
/// NOTE: Some panels wire fewer sources; configure [`crate::Dimensions`] accordingly.
pub const MAX_SOURCE_OUTPUTS: u16 = 960;

/// Errors that can occur when interacting with the display
///
/// Generic over the interface type to preserve the specific error type.
/// This allows error handling code to match on the underlying hardware error.
#[derive(Debug)]
pub enum Error<I: DisplayInterface> {
    /// Interface error (SPI/GPIO)
    ///
    /// Wraps the underlying hardware error from the [`DisplayInterface`] implementation.
    Interface(I::Error),
    /// Invalid dimensions provided
    ///
    /// Dimensions must satisfy:
    /// - 1 <= rows <= MAX_GATE_OUTPUTS (680)
    /// - 8 <= cols <= MAX_SOURCE_OUTPUTS (960)
    /// - cols must be a multiple of 8
    InvalidDimensions {
        /// Number of rows (height) requested
        rows: u16,
        /// Number of columns (width) requested
        cols: u16,
    },
    /// Invalid rotation value
    ///
    /// Currently unused as rotation is type-safe via [`Rotation`](crate::config::Rotation) enum.
    InvalidRotation,
    /// Buffer is too small for the display
    ///
    /// The provided buffer must be at least `dimensions.buffer_size()` bytes.
    BufferTooSmall {
        /// Required buffer size in bytes
        required: usize,
        /// Provided buffer size in bytes
        provided: usize,
    },
    /// Invalid RAM area parameters
    ///
    /// The RAM area must have non-zero width and height, and must fit within display bounds.
    InvalidRamArea {
        /// X coordinate
        x: u16,
        /// Y coordinate
        y: u16,
        /// Width
        w: u16,
        /// Height
        h: u16,
    },
    /// Invalid LUT length
    ///
    /// SSD1677 requires exactly 112 bytes for the LUT.
    InvalidLutLength {
        /// Expected length
        expected: usize,
        /// Provided length
        provided: usize,
    },
    /// Invalid short LUT length
    ///
    /// Some panels use a shortened LUT (e.g., 105 bytes) plus separate voltage settings.
    InvalidLutShortLength {
        /// Expected length
        expected: usize,
        /// Provided length
        provided: usize,
    },
}

impl<I: DisplayInterface> core::fmt::Display for Error<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Interface(_) => write!(f, "Interface error"),
            Self::InvalidDimensions { rows, cols } => {
                write!(f, "Invalid dimensions: {rows}x{cols}")
            }
            Self::InvalidRotation => write!(f, "Invalid rotation"),
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "Buffer too small: required {required} bytes, provided {provided}"
                )
            }
            Self::InvalidRamArea { x, y, w, h } => {
                write!(f, "Invalid RAM area: x={x}, y={y}, w={w}, h={h}")
            }
            Self::InvalidLutLength { expected, provided } => {
                write!(
                    f,
                    "Invalid LUT length: expected {expected} bytes, provided {provided}"
                )
            }
            Self::InvalidLutShortLength { expected, provided } => {
                write!(
                    f,
                    "Invalid short LUT length: expected {expected} bytes, provided {provided}"
                )
            }
        }
    }
}

impl<I: DisplayInterface + core::fmt::Debug> core::error::Error for Error<I> {}

/// Errors that can occur when building configuration
///
/// These errors occur during the builder pattern before the display is created.
#[derive(Debug)]
pub enum BuilderError {
    /// Dimensions were not specified
    ///
    /// [`Builder::dimensions()`](crate::config::Builder::dimensions) must be called before building.
    MissingDimensions,
    /// Invalid dimensions provided
    ///
    /// See [`Dimensions::new()`](crate::config::Dimensions::new) for constraints.
    InvalidDimensions {
        /// Number of rows (height) requested
        rows: u16,
        /// Number of columns (width) requested
        cols: u16,
    },
}

impl core::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingDimensions => write!(f, "Dimensions must be specified"),
            Self::InvalidDimensions { rows, cols } => write!(
                f,
                "Invalid dimensions {rows}x{cols} (max {MAX_GATE_OUTPUTS}x{MAX_SOURCE_OUTPUTS}, cols must be multiple of 8)"
            ),
        }
    }
}

impl core::error::Error for BuilderError {}
