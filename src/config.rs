//! Display configuration types and builder

pub use crate::error::{BuilderError, MAX_GATE_OUTPUTS, MAX_SOURCE_OUTPUTS};

/// Display dimensions
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dimensions {
    /// Number of rows (height in pixels, corresponds to gate outputs)
    pub rows: u16,
    /// Number of columns (width in pixels, corresponds to source outputs)
    pub cols: u16,
}

impl Dimensions {
    /// Create new dimensions with validation
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::InvalidDimensions` if:
    /// - rows > MAX_GATE_OUTPUTS
    /// - cols > MAX_SOURCE_OUTPUTS
    /// - cols % 8 != 0 (must be byte-aligned for memory)
    pub fn new(rows: u16, cols: u16) -> Result<Self, BuilderError> {
        if rows == 0 || rows > MAX_GATE_OUTPUTS {
            return Err(BuilderError::InvalidDimensions { rows, cols });
        }
        if cols == 0 || cols > MAX_SOURCE_OUTPUTS || !cols.is_multiple_of(8) {
            return Err(BuilderError::InvalidDimensions { rows, cols });
        }
        Ok(Self { rows, cols })
    }

    /// Calculate required buffer size in bytes
    pub fn buffer_size(&self) -> usize {
        (self.rows as usize * self.cols as usize) / 8
    }
}

/// Display rotation relative to native orientation
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Rotation {
    /// No rotation
    #[default]
    Rotate0,
    /// Rotate 90 degrees clockwise
    Rotate90,
    /// Rotate 180 degrees
    Rotate180,
    /// Rotate 270 degrees clockwise
    Rotate270,
}

/// RAM X address unit
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RamXAddressing {
    /// X address is in pixels
    #[default]
    Pixels,
    /// X address is in bytes (pixel / 8)
    Bytes,
}

/// Display configuration
///
/// This struct holds all configurable parameters for the SSD1677 controller.
/// Use `Builder` to create a Config.
#[derive(Clone, Debug)]
pub struct Config {
    /// Display dimensions
    pub dimensions: Dimensions,
    /// Display rotation
    pub rotation: Rotation,
    /// Booster soft-start settings (5 bytes for command 0x0C)
    pub booster_soft_start: [u8; 5],
    /// Gate scanning direction byte
    pub gate_scanning: u8,
    /// Border waveform setting
    pub border_waveform: u8,
    /// VCOM register value
    pub vcom: u8,
    /// Data entry mode byte
    pub data_entry_mode: u8,
    /// RAM X address unit (pixel or byte addressing)
    pub ram_x_addressing: RamXAddressing,
    /// Whether RAM Y coordinates are inverted (panel wiring dependent)
    pub ram_y_inverted: bool,
    /// Display Update Control 2 value for full refresh
    pub display_update_ctrl2_full: u8,
    /// Display Update Control 2 value for partial refresh
    pub display_update_ctrl2_partial: u8,
    /// Display Update Control 2 value for fast refresh
    pub display_update_ctrl2_fast: u8,
    /// Bits to OR in when powering on the display
    pub display_update_power_on: u8,
    /// Bits to OR in when powering off the display
    pub display_update_power_off: u8,
    /// Fill value used to clear the BW RAM
    pub clear_bw_value: u8,
    /// Fill value used to clear the RED RAM
    pub clear_red_value: u8,
    /// Temperature sensor control
    pub temp_sensor_control: u8,
}

impl Config {
    /// Get the rotated dimensions based on rotation setting
    pub fn rotated_dimensions(&self) -> Dimensions {
        match self.rotation {
            Rotation::Rotate0 | Rotation::Rotate180 => self.dimensions,
            Rotation::Rotate90 | Rotation::Rotate270 => Dimensions {
                rows: self.dimensions.cols,
                cols: self.dimensions.rows,
            },
        }
    }
}

/// Builder for constructing display configuration
///
/// # Example
///
/// ```rust,no_run
/// use ssd1677::{Builder, Dimensions, Rotation};
///
/// let dims = match Dimensions::new(480, 480) {
///     Ok(dims) => dims,
///     Err(_) => return,
/// };
/// let config = match Builder::new().dimensions(dims).rotation(Rotation::Rotate0).build() {
///     Ok(config) => config,
///     Err(_) => return,
/// };
/// let _ = config;
/// ```
#[must_use]
pub struct Builder {
    /// Display dimensions (required)
    dimensions: Option<Dimensions>,
    /// Display rotation
    rotation: Rotation,
    /// Booster soft-start settings (5 bytes for command 0x0C)
    booster_soft_start: [u8; 5],
    /// Gate scanning direction byte
    gate_scanning: u8,
    /// Border waveform setting
    border_waveform: u8,
    /// VCOM register value
    vcom: u8,
    /// Data entry mode byte
    data_entry_mode: u8,
    /// RAM X address unit (pixel or byte addressing)
    ram_x_addressing: RamXAddressing,
    /// Whether RAM Y coordinates are inverted (panel wiring dependent)
    ram_y_inverted: bool,
    /// Display Update Control 2 value for full refresh
    display_update_ctrl2_full: u8,
    /// Display Update Control 2 value for partial refresh
    display_update_ctrl2_partial: u8,
    /// Display Update Control 2 value for fast refresh
    display_update_ctrl2_fast: u8,
    /// Bits to OR in when powering on the display
    display_update_power_on: u8,
    /// Bits to OR in when powering off the display
    display_update_power_off: u8,
    /// Fill value used to clear the BW RAM
    clear_bw_value: u8,
    /// Fill value used to clear the RED RAM
    clear_red_value: u8,
    /// Temperature sensor control
    temp_sensor_control: u8,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            dimensions: None,
            rotation: Rotation::Rotate0,
            // Default booster soft-start sequence (panel-specific, override as needed)
            booster_soft_start: [0xAE, 0xC7, 0xC3, 0xC0, 0x40],
            // Default gate scanning (panel-specific, override as needed)
            gate_scanning: 0x02,
            // Default border waveform
            border_waveform: 0x01,
            // Default VCOM
            vcom: 0x3C,
            // Default: X increment, Y decrement (common for many panels)
            data_entry_mode: 0x01,
            // Default: X address in pixels (panel-specific)
            ram_x_addressing: RamXAddressing::Pixels,
            // Default: no Y inversion (panel-specific)
            ram_y_inverted: false,
            // Default display update control values (from datasheet examples)
            display_update_ctrl2_full: 0xF7,
            display_update_ctrl2_partial: 0xC7,
            display_update_ctrl2_fast: 0xC7,
            // Default power on/off bits (enable/disable clock+analog)
            display_update_power_on: 0xC0,
            display_update_power_off: 0x03,
            // Default clear values (white for BW, no red)
            clear_bw_value: 0xFF,
            clear_red_value: 0x00,
            // Default: internal temperature sensor
            temp_sensor_control: 0x80,
        }
    }
}

