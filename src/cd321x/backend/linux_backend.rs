/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */
use std::fs;
use i2cdev::core::I2CDevice;
use i2cdev::linux::LinuxI2CDevice;
use log::{error, info};
use crate::{Result, Error};
use crate::cd321x::backend::ControllerBackend;

pub(crate) struct LinuxBackend {
    i2c: LinuxI2CDevice,
}

impl LinuxBackend {

    pub(crate) fn new(bus: &str, slave_address: u16) -> Result<Self> {
        let i2c = Self::verify_i2c_device(bus, slave_address)?;
        Ok(Self { i2c: i2c })
    }

    /// Tries to open the given I2C bus and slave address.
    /// Returns a configured LinuxI2CDevice on success.
    pub(crate) fn verify_i2c_device(bus: &str, slave_address: u16) -> Result<LinuxI2CDevice> {

        match LinuxI2CDevice::new(bus, slave_address) {
            Ok(dev) => {
                return Ok(dev);
            }
            Err(_) => {} // Fall through to attempt forced open
        }

        info!("Safely opening failed ==> Forcefully opening device...");
        let forced = unsafe { LinuxI2CDevice::force_new(bus, slave_address) };
        match forced {
            Ok(dev) => Ok(dev),
            Err(_) => { Err(Error::I2CError) }
        }
    }

    fn get_host_info() -> Result<String> {
        let compat: Vec<u8> = fs::read("/proc/device-tree/compatible").map_err(Error::Io)?;
        let compat = std::str::from_utf8(&compat[0..10]).map_err(Error::Utf8)?;
        Ok(compat.to_string())
    }
}

impl ControllerBackend for LinuxBackend {
    fn verify_host(&self) -> Result<()> {
        let compat = Self::get_host_info()?;
        let (manufacturer, _) =  compat.split_once(",").ok_or(Error::Compatible)?;
        if manufacturer != "apple" {
            error!("Host is not an Apple silicon system: \"{compat}\"");
            return Err(Error::Compatible);
        }
        Ok(())
    }
    
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_unlock_key(&mut self) -> Result<Vec<u8>> {
        let compat = Self::get_host_info()?;
        let (_, device) =  compat.split_once(",").ok_or(Error::Compatible)?;
        Ok(device.to_uppercase().into_bytes())
    }

    fn deinit_device(&mut self) { }

    fn read(&mut self, data_addr: u8, buffer: &mut [u8]) -> Result<()> {
        self.i2c.write(&[data_addr]).map_err(|_| Error::I2CError)?;
        self.i2c.read(buffer).map_err(|_| Error::I2CError)?;
        Ok(())
    }

    fn write(&mut self, data_addr: u8, buffer: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(1 + buffer.len());
        buf.push(data_addr);
        buf.extend_from_slice(buffer);
        self.i2c.write(&buf).map_err(|_| Error::I2CError)?;
        Ok(())
    }
}