use std::collections::{HashMap, HashSet};
use std::process::Command;

use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg, ArgMatches};
use colorsys::Rgb;
use govee_rs::schema::{Color, Device, PowerState};
use govee_rs::{Client, API_BASE};

use crate::error::{Result, SpiritError, UnwrapOrExit};
use crate::settings::{Settings, DeviceSetting};

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
            check(&client, &devices, check_matches).unwrap_or_exit("Could not check given command");
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
                .filter(|device| device_names.contains_key(&device.name))
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
        None => HashMap::new()
    };

    for device in devices {
        if let Some(color) = get_color(device, matches, settings, &device_settings)? {
            client.set_color(&device, &color)?;
        } else {
            client.toggle(&device, PowerState::On)?;
        }
    }

    Ok(())
}

fn get_color(device: &Device, matches: &ArgMatches, settings: &Option<Settings>, device_settings: &HashMap<String, DeviceSetting>) -> Result<Option<Color>> {
    let device_color = match device_settings.get(&device.name) {
        Some(setting) => setting.color()?,
        None => None
    };

    let mut color: Option<String> = None;
    if let Some(color_str) = matches.value_of("color") {
        color = Some(color_str.to_string());
    } else if let Some(device_color) = device_color {
        return Ok(Some(device_color))
    } else if let Some(settings) = settings {
        if let Some(ref default) = settings.default {
            color = Some(default.clone());
        }
    }

    if let Some(color_str) = color {
        let parsed = Rgb::from_hex_str(&color_str)?;
        Ok(Some(Color {
            r: parsed.get_red() as u32,
            g: parsed.get_green() as u32,
            b: parsed.get_blue() as u32,
        }))
    } else {
        Ok(None)
    }
}

fn check(client: &Client, devices: &Vec<Device>, matches: &ArgMatches) -> Result<()> {
    let success = Rgb::from_hex_str(matches.value_of("success").unwrap())?;
    let fail = Rgb::from_hex_str(matches.value_of("fail").unwrap())?;

    let parsed: Vec<&str> = matches.values_of("cmd").unwrap().collect();

    let (cmd, args) = parsed.split_first().expect("command was empty");

    let res = Command::new(cmd).args(args).status()?;

    let mut color = Color {
        r: success.get_red() as u32,
        g: success.get_green() as u32,
        b: success.get_blue() as u32,
    };

    if !res.success() {
        color = Color {
            r: fail.get_red() as u32,
            g: fail.get_green() as u32,
            b: fail.get_blue() as u32,
        };
    }

    for device in devices {
        client.set_color(&device, &color)?;
    }

    std::process::exit(res.code().expect("could not get status code"));
}
