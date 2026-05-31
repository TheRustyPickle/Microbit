use embedded_hal::i2c::I2c;

use crate::{
    AUTO_INCREMENT, MAGNO_CONF_A, MAGNO_SLAVE_ADDRESS, MAGNO_X_L, MagnoAxis, SENSITIVITY,
    WHO_AM_I_M,
};

pub struct Magnetometer<I2C> {
    i2c: I2C,
}

pub enum OutputRate {
    Hz10,
    Hz20,
    Hz50,
    Hz100,
}

impl OutputRate {
    fn to_odr_value(&self) -> (u8, u8) {
        match self {
            OutputRate::Hz10 => (0, 0),
            OutputRate::Hz20 => (0, 1),
            OutputRate::Hz50 => (1, 0),
            OutputRate::Hz100 => (1, 1),
        }
    }
}

pub enum SystemMode {
    Continuous,
    Single,
    Idle,
}

impl SystemMode {
    fn to_md_value(&self) -> (u8, u8) {
        match self {
            SystemMode::Continuous => (0, 0),
            SystemMode::Single => (0, 1),
            SystemMode::Idle => (1, 1),
        }
    }
}

impl<I2C, E> Magnetometer<I2C>
where
    I2C: I2c<Error = E>,
{
    pub fn new(i2c: I2C) -> Self {
        Self { i2c }
    }

    pub fn verify_who_am_i(&mut self) -> Result<bool, E> {
        let mut rx_buf = [0u8; 1];

        self.i2c
            .write_read(MAGNO_SLAVE_ADDRESS, &[WHO_AM_I_M], &mut rx_buf)?;

        Ok(rx_buf[0] == 64)
    }

    pub fn set_config(
        mut self,
        temperature_compensation: bool,
        reboot: bool,
        soft_reset: bool,
        low_power: bool,
        odr: OutputRate,
        md: SystemMode,
    ) -> Result<Self, E> {
        let temp_comp_bit = if temperature_compensation { 1 } else { 0 };
        let reboot_bit = if reboot { 1 } else { 0 };
        let soft_reset_bit = if soft_reset { 1 } else { 0 };
        let low_power_bit = if low_power { 1 } else { 0 };
        let (odr0, odr1) = odr.to_odr_value();
        let (md0, md1) = md.to_md_value();

        let bit_value = md0
            | (md1 << 1)
            | (odr0 << 2)
            | (odr1 << 3)
            | (low_power_bit << 4)
            | (soft_reset_bit << 5)
            | (reboot_bit << 6)
            | (temp_comp_bit << 7);

        self.i2c
            .write(MAGNO_SLAVE_ADDRESS, &[MAGNO_CONF_A, bit_value])?;

        Ok(self)
    }

    pub fn get_magnometer_value(&mut self) -> Result<MagnoAxis, E> {
        let mut rx_buf = [0u8; 6];

        self.i2c.write_read(
            MAGNO_SLAVE_ADDRESS,
            &[MAGNO_X_L | AUTO_INCREMENT],
            &mut rx_buf,
        )?;

        let x_value = i16::from_le_bytes([rx_buf[0], rx_buf[1]]) as i32 * SENSITIVITY;
        let y_value = i16::from_le_bytes([rx_buf[2], rx_buf[3]]) as i32 * SENSITIVITY;
        let z_value = i16::from_le_bytes([rx_buf[4], rx_buf[5]]) as i32 * SENSITIVITY;

        let axis = MagnoAxis {
            x: x_value,
            y: y_value,
            z: z_value,
        };

        Ok(axis)
    }
}
