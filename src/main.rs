/*
 * SPDX-License-Identifier: Apache-2.0
 *
 * Copyright The Asahi Linux Contributors
 */

pub mod cd321x;

use env_logger::Env;
use log::{error};
use std::{process::ExitCode};
use crate::cd321x::Device;

#[derive(Debug)]
#[allow(dead_code)]
enum Error {
    Device,
    Compatible,
    FeatureMissing,
    TypecController,
    NoCableConnected,
    InvalidArgument,
    ReconnectTimeout,
    ControllerTimeout,
    I2CError,
    Io(std::io::Error),
    Utf8(std::str::Utf8Error),
}

type Result<T> = std::result::Result<T, Error>;

fn vdmtool() -> Result<()> {
    let matches = clap::command!()
        .arg(
            clap::arg!(-b --bus [BUS] "i2c bus of the USB-C controller device.")
                .default_value("/dev/i2c-0"),
        )
        .arg(
            clap::arg!(-a --address [ADDRESS] "i2c slave address of the USB-C controller device.")
                .default_value("0x38"),
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

    let addr_str = matches.get_one::<String>("address").unwrap();
    let addr: u16;
    if addr_str.starts_with("0x") {
        addr = u16::from_str_radix(&addr_str[2..], 16).unwrap();
    } else {
        addr = u16::from_str_radix(addr_str, 10).unwrap();
    }

    let mut device = Device::new(matches.get_one::<String>("bus").unwrap(), addr)?;

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
