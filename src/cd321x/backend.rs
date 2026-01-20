/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

#[cfg(target_os = "macos")]
pub mod macos_backend;
#[cfg(target_os = "linux")]
pub mod linux_backend;

#[cfg(target_os = "macos")]
use macos_backend::MacOSBackend;
#[cfg(target_os = "linux")]
use linux_backend::LinuxBackend;


pub fn create_backend(bus: &str, chip_addr: u16) -> Result<Box<dyn ControllerBackend>>
{
    #[cfg(target_os = "linux")]
    {
        let backend = LinuxBackend::new(bus, chip_addr)?;
        Ok(Box::new(backend))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(MacOSBackend {}))
    }
}

use crate::{Error, Result};

pub trait ControllerBackend {
    fn verify_host(&self) -> Result<()>;
    fn get_unlock_key(&mut self) -> Result<Vec<u8>>;
    fn init(&mut self) -> Result<()>;
    fn deinit_device(&mut self);
    fn read(&mut self, data_addr: u8, buffer: &mut [u8]) -> Result<()>;
    fn write(&mut self, data_addr: u8, buffer: &[u8]) -> Result<()>;

}
