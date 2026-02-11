//! Core display operations

use embedded_hal::delay::DelayNs;

use crate::command::{
    AUTO_WRITE_BW_RAM, AUTO_WRITE_RED_RAM, BOOSTER_SOFT_START, BORDER_WAVEFORM, CTRL1_BYPASS_RED,
    CTRL1_NORMAL, DATA_ENTRY_MODE, DEEP_SLEEP, DISPLAY_UPDATE_CTRL1, DISPLAY_UPDATE_CTRL2,
    DRIVER_OUTPUT_CONTROL, GATE_VOLTAGE, MASTER_ACTIVATION, SET_RAM_X_COUNTER, SET_RAM_X_RANGE,
    SET_RAM_Y_COUNTER, SET_RAM_Y_RANGE, SOFT_RESET, SOURCE_VOLTAGE, TEMP_SENSOR_CONTROL, WRITE_LUT,
    WRITE_RAM_BW, WRITE_RAM_RED, WRITE_VCOM,
};
use crate::config::{Config, RamXAddressing};
use crate::error::Error;
use crate::interface::DisplayInterface;
use crate::lut::{LUT_FAST, LUT_PARTIAL};

type DisplayResult<I> = core::result::Result<(), Error<I>>;

/// Region specification for partial updates
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    /// X coordinate in pixels (should be byte-aligned, i.e., multiple of 8)
    pub x: u16,
    /// Y coordinate in pixels
    pub y: u16,
    /// Width in pixels (should be multiple of 8)
    pub w: u16,
    /// Height in pixels
    pub h: u16,
}

impl Region {
    /// Create a new region
    #[allow(clippy::many_single_char_names)]
    pub fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }

    /// Calculate the buffer size in bytes for this region
    pub fn buffer_size(&self) -> usize {
        (self.w as usize / 8) * self.h as usize
    }
}

/// Update configuration for a specific display region
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpdateRegion<'a> {
    /// Target region
    pub region: Region,
    /// Black/white buffer for the region
    pub black_buffer: &'a [u8],
    /// Red buffer for the region (empty slice disables red plane update)
    pub red_buffer: &'a [u8],
    /// Refresh mode to use for this update
    pub mode: RefreshMode,
}

/// Refresh mode for display updates
///
/// Different refresh modes trade off speed vs quality.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RefreshMode {
    /// Full refresh using OTP LUT (slowest, best quality, no ghosting)
    ///
    /// Uses the built-in waveform from the controller's OTP memory.
    /// Best for: Initial display, periodic ghost cleanup, high-quality images.
    #[default]
    Full,
    /// Partial refresh using custom LUT (~1720ms, balanced)
    ///
    /// Two-phase transitions for good contrast with faster update.
    /// Best for: Reading, page turns, moderate update frequency.
    Partial,
    /// Fast refresh using custom LUT (~300ms, fastest)
    ///
    /// Single-phase transitions for maximum speed.
    /// Best for: UI updates, scrolling, cursor movement.
    /// May have slight ghosting on high-contrast transitions.
    Fast,
}

/// Deep sleep mode configuration
///
/// Controls RAM preservation behavior when entering deep sleep.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(u8)]
pub enum DeepSleepMode {
    /// Normal deep sleep, RAM content is NOT preserved
    Normal = 0x00,
    /// Deep sleep with RAM content preserved
    #[default]
    PreserveRam = 0x01,
    /// Deep sleep with RAM and analog circuit preserved
    PreserveRamAndAnalog = 0x03,
}

/// Core display driver for SSD1677
///
/// This struct provides low-level operations for the SSD1677 controller.
/// For graphics support, use `GraphicDisplay` (requires `graphics` feature).
pub struct Display<I>
where
    I: DisplayInterface,
{
    /// Hardware interface
    interface: I,
    /// Display configuration
    config: Config,
    /// Whether the display power is on
    is_display_on: bool,
}

