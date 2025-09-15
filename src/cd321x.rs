/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

use crate::{Error, Result};
use log::{error, info};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    path, thread,
    time::{Duration, Instant},
};

const RECONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_WAIT: Duration = Duration::from_millis(100);
const RECONNECT_WAIT: Duration = Duration::from_secs(1);

#[allow(dead_code)]
enum VdmSopType {
    Sop = 0b00,
    SopPrime = 0b01,
    SopPrimePrime = 0b10,
    SopStar = 0b11,
}

pub(crate) struct Device {
    path: path::PathBuf,
    key: Vec<u8>,
}

fn verify_device(dev: &str) -> Result<path::PathBuf> {
    let mut fname = OpenOptions::new()
        .read(true)
        .open(path::Path::new(dev).join("name"))
        .unwrap();
    let mut data = Vec::new();
    fname.read_to_end(&mut data).unwrap();
    let name = std::str::from_utf8(&data).map_err(Error::Utf8)?.trim();
    if name != "cd321x" {
        error!("{dev}/name \"{name}\" does not match \"cd321x\"");
        return Err(Error::TypecController);
    }

    let vdm_dir = path::Path::new(dev).join("cd321x_vdm");
    if !vdm_dir.exists() {
        error!("{} does not exists", vdm_dir.display());
        return Err(Error::FeatureMissing);
    }
    Ok(vdm_dir.to_path_buf())
}

impl Device {
    pub(crate) fn new(dev: &str, code: String) -> Result<Self> {
        let device = Self {
            path: verify_device(dev)?,
            key: code.into_bytes().into_iter().rev().collect::<Vec<u8>>(),
        };
        device.lock(device.key.as_slice())?;
        device.dbma(true)?;

        Ok(device)
    }

    fn command(&self, command: &[u8; 4], data: &[u8]) -> Result<()> {
        let data: Vec<u8> = [command, data].concat();
        fs::write(self.path.as_path().join("command"), &data).map_err(Error::Io)
    }

    fn lock(&self, key: &[u8]) -> Result<()> {
        self.command(b"LOCK", key)
    }

    fn dbma(&self, debug: bool) -> Result<()> {
        let data: [u8; 1] = if debug { [1] } else { [0] };
        self.command(b"DBMa", &data)
    }

    fn vdms(&self, sop: VdmSopType, vdos: &[u32]) -> Result<()> {
        if vdos.is_empty() || vdos.len() > 7 {
            return Err(Error::InvalidArgument);
        }
        let data = [
            vec![((sop as u8) << 4) | vdos.len() as u8],
            vdos.iter().flat_map(|val| val.to_le_bytes()).collect(),
        ]
        .concat();
        self.command(b"VDMs", &data)
    }

    fn dven(&self, vdos: &[u32]) -> Result<()> {
        let data: Vec<u8> = vdos.iter().flat_map(|val| val.to_le_bytes()).collect();
        self.command(b"DEVn", &data)
    }

    fn is_connected(&self) -> Option<bool> {
        let data: Vec<u8> = fs::read(self.path.as_path().join("power_status")).ok()?;
        let string = std::str::from_utf8(&data).ok()?;
        if string.len() < 6 {
            return None;
        }
        let power_status = u16::from_str_radix(&string[2..6], 16).ok()?;

        Some((power_status & 1) != 0)
    }

    pub(crate) fn dfu(&self) -> Result<()> {
        let vdos: [u32; 3] = [0x5ac8012, 0x106, 0x80010000];
        info!("Rebooting target into DFU mode...");
        self.vdms(VdmSopType::SopStar, &vdos)
    }
    pub(crate) fn reboot(&self) -> Result<()> {
        let vdos: [u32; 3] = [0x5ac8012, 0x105, 0x80000000];
        info!("Rebooting target into normal mode...");
        self.vdms(VdmSopType::SopStar, &vdos)
    }

    pub(crate) fn reboot_serial(&self) -> Result<()> {
        self.reboot()?;
        info!("Waiting for connection...");

        thread::sleep(RECONNECT_WAIT);

        let now = Instant::now();
        loop {
            if self.is_connected().unwrap_or(false) {
                break;
            }
            thread::sleep(POLL_WAIT);
            if now.elapsed() > RECONNECT_TIMEOUT {
                error!("Timeout while waiting ");
                return Err(Error::ReconnectTimeout);
            }
        }
        info!(" Connected");
        thread::sleep(RECONNECT_WAIT);
        self.serial()
    }

    pub(crate) fn serial(&self) -> Result<()> {
        let vdos: [u32; 2] = [0x5ac8012, 0x1840306];
        info!("Putting target into serial mode...");
        self.vdms(VdmSopType::SopStar, &vdos)?;
        info!("Putting local end into serial mode... ");
        self.dven(&vdos[1..2])
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let lock: [u8; 4] = [0, 0, 0, 0];
        let _ = self.dbma(false);
        let _ = self.lock(&lock);
    }
}
