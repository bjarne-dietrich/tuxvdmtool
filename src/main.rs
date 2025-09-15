/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

pub mod cd321x;

use env_logger::Env;
use log::error;
use std::{fs, process::ExitCode};

#[derive(Debug)]
#[allow(dead_code)]
enum Error {
    Device,
    Compatible,
    FeatureMissing,
    TypecController,
    InvalidArgument,
    ReconnectTimeout,
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
}

type Result<T> = std::result::Result<T, Error>;

fn vdmtool() -> Result<()> {
    let matches = clap::command!()
        .arg(
            clap::arg!(-d --device [DEVICE] "Path to the USB-C controller device.")
                .default_value("/sys/class/i2c-dev/i2c-0/device/0-0038"),
        )
        .subcommand(
            clap::Command::new("reboot")
                .about("reboot the target")
                .subcommand(
                    clap::Command::new("serial").about("reboot the target and enter serial mode"),
                ),
        )
        // dummy command to display help for "reboot serial"
        .subcommand(
            clap::Command::new("reboot serial").about("reboot the target and enter serial mode"),
        )
        .subcommand(clap::Command::new("serial").about("enter serial mode on both ends"))
        .subcommand(clap::Command::new("dfu").about("put the target into DFU mode"))
        .subcommand(clap::Command::new("nop").about("Do nothing"))
        .arg_required_else_help(true)
        .get_matches();

    let compat: Vec<u8> = fs::read("/proc/device-tree/compatible").map_err(Error::Io)?;
    let compat = std::str::from_utf8(&compat[0..10]).map_err(Error::Utf8)?;
    let (manufacturer, device) = compat.split_once(",").ok_or(Error::Compatible)?;
    if manufacturer != "apple" {
        error!("Host is not an Apple silicon system: \"{compat}\"");
        return Err(Error::Compatible);
    }
    let code = device.to_uppercase();
    let device = cd321x::Device::new(matches.get_one::<String>("device").unwrap(), code)?;

    match matches.subcommand() {
        Some(("dfu", _)) => {
            device.dfu()?;
        }
        Some(("reboot", args)) => match args.subcommand() {
            Some(("serial", _)) => {
                device.reboot_serial()?;
            }
            None => {
                device.reboot()?;
            }
            _ => {}
        },
        Some(("nop", _)) => {}
        Some(("serial", _)) => {
            device.serial()?;
        }
        _ => {}
    }
    Ok(())
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    match vdmtool() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            error!("vdmtool: {:?}", e);
            ExitCode::FAILURE
        }
    }
}
