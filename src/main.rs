use std::collections::{HashMap, HashSet};
use std::process::Command;

use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg, ArgMatches};
use colorsys::Rgb;
use govee_rs::schema::{Color, Device, PowerState};
use govee_rs::{Client, API_BASE};

use crate::error::{Result, SpiritError, UnwrapOrExit};
use crate::settings::{Settings, DeviceSetting, DeviceSettingMap};

mod error;
mod settings;

fn main() {
    let settings = Settings::new().unwrap_or_exit("Could not load spirit.toml file");

    let mut success_color = "#00ff00".to_string();
    let mut fail_color = "#ff0000".to_string();

    if let Some(ref settings) = settings {
        if let Some(ref success) = settings.success {
            success_color = success.clone();
        }

        if let Some(ref fail) = settings.fail {
            fail_color = fail.clone();
        }
    }

    let mut app = App::new("spirit")
        .about(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .global_setting(AppSettings::ColorAuto)
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("govee_key")
                .help("Gove API key")
                .long("key")
                .short("k")
                .env("GOVEE_KEY")
                .hide_env_values(true)
                .required(false),
        )
        .arg(
            Arg::with_name("all")
                .help("Operate on all devices regardless of config")
                .long("all")
                .short("a")
                .required(false)
                .conflicts_with("device"),
        )
        .arg(
            Arg::with_name("device")
                .help("Device name - if not provided, will operate on all devices")
                .long("device")
                .short("d")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1)
                .required(false)
                .conflicts_with("all"),
        )
        .subcommand(
            App::new("info")
                .about("Displays the info for a device")
        )
        .subcommand(
            App::new("toggle")
                .about("Toggle device(s)")
                .arg(
                    Arg::with_name("on")
                        .help("Set the desired state to on (default)")
                        .long("on")
                        .conflicts_with("off"),
                )
                .arg(
                    Arg::with_name("off")
                        .help("Set the desired state to off")
                        .long("off")
                        .conflicts_with("on"),
                )
                .arg(
                    Arg::with_name("color")
                        .long("color")
                        .short("c")
                        .help("The color to set the lights to when turning on")
                        .conflicts_with("off")
                        .takes_value(true),
                ),
        )
        .subcommand(
            App::new("check")
                .about("Reacts to success or fail")
                .arg(
                    Arg::with_name("success")
                        .long("success")
                        .short("s")
                        .help("Hex color to set on success")
                        .env("SPIRIT_SUCCESS_COLOR")
                        .default_value(&success_color),
                )
                .arg(
                    Arg::with_name("fail")
                        .long("fail")
                        .short("f")
                        .help("Hex color to set on fail")
                        .env("SPIRIT_FAIL_COLOR")
                        .default_value(&fail_color),
                )
                .arg(
                    Arg::with_name("cmd")
                        .help("Command to run")
                        .required(true)
                        .multiple(true)
                        .last(true),
                ),
        );

    let matches = app.clone().get_matches();

    let client = make_client(&matches).unwrap_or_exit("GOVEE_KEY env var must be set");
    let devices =
        get_devices(&client, &matches, &settings).unwrap_or_exit("Could not fetch list of devices");

    match matches.subcommand() {
        ("info", Some(_)) => {
            info(&client, &devices).unwrap_or_exit("Could not get device info");
        }
        ("toggle", Some(toggle_matches)) => {
            toggle(&client, &devices, toggle_matches, &settings)
                .unwrap_or_exit("Could not toggle power state");
        }
        ("check", Some(check_matches)) => {
            check(&client, &devices, check_matches, &settings).unwrap_or_exit("Could not check given command");
        }
        // If we weren't given a subcommand we know how to handle, display
        // the help message and exit.
        _ => {
            app.print_help().expect("Unable to display help message");
        }
    };
}

fn make_client(matches: &ArgMatches) -> Result<Client> {
    if let Some(key) = matches.value_of("govee_key") {
        return Ok(Client::new(API_BASE, key));
    }

    Err(SpiritError::Error(
        "must either supply govee key or set GOVEE_KEY".to_string(),
    ))
}

fn get_devices(
    client: &Client,
    matches: &ArgMatches,
    settings: &Option<Settings>,
) -> Result<Vec<Device>> {
    let devices = client.devices()?.devices;

    if !matches.is_present("all") {
        if matches.is_present("device") {
            let device_names: HashSet<&str> = matches.values_of("device").unwrap().collect();
            let filtered_devices: Vec<Device> = devices
                .into_iter()
                .filter(|device| device_names.contains(device.name.as_str()))
                .collect();

            if filtered_devices.len() < 1 {
                return Err(SpiritError::Error("No devices matched".to_string()));
            }

            return Ok(filtered_devices);
        } else if let Some(settings) = settings {
            let device_names = settings.device_settings();
            let filtered_devices: Vec<Device> = devices
                .into_iter()
                .filter(|device| device_names.get(&device.name).is_some())
                .collect();

            if filtered_devices.len() < 1 {
                return Err(SpiritError::Error("No devices matched".to_string()));
            }

            return Ok(filtered_devices);
        }
    }

    Ok(devices)
}

fn info(client: &Client, devices: &Vec<Device>) -> Result<()> {
    for device in devices {
        println!("{:#?}", client.state(&device)?);
    }
    Ok(())
}

fn toggle(
    client: &Client,
    devices: &Vec<Device>,
    matches: &ArgMatches,
    settings: &Option<Settings>,
) -> Result<()> {
    if matches.is_present("off") {
        for device in devices {
            client.toggle(&device, PowerState::Off)?;
        }

        return Ok(());
    }

    let device_settings = match settings {
        Some(settings) => settings.device_settings(),
        None => DeviceSettingMap::default()
    };

    let force = matches.value_of("color");
    let default = settings.as_ref().and_then(|s| s.default.as_deref());

    for device in devices {
        if let Some(color) = device_settings.default_color(&device.name, force, default)? {
            client.set_color(&device, &color)?;
        } else {
            client.toggle(&device, PowerState::On)?;
        }
    }

    Ok(())
}

fn check(client: &Client, devices: &Vec<Device>, matches: &ArgMatches, settings: &Option<Settings>) -> Result<()> {
    let success = matches.value_of("success");
    let fail = matches.value_of("fail");

    let device_settings = match settings {
        Some(settings) => settings.device_settings(),
        None => DeviceSettingMap::default()
    };

    let parsed: Vec<&str> = matches.values_of("cmd").unwrap().collect();

    let (cmd, args) = parsed.split_first().expect("command was empty");

    let res = Command::new(cmd).args(args).status()?;

    for device in devices {
        let color = if res.success() {
            device_settings.success_color(&device.name, success)?
        } else {
            device_settings.fail_color(&device.name, fail)?
        }.unwrap();
        client.set_color(&device, &color)?;
    }

    std::process::exit(res.code().expect("could not get status code"));
}
