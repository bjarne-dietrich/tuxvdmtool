/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

pub mod backend;

use crate::{Error, Result};
use log::{error, info};
use std::{
    thread,
    time::{Duration, Instant},
    str::FromStr,
};

use backend::{ControllerBackend, create_backend};

const RECONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_WAIT: Duration = Duration::from_millis(100);
const RECONNECT_WAIT: Duration = Duration::from_secs(1);

const  TPS_REG_MODE	: u8 = 0x03;
const TPS_REG_CMD1: u8 = 0x08;
const TPS_REG_DATA1: u8 = 0x09;
const TPS_REG_POWER_STATUS: u8 = 0x3f;

#[allow(dead_code)]
enum VdmSopType {
    Sop = 0b00,
    SopPrime = 0b01,
    SopPrimePrime = 0b10,
    SopStar = 0b11,
}

#[allow(dead_code)]
#[derive(Debug)]
#[derive(PartialEq)]
enum TpsMode {
    TpsModeApp,
    TpsModeBoot,
    TpsModeBist,
    TpsModeDisc,
    TpsModePtch,
    TpsModeDbma,
}
impl FromStr for TpsMode {
    type Err = ();
    fn from_str(input: &str) -> std::result::Result<TpsMode, ()> {
        match input {
            "APP " => Ok(TpsMode::TpsModeApp),
            "BOOT" => Ok(TpsMode::TpsModeBoot),
            "BIST" => Ok(TpsMode::TpsModeBist),
            "DISC" => Ok(TpsMode::TpsModeDisc),
            "PTCH" => Ok(TpsMode::TpsModePtch),
            "DBMa" => Ok(TpsMode::TpsModeDbma),
            _ => Err(()),
        }
    }
}

pub(crate) struct Device {
    backend: Box<dyn ControllerBackend>,
}

fn is_invalid_cmd(val: u32) -> bool {
    val == 0x444d4321
}

impl Device {
    pub (crate) fn new(bus: &str, chip_addr: u16) -> Result<Self> {
        let mut backend = create_backend(bus, chip_addr)?;
        backend.init()?;
        let key = backend.get_unlock_key()?;


        let mut device = Self {
            backend,
        };

        let power_status = device.read_power_status()?;

        // Check for Connected Cable
        if power_status & 1 == 0 {
            return Err(Error::NoCableConnected)
        }
        // Print Connection
        if power_status & 2 == 0 { info!("Connection: Source"); } else { info!("Connection: Sink"); }

        if device.get_mode()? != TpsMode::TpsModeDbma {
            device.unlock(&key)?;
            device.dbma_mode(true)?;
        }

        Ok(device)
    }