impl Builder {
    /// Create a new Builder with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set display dimensions (required)
    pub fn dimensions(mut self, dims: Dimensions) -> Self {
        self.dimensions = Some(dims);
        self
    }

    /// Set display rotation
    pub fn rotation(mut self, rotation: Rotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set booster soft-start parameters
    pub fn booster_soft_start(mut self, values: [u8; 5]) -> Self {
        self.booster_soft_start = values;
        self
    }

    /// Set gate scanning direction
    pub fn gate_scanning(mut self, value: u8) -> Self {
        self.gate_scanning = value;
        self
    }

    /// Set border waveform
    pub fn border_waveform(mut self, value: u8) -> Self {
        self.border_waveform = value;
        self
    }

    /// Set VCOM value
    pub fn vcom(mut self, value: u8) -> Self {
        self.vcom = value;
        self
    }

    /// Set data entry mode
    pub fn data_entry_mode(mut self, value: u8) -> Self {
        self.data_entry_mode = value;
        self
    }

    /// Set the RAM X address unit (pixels or bytes)
    pub fn ram_x_addressing(mut self, value: RamXAddressing) -> Self {
        self.ram_x_addressing = value;
        self
    }

    /// Set whether RAM Y coordinates are inverted
    ///
    /// Some panels wire gate scanning in reverse; set to true to invert Y.
    pub fn ram_y_inverted(mut self, value: bool) -> Self {
        self.ram_y_inverted = value;
        self
    }

    /// Set Display Update Control 2 value for full refresh
    pub fn display_update_ctrl2_full(mut self, value: u8) -> Self {
        self.display_update_ctrl2_full = value;
        self
    }

    /// Set Display Update Control 2 value for partial refresh
    pub fn display_update_ctrl2_partial(mut self, value: u8) -> Self {
        self.display_update_ctrl2_partial = value;
        self
    }

    /// Set Display Update Control 2 value for fast refresh
    pub fn display_update_ctrl2_fast(mut self, value: u8) -> Self {
        self.display_update_ctrl2_fast = value;
        self
    }

    /// Set bits to OR in when powering on the display
    pub fn display_update_power_on(mut self, value: u8) -> Self {
        self.display_update_power_on = value;
        self
    }

    /// Set bits to OR in when powering off the display
    pub fn display_update_power_off(mut self, value: u8) -> Self {
        self.display_update_power_off = value;
        self
    }

    /// Set the fill value used to clear the BW RAM
    pub fn clear_bw_value(mut self, value: u8) -> Self {
        self.clear_bw_value = value;
        self
    }

    /// Set the fill value used to clear the RED RAM
    pub fn clear_red_value(mut self, value: u8) -> Self {
        self.clear_red_value = value;
        self
    }

    /// Set temperature sensor control
    pub fn temp_sensor_control(mut self, value: u8) -> Self {
        self.temp_sensor_control = value;
        self
    }

    /// Build the configuration
    ///
    /// # Errors
    ///
    /// Returns `BuilderError::MissingDimensions` if dimensions were not set
    pub fn build(self) -> Result<Config, BuilderError> {
        Ok(Config {
            dimensions: self.dimensions.ok_or(BuilderError::MissingDimensions)?,
            rotation: self.rotation,
            booster_soft_start: self.booster_soft_start,
            gate_scanning: self.gate_scanning,
            border_waveform: self.border_waveform,
            vcom: self.vcom,
            data_entry_mode: self.data_entry_mode,
            ram_x_addressing: self.ram_x_addressing,
            ram_y_inverted: self.ram_y_inverted,
            display_update_ctrl2_full: self.display_update_ctrl2_full,
            display_update_ctrl2_partial: self.display_update_ctrl2_partial,
            display_update_ctrl2_fast: self.display_update_ctrl2_fast,
            display_update_power_on: self.display_update_power_on,
            display_update_power_off: self.display_update_power_off,
            clear_bw_value: self.clear_bw_value,
            clear_red_value: self.clear_red_value,
            temp_sensor_control: self.temp_sensor_control,
        })
    }
}