impl<I> Display<I>
where
    I: DisplayInterface,
{
    /// Create a new Display instance
    pub fn new(interface: I, config: Config) -> Self {
        Self {
            interface,
            config,
            is_display_on: false,
        }
    }

    /// Perform hardware reset, software reset, and initialization
    pub fn reset<D: DelayNs>(&mut self, delay: &mut D) -> DisplayResult<I> {
        self.interface.reset(delay);
        self.send_command(SOFT_RESET)?;
        self.interface.busy_wait(delay).map_err(Error::Interface)?;
        self.init(delay)
    }

    /// Initialize the controller with configuration
    fn init<D: DelayNs>(&mut self, delay: &mut D) -> DisplayResult<I> {
        // Temperature sensor
        self.send_command(TEMP_SENSOR_CONTROL)?;
        self.send_data(&[self.config.temp_sensor_control])?;

        // Booster soft start
        self.send_command(BOOSTER_SOFT_START)?;
        let booster_data = self.config.booster_soft_start;
        self.send_data(&booster_data)?;

        // Driver output control
        let rows = self.config.dimensions.rows;
        self.send_command(DRIVER_OUTPUT_CONTROL)?;
        self.send_data(&[
            ((rows - 1) % 256) as u8,
            ((rows - 1) / 256) as u8,
            self.config.gate_scanning,
        ])?;

        // Border waveform
        self.send_command(BORDER_WAVEFORM)?;
        self.send_data(&[self.config.border_waveform])?;

        // VCOM
        self.send_command(WRITE_VCOM)?;
        self.send_data(&[self.config.vcom])?;

        // Clear RAM to white
        self.clear_ram(delay)?;

        Ok(())
    }

    /// Clear display RAM to configured values
    fn clear_ram<D: DelayNs>(&mut self, delay: &mut D) -> DisplayResult<I> {
        // Clear BW RAM
        self.send_command(AUTO_WRITE_BW_RAM)?;
        self.send_data(&[self.config.clear_bw_value])?;
        self.interface.busy_wait(delay).map_err(Error::Interface)?;

        // Clear RED RAM
        self.send_command(AUTO_WRITE_RED_RAM)?;
        self.send_data(&[self.config.clear_red_value])?;
        self.interface.busy_wait(delay).map_err(Error::Interface)?;

        Ok(())
    }

    /// Update display with user-provided buffers (full refresh)
    ///
    /// # Arguments
    ///
    /// * `black_buffer` - Black/white pixel data (0=black, 1=white)
    /// * `red_buffer` - Red pixel data (0=use BW, 1=red).
    ///   Pass an empty slice or an all-zero buffer to bypass the red plane for pure B/W updates.
    /// * `delay` - Delay implementation
    pub fn update<D: DelayNs>(
        &mut self,
        black_buffer: &[u8],
        red_buffer: &[u8],
        delay: &mut D,
    ) -> DisplayResult<I> {
        self.update_with_mode(black_buffer, red_buffer, RefreshMode::Full, delay)
    }

    /// Update display with specified refresh mode
    ///
    /// # Arguments
    ///
    /// * `black_buffer` - Black/white pixel data (0=black, 1=white)
    /// * `red_buffer` - Red pixel data (0=use BW, 1=red).
    ///   Pass an empty slice or an all-zero buffer to bypass the red plane for pure B/W updates.
    /// * `mode` - Refresh mode (Full, Partial, or Fast)
    /// * `delay` - Delay implementation
    pub fn update_with_mode<D: DelayNs>(
        &mut self,
        black_buffer: &[u8],
        red_buffer: &[u8],
        mode: RefreshMode,
        delay: &mut D,
    ) -> DisplayResult<I> {
        self.update_with_mode_internal(black_buffer, red_buffer, mode, delay, true)
    }

    /// Update display with specified refresh mode without loading built-in LUTs
    ///
    /// Useful for panels that rely on OTP LUTs with specific update control values.
    pub fn update_with_mode_no_lut<D: DelayNs>(
        &mut self,
        black_buffer: &[u8],
        red_buffer: &[u8],
        mode: RefreshMode,
        delay: &mut D,
    ) -> DisplayResult<I> {
        self.update_with_mode_internal(black_buffer, red_buffer, mode, delay, false)
    }

    /// Update display with specified refresh mode and custom LUT
    ///
    /// Loads the provided LUT before refreshing, and does not overwrite it
    /// with built-in LUTs.
    pub fn update_with_custom_lut<D: DelayNs>(
        &mut self,
        black_buffer: &[u8],
        red_buffer: &[u8],
        mode: RefreshMode,
        lut: &[u8],
        delay: &mut D,
    ) -> DisplayResult<I> {
        self.load_lut(lut)?;
        self.update_with_mode_internal(black_buffer, red_buffer, mode, delay, false)
    }

    /// Update a specific region of the display
    pub fn update_region<D: DelayNs>(
        &mut self,
        update: UpdateRegion<'_>,
        delay: &mut D,
    ) -> DisplayResult<I> {
        self.update_region_internal(update, delay, true)
    }

    /// Update a specific region without loading built-in LUTs
    ///
    /// Useful for panels that rely on OTP LUTs with specific update control values.
    pub fn update_region_no_lut<D: DelayNs>(
        &mut self,
        update: UpdateRegion<'_>,
        delay: &mut D,
    ) -> DisplayResult<I> {
        self.update_region_internal(update, delay, false)
    }

    /// Update a specific region of the display using a custom LUT
    ///
    /// Loads the provided LUT before refreshing, and does not overwrite it
    /// with built-in LUTs.
    pub fn update_region_with_custom_lut<D: DelayNs>(
        &mut self,
        update: UpdateRegion<'_>,
        lut: &[u8],
        delay: &mut D,
    ) -> DisplayResult<I> {
        self.load_lut(lut)?;
        self.update_region_internal(update, delay, false)
    }

    /// Full refresh with all pixels
    pub fn full_refresh<D: DelayNs>(&mut self, delay: &mut D) -> DisplayResult<I> {
        self.refresh_with_mode(RefreshMode::Full, delay, false, false)
    }

    /// Fast refresh (uses fast LUT, ~300ms)
    pub fn fast_refresh<D: DelayNs>(&mut self, delay: &mut D) -> DisplayResult<I> {
        self.refresh_with_mode(RefreshMode::Fast, delay, false, false)
    }

    /// Refresh the display with a specific mode
    fn refresh_with_mode<D: DelayNs>(
        &mut self,
        mode: RefreshMode,
        delay: &mut D,
        turn_off: bool,
        use_red: bool,
    ) -> DisplayResult<I> {
        self.send_command(DISPLAY_UPDATE_CTRL1)?;
        let ctrl1 = if use_red {
            CTRL1_NORMAL
        } else {
            CTRL1_BYPASS_RED
        };
        self.send_data(&[ctrl1])?;

        let mut display_mode: u8 = match mode {
            RefreshMode::Full => self.config.display_update_ctrl2_full,
            RefreshMode::Partial => self.config.display_update_ctrl2_partial,
            RefreshMode::Fast => self.config.display_update_ctrl2_fast,
        };

        if !self.is_display_on {
            display_mode |= self.config.display_update_power_on;
        }

        if turn_off {
            display_mode |= self.config.display_update_power_off;
            self.is_display_on = false;
        } else {
            self.is_display_on = true;
        }

        self.send_command(DISPLAY_UPDATE_CTRL2)?;
        self.send_data(&[display_mode])?;

        self.send_command(MASTER_ACTIVATION)?;

        self.interface.busy_wait(delay).map_err(Error::Interface)?;

        Ok(())
    }

    /// Enter deep sleep mode
    ///
    /// # Arguments
    ///
    /// * `delay` - Delay implementation for busy-waiting
    /// * `preserve_ram` - RAM preservation mode:
    ///   - `DeepSleepMode::Normal` (0x00): RAM content is not preserved
    ///   - `DeepSleepMode::PreserveRam` (0x01): RAM content is preserved
    ///   - `DeepSleepMode::PreserveRamAndAnalog` (0x03): RAM and analog are preserved
    pub fn deep_sleep<D: DelayNs>(
        &mut self,
        delay: &mut D,
        mode: DeepSleepMode,
    ) -> DisplayResult<I> {
        if self.is_display_on {
            // Power down first
            self.send_command(DISPLAY_UPDATE_CTRL1)?;
            self.send_data(&[CTRL1_BYPASS_RED])?;

            self.send_command(DISPLAY_UPDATE_CTRL2)?;
            self.send_data(&[0x03])?; // Power down

            self.send_command(MASTER_ACTIVATION)?;
            self.interface.busy_wait(delay).map_err(Error::Interface)?;

            self.is_display_on = false;
        }

        // Enter deep sleep
        self.send_command(DEEP_SLEEP)?;
        self.send_data(&[mode as u8])?;

        Ok(())
    }

    /// LUT size required by SSD1677 controller
    pub const LUT_SIZE: usize = 112;
    /// Short LUT size used by some panels (requires separate voltage settings)
    pub const LUT_SHORT_SIZE: usize = 105;

    /// Load custom LUT (112 bytes for SSD1677)
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidLutLength` if the LUT is not exactly 112 bytes.
    pub fn load_lut(&mut self, lut: &[u8]) -> DisplayResult<I> {
        if lut.len() != Self::LUT_SIZE {
            return Err(Error::InvalidLutLength {
                expected: Self::LUT_SIZE,
                provided: lut.len(),
            });
        }
        self.send_command(WRITE_LUT)?;
        self.send_data(lut)?;

        Ok(())
    }

    /// Load a shortened LUT and set voltage registers separately
    ///
    /// This supports panels that use a 105-byte LUT plus separate gate/source/VCOM settings.
    pub fn load_lut_with_voltages(
        &mut self,
        lut: &[u8],
        gate_voltage: u8,
        source_voltage: [u8; 3],
        vcom: u8,
    ) -> DisplayResult<I> {
        if lut.len() != Self::LUT_SHORT_SIZE {
            return Err(Error::InvalidLutShortLength {
                expected: Self::LUT_SHORT_SIZE,
                provided: lut.len(),
            });
        }
        self.send_command(WRITE_LUT)?;
        self.send_data(lut)?;
        self.set_gate_voltage(gate_voltage)?;
        self.set_source_voltage(source_voltage)?;
        self.set_vcom(vcom)?;
        Ok(())
    }

    /// Set gate driving voltage (VGH)
    pub fn set_gate_voltage(&mut self, voltage: u8) -> DisplayResult<I> {
        self.send_command(GATE_VOLTAGE)?;
        self.send_data(&[voltage])?;
        Ok(())
    }

    /// Set source driving voltages (VSH1, VSH2, VSL)
    pub fn set_source_voltage(&mut self, voltages: [u8; 3]) -> DisplayResult<I> {
        self.send_command(SOURCE_VOLTAGE)?;
        self.send_data(&voltages)?;
        Ok(())
    }

    /// Set VCOM voltage
    pub fn set_vcom(&mut self, vcom: u8) -> DisplayResult<I> {
        self.send_command(WRITE_VCOM)?;
        self.send_data(&[vcom])?;
        Ok(())
    }

    /// Set RAM area for partial updates
    ///
    /// Coordinates are specified in pixels. X and width must be byte-aligned
    /// (multiples of 8) because RAM writes are byte-packed.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidRamArea` if:
    /// - w == 0 or h == 0 (would cause underflow)
    /// - x + w > cols or y + h > rows (out of bounds)
    #[allow(clippy::many_single_char_names)]
    fn set_ram_area(&mut self, x: u16, y: u16, w: u16, h: u16) -> DisplayResult<I> {
        if w == 0 || h == 0 {
            return Err(Error::InvalidRamArea { x, y, w, h });
        }
        if x.saturating_add(w) > self.config.dimensions.cols
            || y.saturating_add(h) > self.config.dimensions.rows
        {
            return Err(Error::InvalidRamArea { x, y, w, h });
        }
        if x % 8 != 0 || w % 8 != 0 {
            return Err(Error::InvalidRamArea { x, y, w, h });
        }

        self.send_command(DATA_ENTRY_MODE)?;
        self.send_data(&[self.config.data_entry_mode])?;

        let id0 = (self.config.data_entry_mode & 0x01) != 0;
        let id1 = (self.config.data_entry_mode & 0x02) != 0;

        let (x_start_raw, x_end_raw) = match self.config.ram_x_addressing {
            RamXAddressing::Pixels => (x, x + w - 1),
            RamXAddressing::Bytes => (x / 8, (x + w - 1) / 8),
        };
        let (x_start, x_end) = if id0 {
            (x_start_raw, x_end_raw)
        } else {
            (x_end_raw, x_start_raw)
        };
        self.send_command(SET_RAM_X_RANGE)?;
        self.send_data(&[
            (x_start % 256) as u8,
            (x_start / 256) as u8,
            (x_end % 256) as u8,
            (x_end / 256) as u8,
        ])?;

        // Y range (optional inversion, pixel units)
        let y_base = if self.config.ram_y_inverted {
            self.config.dimensions.rows - y - h
        } else {
            y
        };
        let y_start_raw = y_base;
        let y_end_raw = y_base + h - 1;
        let (y_start, y_end) = if id1 {
            (y_start_raw, y_end_raw)
        } else {
            (y_end_raw, y_start_raw)
        };

        self.send_command(SET_RAM_Y_RANGE)?;
        self.send_data(&[
            (y_start % 256) as u8,
            (y_start / 256) as u8,
            (y_end % 256) as u8,
            (y_end / 256) as u8,
        ])?;

        // Set counters
        self.send_command(SET_RAM_X_COUNTER)?;
        self.send_data(&[(x_start % 256) as u8, (x_start / 256) as u8])?;

        self.send_command(SET_RAM_Y_COUNTER)?;
        self.send_data(&[(y_start % 256) as u8, (y_start / 256) as u8])?;

        Ok(())
    }

    /// Send a command to the display controller
    fn send_command(&mut self, cmd: u8) -> DisplayResult<I> {
        self.interface.send_command(cmd).map_err(Error::Interface)
    }

    /// Send data to the display controller
    fn send_data(&mut self, data: &[u8]) -> DisplayResult<I> {
        self.interface.send_data(data).map_err(Error::Interface)
    }

    /// Get display dimensions
    pub fn dimensions(&self) -> &crate::config::Dimensions {
        &self.config.dimensions
    }

    /// Get display rotation
    pub fn rotation(&self) -> crate::config::Rotation {
        self.config.rotation
    }

    /// Access the underlying configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    fn update_with_mode_internal<D: DelayNs>(
        &mut self,
        black_buffer: &[u8],
        red_buffer: &[u8],
        mode: RefreshMode,
        delay: &mut D,
        use_builtin_lut: bool,
    ) -> DisplayResult<I> {
        let explicit_red = !red_buffer.is_empty() && red_buffer.iter().any(|byte| *byte != 0);
        let single_buffer_fast = mode == RefreshMode::Fast && !explicit_red;
        // Keep RED RAM in sync for the next differential fast refresh even when the caller
        // only provides a BW buffer.
        let sync_red_before_refresh = mode != RefreshMode::Fast && !explicit_red;
        let use_red_for_refresh = explicit_red || single_buffer_fast;
        let expected_size = self.config.dimensions.buffer_size();

        if black_buffer.len() < expected_size {
            return Err(Error::BufferTooSmall {
                required: expected_size,
                provided: black_buffer.len(),
            });
        }
        if explicit_red && red_buffer.len() < expected_size {
            return Err(Error::BufferTooSmall {
                required: expected_size,
                provided: red_buffer.len(),
            });
        }

        if use_builtin_lut {
            match mode {
                RefreshMode::Full => {}
                RefreshMode::Partial => self.load_lut(&LUT_PARTIAL)?,
                RefreshMode::Fast => self.load_lut(&LUT_FAST)?,
            }
        }

        self.set_ram_area(
            0,
            0,
            self.config.dimensions.cols,
            self.config.dimensions.rows,
        )?;

        self.send_command(WRITE_RAM_BW)?;
        self.send_data(&black_buffer[..expected_size])?;

        if explicit_red {
            self.send_command(WRITE_RAM_RED)?;
            self.send_data(&red_buffer[..expected_size])?;
        } else if sync_red_before_refresh {
            self.send_command(WRITE_RAM_RED)?;
            self.send_data(&black_buffer[..expected_size])?;
        }

        self.refresh_with_mode(mode, delay, false, use_red_for_refresh)?;

        if single_buffer_fast {
            self.set_ram_area(
                0,
                0,
                self.config.dimensions.cols,
                self.config.dimensions.rows,
            )?;
            self.send_command(WRITE_RAM_RED)?;
            self.send_data(&black_buffer[..expected_size])?;
        }

        Ok(())
    }

    fn update_region_internal<D: DelayNs>(
        &mut self,
        update: UpdateRegion<'_>,
        delay: &mut D,
        use_builtin_lut: bool,
    ) -> DisplayResult<I> {
        let explicit_red =
            !update.red_buffer.is_empty() && update.red_buffer.iter().any(|byte| *byte != 0);
        let single_buffer_fast = update.mode == RefreshMode::Fast && !explicit_red;
        let sync_red_before_refresh = update.mode != RefreshMode::Fast && !explicit_red;
        let use_red_for_refresh = explicit_red || single_buffer_fast;
        let expected_size = update.region.buffer_size();

        if update.black_buffer.len() < expected_size {
            return Err(Error::BufferTooSmall {
                required: expected_size,
                provided: update.black_buffer.len(),
            });
        }
        if explicit_red && update.red_buffer.len() < expected_size {
            return Err(Error::BufferTooSmall {
                required: expected_size,
                provided: update.red_buffer.len(),
            });
        }

        if use_builtin_lut {
            match update.mode {
                RefreshMode::Full => {}
                RefreshMode::Partial => self.load_lut(&LUT_PARTIAL)?,
                RefreshMode::Fast => self.load_lut(&LUT_FAST)?,
            }
        }

        self.set_ram_area(
            update.region.x,
            update.region.y,
            update.region.w,
            update.region.h,
        )?;

        self.send_command(WRITE_RAM_BW)?;
        self.send_data(&update.black_buffer[..expected_size])?;

        if explicit_red {
            self.send_command(WRITE_RAM_RED)?;
            self.send_data(&update.red_buffer[..expected_size])?;
        } else if sync_red_before_refresh {
            self.send_command(WRITE_RAM_RED)?;
            self.send_data(&update.black_buffer[..expected_size])?;
        }

        self.refresh_with_mode(update.mode, delay, false, use_red_for_refresh)?;

        if single_buffer_fast {
            self.set_ram_area(
                update.region.x,
                update.region.y,
                update.region.w,
                update.region.h,
            )?;
            self.send_command(WRITE_RAM_RED)?;
            self.send_data(&update.black_buffer[..expected_size])?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Builder, Dimensions};

    #[derive(Debug)]
    struct MockInterface {
        commands: alloc::vec::Vec<u8>,
        data: alloc::vec::Vec<alloc::vec::Vec<u8>>,
        command_data: alloc::vec::Vec<(u8, alloc::vec::Vec<u8>)>,
        last_command: Option<u8>,
    }

    impl MockInterface {
        fn new() -> Self {
            Self {
                commands: alloc::vec::Vec::new(),
                data: alloc::vec::Vec::new(),
                command_data: alloc::vec::Vec::new(),
                last_command: None,
            }
        }
    }

    impl DisplayInterface for MockInterface {
        type Error = core::convert::Infallible;

        fn send_command(&mut self, command: u8) -> Result<(), Self::Error> {
            self.commands.push(command);
            self.last_command = Some(command);
            Ok(())
        }

        fn send_data(&mut self, data: &[u8]) -> Result<(), Self::Error> {
            self.data.push(data.to_vec());
            if let Some(cmd) = self.last_command {
                self.command_data.push((cmd, data.to_vec()));
            }
            Ok(())
        }

        fn reset<D: DelayNs>(&mut self, _delay: &mut D) {}

        fn busy_wait<D: DelayNs>(&mut self, _delay: &mut D) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    struct MockDelay;
    impl DelayNs for MockDelay {
        fn delay_ns(&mut self, _ns: u32) {}
    }

    fn test_display() -> Display<MockInterface> {
        let interface = MockInterface::new();
        let config = Builder::new()
            .dimensions(Dimensions::new(480, 480).unwrap())
            .build()
            .unwrap();
        Display::new(interface, config)
    }

    #[test]
    fn test_set_ram_area_zero_width_returns_error() {
        let mut display = test_display();
        let result = display.set_ram_area(0, 0, 0, 100);
        assert!(matches!(result, Err(Error::InvalidRamArea { w: 0, .. })));
    }

    #[test]
    fn test_set_ram_area_zero_height_returns_error() {
        let mut display = test_display();
        let result = display.set_ram_area(0, 0, 100, 0);
        assert!(matches!(result, Err(Error::InvalidRamArea { h: 0, .. })));
    }

    #[test]
    fn test_set_ram_area_out_of_bounds_x_returns_error() {
        let mut display = test_display();
        let result = display.set_ram_area(400, 0, 100, 100);
        assert!(matches!(result, Err(Error::InvalidRamArea { .. })));
    }

    #[test]
    fn test_set_ram_area_out_of_bounds_y_returns_error() {
        let mut display = test_display();
        let result = display.set_ram_area(0, 400, 100, 100);
        assert!(matches!(result, Err(Error::InvalidRamArea { .. })));
    }

    #[test]
    fn test_set_ram_area_valid_succeeds() {
        let mut display = test_display();
        let result = display.set_ram_area(0, 0, 480, 480);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_lut_wrong_length_returns_error() {
        let mut display = test_display();
        let short_lut = [0u8; 50];
        let result = display.load_lut(&short_lut);
        assert!(matches!(
            result,
            Err(Error::InvalidLutLength {
                expected: 112,
                provided: 50
            })
        ));
    }

    #[test]
    fn test_load_lut_too_long_returns_error() {
        let mut display = test_display();
        let long_lut = [0u8; 200];
        let result = display.load_lut(&long_lut);
        assert!(matches!(
            result,
            Err(Error::InvalidLutLength {
                expected: 112,
                provided: 200
            })
        ));
    }

    #[test]
    fn test_load_lut_correct_length_succeeds() {
        let mut display = test_display();
        let lut = [0u8; 112];
        let result = display.load_lut(&lut);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deep_sleep_mode_normal() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let result = display.deep_sleep(&mut delay, DeepSleepMode::Normal);
        assert!(result.is_ok());
        let last_data = display.interface.data.last().unwrap();
        assert_eq!(last_data, &[0x00]);
    }

    #[test]
    fn test_deep_sleep_mode_preserve_ram() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let result = display.deep_sleep(&mut delay, DeepSleepMode::PreserveRam);
        assert!(result.is_ok());
        let last_data = display.interface.data.last().unwrap();
        assert_eq!(last_data, &[0x01]);
    }

    #[test]
    fn test_deep_sleep_mode_preserve_ram_and_analog() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let result = display.deep_sleep(&mut delay, DeepSleepMode::PreserveRamAndAnalog);
        assert!(result.is_ok());
        let last_data = display.interface.data.last().unwrap();
        assert_eq!(last_data, &[0x03]);
    }

    #[test]
    fn test_update_with_mode_full() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let buffer_size = display.dimensions().buffer_size();
        let black_buf = alloc::vec![0xFFu8; buffer_size];
        let red_buf = alloc::vec![0x00u8; buffer_size];
        let result = display.update_with_mode(&black_buf, &red_buf, RefreshMode::Full, &mut delay);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_with_mode_fast() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let buffer_size = display.dimensions().buffer_size();
        let black_buf = alloc::vec![0xFFu8; buffer_size];
        let red_buf = alloc::vec![0x00u8; buffer_size];
        let result = display.update_with_mode(&black_buf, &red_buf, RefreshMode::Fast, &mut delay);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_with_mode_fast_empty_red_uses_differential_compare() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let buffer_size = display.dimensions().buffer_size();
        let black_buf = alloc::vec![0xAAu8; buffer_size];
        let red_buf = alloc::vec![0x00u8; buffer_size];

        let result = display.update_with_mode(&black_buf, &red_buf, RefreshMode::Fast, &mut delay);
        assert!(result.is_ok());

        let ctrl1 = display
            .interface
            .command_data
            .iter()
            .rev()
            .find(|(cmd, _)| *cmd == DISPLAY_UPDATE_CTRL1)
            .map(|(_, data)| data.clone());
        assert_eq!(ctrl1, Some(alloc::vec![CTRL1_NORMAL]));
    }

    #[test]
    fn test_update_with_mode_all_zero_red_bypasses_red_plane() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let buffer_size = display.dimensions().buffer_size();
        let black_buf = alloc::vec![0xFFu8; buffer_size];
        let red_buf = alloc::vec![0x00u8; buffer_size];
        let result = display.update_with_mode(&black_buf, &red_buf, RefreshMode::Full, &mut delay);
        assert!(result.is_ok());

        let ctrl1 = display
            .interface
            .command_data
            .iter()
            .rev()
            .find(|(cmd, _)| *cmd == DISPLAY_UPDATE_CTRL1)
            .map(|(_, data)| data.clone());

        assert_eq!(ctrl1, Some(alloc::vec![CTRL1_BYPASS_RED]));
    }

    #[test]
    fn test_update_with_mode_full_syncs_red_ram_when_red_is_empty() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let buffer_size = display.dimensions().buffer_size();
        let black_buf = alloc::vec![0xA5u8; buffer_size];
        let red_buf = alloc::vec![0x00u8; buffer_size];

        let result = display.update_with_mode(&black_buf, &red_buf, RefreshMode::Full, &mut delay);
        assert!(result.is_ok());

        let wrote_synced_red = display.interface.command_data.iter().any(|(cmd, data)| {
            *cmd == WRITE_RAM_RED
                && data.len() == black_buf.len()
                && data.first() == black_buf.first()
                && data.last() == black_buf.last()
        });

        assert!(wrote_synced_red);
    }

    #[test]
    fn test_update_with_mode_nonzero_red_uses_red_plane() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let buffer_size = display.dimensions().buffer_size();
        let black_buf = alloc::vec![0xFFu8; buffer_size];
        let mut red_buf = alloc::vec![0x00u8; buffer_size];
        red_buf[0] = 0x01;

        let result = display.update_with_mode(&black_buf, &red_buf, RefreshMode::Full, &mut delay);
        assert!(result.is_ok());

        let ctrl1 = display
            .interface
            .command_data
            .iter()
            .rev()
            .find(|(cmd, _)| *cmd == DISPLAY_UPDATE_CTRL1)
            .map(|(_, data)| data.clone());

        assert_eq!(ctrl1, Some(alloc::vec![CTRL1_NORMAL]));
    }

    #[test]
    fn test_update_with_mode_partial() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let buffer_size = display.dimensions().buffer_size();
        let black_buf = alloc::vec![0xFFu8; buffer_size];
        let red_buf = alloc::vec![0x00u8; buffer_size];
        let result =
            display.update_with_mode(&black_buf, &red_buf, RefreshMode::Partial, &mut delay);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_region_valid() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let region_size = (80 / 8) * 80; // 80x80 region
        let black_buf = alloc::vec![0xFFu8; region_size];
        let red_buf = alloc::vec![0x00u8; region_size];
        let result = display.update_region(
            UpdateRegion {
                region: Region::new(0, 0, 80, 80),
                black_buffer: &black_buf,
                red_buffer: &red_buf,
                mode: RefreshMode::Fast,
            },
            &mut delay,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_region_out_of_bounds() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let region_size = (80 / 8) * 80;
        let black_buf = alloc::vec![0xFFu8; region_size];
        let red_buf = alloc::vec![0x00u8; region_size];
        let result = display.update_region(
            UpdateRegion {
                region: Region::new(450, 0, 80, 80),
                black_buffer: &black_buf,
                red_buffer: &red_buf,
                mode: RefreshMode::Fast,
            },
            &mut delay,
        );
        assert!(matches!(result, Err(Error::InvalidRamArea { .. })));
    }

    #[test]
    fn test_update_region_buffer_too_small() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let black_buf = alloc::vec![0xFFu8; 10]; // Too small for 80x80
        let red_buf = alloc::vec![0x00u8; 10];
        let result = display.update_region(
            UpdateRegion {
                region: Region::new(0, 0, 80, 80),
                black_buffer: &black_buf,
                red_buffer: &red_buf,
                mode: RefreshMode::Fast,
            },
            &mut delay,
        );
        assert!(matches!(result, Err(Error::BufferTooSmall { .. })));
    }

    #[test]
    fn test_fast_refresh() {
        let mut display = test_display();
        let mut delay = MockDelay;
        let result = display.fast_refresh(&mut delay);
        assert!(result.is_ok());
    }

    #[test]
    fn test_refresh_mode_default_is_full() {
        assert_eq!(RefreshMode::default(), RefreshMode::Full);
    }
}
