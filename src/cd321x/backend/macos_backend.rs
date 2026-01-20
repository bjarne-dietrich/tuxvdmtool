/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */
use crate::{Error, Result};
use crate::cd321x::backend::{ControllerBackend};

pub(crate) struct MacOSBackend{}

use std::ffi::CString;
use std::ptr;

use io_kit_sys::{IOServiceMatching, IOServiceGetMatchingServices,IOIteratorNext, IOObjectRelease, kIOMasterPortDefault};
use log::info;

fn find_services(service_name: &str) -> Result<Vec<io_kit_sys::types::io_object_t>> {
    unsafe {
        let c_name = CString::new(service_name).map_err(|e| Error::FeatureMissing)?;

        let matching_dict = IOServiceMatching(c_name.as_ptr());
        if matching_dict.is_null() {
            return Err(Error::FeatureMissing);
        }

        let mut iterator: u32 = 0;

        let ret = IOServiceGetMatchingServices(kIOMasterPortDefault, matching_dict, &mut iterator);
        if ret != 0 {
            return Err(Error::FeatureMissing);
        }

        let mut results = Vec::new();

        loop {
            let service = IOIteratorNext(iterator);
            if service == 0 {
                break;
            }
            results.push(service);

            IOObjectRelease(service);
        }

        IOObjectRelease(iterator);

        info!("Found matching services {:?}", results);

        Ok(results)
    }
}


impl ControllerBackend for MacOSBackend {
    fn verify_host(&self) -> Result<()> {
        find_services("AppleHPM")?;
        Ok(())
    }

    fn get_unlock_key(&mut self) -> Result<Vec<u8>> {
        let key:u32 = 0; // TODO
        Ok(key.to_le_bytes().to_vec())
    }

    fn init(&mut self) -> Result<()> {
        self.verify_host()
    }

    fn deinit_device(&mut self) {
        todo!()
    }

    fn read(&mut self, data_addr: u8, buffer: &mut [u8]) -> Result<()> {
        Ok(())
    }

    fn write(&mut self, data_addr: u8, buffer: &[u8]) -> Result<()> {
        Ok(())
    }
}
