/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */
use crate::{Error, Result};
use crate::cd321x::backend::{ControllerBackend};

pub(crate) struct MacOSBackend{}

unsafe extern "C" {
    safe fn GetUnlockKey() -> u32;
}

impl ControllerBackend for MacOSBackend {
    fn verify_host(&self) -> Result<()> {
        todo!()
    }

    fn get_unlock_key(&mut self) -> Result<Vec<u8>> {
        let key = GetUnlockKey();
        Ok(key.to_le_bytes().to_vec())
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn deinit_device(&mut self) {
        todo!()
    }

    fn read(&mut self, data_addr: u8, buffer: &mut [u8]) -> Result<()> {
        todo!()
    }

    fn write(&mut self, data_addr: u8, buffer: &[u8]) -> Result<()> {
        todo!()
    }
}