    fn read_power_status(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_block(TPS_REG_POWER_STATUS, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn get_mode(&mut self) -> Result<TpsMode> {
        let mut buf = [0u8; 4];
        self.read_block(TPS_REG_MODE, &mut buf)?;
        let s = std::str::from_utf8(&buf).unwrap();
        let m = TpsMode::from_str(s).map_err(|_| Error::TypecController)?;
        Ok(m)
    }

    fn unlock(&mut self, key: &[u8]) -> Result<()> {
        let res = self.exec_cmd(b"LOCK", &key);
        if res.is_err() {
            info!("Unlocking failed, trying to reset...");
            let res = self.exec_cmd(b"Gaid", &[]);
            if res.is_err() {
                info!("Failed to unlock device!");
                return Err(Error::TypecController)
            }
            info!("Reset OK, unlocking once more...");
            self.exec_cmd(b"LOCK", &key)?;
        }
        info!("Device unlocked.");
        Ok(())
    }

    fn lock(&mut self) -> Result<()> {
        self.exec_cmd(b"LOCK", &[0u8; 4])
    }

    fn dbma_mode(&mut self, enabled: bool) -> Result<()> {
        let data: [u8; 1] = if enabled { [1] } else { [0] };
        self.exec_cmd(b"DBMa", &data)?;
        if self.get_mode()? != TpsMode::TpsModeDbma {
            return Err(Error::TypecController);
        }
        Ok(())
    }

    pub(crate) fn dfu(&mut self) -> Result<()> {
        let vdos: [u32; 3] = [0x5ac8012, 0x106, 0x80010000];
        info!("Rebooting target into DFU mode...");
        self.vdms(VdmSopType::SopStar, &vdos)
    }

    fn vdms(&mut self, sop: VdmSopType, vdos: &[u32]) -> Result<()> {
        if vdos.is_empty() || vdos.len() > 7 {
            return Err(Error::InvalidArgument);
        }
        if self.get_mode()? != TpsMode::TpsModeDbma {
            return Err(Error::TypecController);
        }
        let data = [
            vec![((sop as u8) << 4) | vdos.len() as u8],
            vdos.iter().flat_map(|val| val.to_le_bytes()).collect(),
        ]
            .concat();
        self.exec_cmd_with_timing(b"VDMs", &data, Duration::from_millis(200), Duration::from_millis(200))
    }

    fn dven(&mut self, vdos: &[u32]) -> Result<()> {
        let data: Vec<u8> = vdos.iter().flat_map(|val| val.to_le_bytes()).collect();
        self.exec_cmd(b"DVEn", &data)
    }

    pub(crate) fn reboot(&mut self) -> Result<()> {
        let vdos: [u32; 3] = [0x5ac8012, 0x105, 0x80000000];
        info!("Rebooting target into normal mode...");
        self.vdms(VdmSopType::SopStar, &vdos)
    }

    pub(crate) fn serial(&mut self) -> Result<()> {
        let vdos: [u32; 2] = [0x5ac8012, 0x1840306];
        info!("Putting target into serial mode...");
        self.vdms(VdmSopType::SopStar, &vdos)?;
        info!("Putting local end into serial mode... ");
        if self.get_mode()? != TpsMode::TpsModeDbma {
            return Err(Error::TypecController)
        }
        self.dven(&vdos[1..2])
    }

    pub(crate) fn reboot_serial(&mut self) -> Result<()> {
        self.reboot()?;
        info!("Waiting for connection...");

        thread::sleep(RECONNECT_WAIT);

        let now = Instant::now();
        loop {
            let power_status = self.read_power_status()?;
            // Check for Connected Cable
            if power_status & 1 != 0 {
                break;
            }
            thread::sleep(POLL_WAIT);
            if now.elapsed() > RECONNECT_TIMEOUT {
                error!("Timeout while waiting ");
                return Err(Error::ReconnectTimeout);
            }
        }
        info!("Connected");
        thread::sleep(RECONNECT_WAIT);
        self.serial()
    }

    fn exec_cmd(
        &mut self,
        cmd_tag: &[u8; 4],
        in_data: &[u8],
    ) -> Result<()> {
        self.exec_cmd_with_timing(cmd_tag, in_data, Duration::from_secs(1), Duration::ZERO)
    }

    fn exec_cmd_with_timing(
        &mut self,
        cmd_tag: &[u8; 4],
        in_data: &[u8],
        cmd_timeout: Duration,
        res_delay: Duration,
    ) -> Result<()> {


        // First: Check CMD1 Register busy
        {
            let mut status_buf = [0u8; 4];
            self.read_block(TPS_REG_CMD1, &mut status_buf)?;
            let val = u32::from_le_bytes(status_buf);
            if val != 0 && !is_invalid_cmd(val) {
                info!("Busy Check Failed with VAL = {:?}", val);
                return Err(Error::TypecController);
            }
        }

        // Write input Data to DATA1
        if !in_data.is_empty() {
            self.write_block(TPS_REG_DATA1, in_data)?;
        }

        // Write 4-byte command tag
        self.write_block(TPS_REG_CMD1, cmd_tag)?;

        // Poll until CMD1 becomes zero or timeout
        let start = Instant::now();
        loop {
            let mut status_buf = [0u8; 4];
            self.read_block(TPS_REG_CMD1, &mut status_buf)?;
            let val = u32::from_le_bytes(status_buf);
            if is_invalid_cmd(val) {
                info!("Invalid Command");
                return Err(Error::TypecController)
            }
            if val == 0 {
                break;
            }
            if start.elapsed() > cmd_timeout {
                return Err(Error::ControllerTimeout)
            }
        }
        thread::sleep(res_delay);
        Ok(())
    }

    fn write_block(&mut self, reg: u8, data: &[u8]) -> Result<()> {
        // First Byte of data is always length.
        // We enlarge the buffer to account for that.
        let mut buf = Vec::with_capacity(1 + data.len());
        let size: u8 = data.len().try_into().unwrap();
        buf.push(size);
        buf.extend_from_slice(data);
        self.backend.write(reg , &buf)?;
        Ok(())
    }

    fn read_block(&mut self, reg: u8, buf: &mut [u8]) -> Result<()> {
        // First Byte is always length of Data!
        // We enlarge the buffer to account for that

        let mut internal_buf = vec![0u8; buf.len() + 1];
        self.backend.read(reg, &mut internal_buf)?;
        buf.copy_from_slice(&internal_buf[1..=buf.len()]);

        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.dbma_mode(false);
        let _ = self.lock();
    }
}
