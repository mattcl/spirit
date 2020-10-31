use std::env;
use std::process;
use std::process::Command;

use clap::{
    crate_authors,
    crate_description,
    crate_version,
    App,
    AppSettings,
    Arg,
    ArgMatches,
};
use colorsys::Rgb;
use govee_rs::{API_BASE, Client};
use govee_rs::schema::{Color, PowerState};

use crate::error::{Result, UnwrapOrExit};

mod error;

fn main() {
    let mut app = App::new("spirit")
        .about(crate_description!())
        .version(crate_version!())
        .author(crate_authors!())
        .arg(
            Arg::with_name("device")
                .help("device name - if not provided, will operate on all devices")
                .required(false),
        )
        .subcommand(
            App::new("toggle")
                .about("toggle device(s)")
                .arg(
                    Arg::with_name("on")
                        .help("set the desired state to on (default)")
                        .long("on")
                        .conflicts_with("off")
                )
                .arg(
                    Arg::with_name("off")
                        .help("set the desired state to off")
                        .long("off")
                        .conflicts_with("on")
                )
        )
        .subcommand(
            App::new("check")
                .about("reacts to success or fail")
                .arg(
                    Arg::with_name("success")
                        .long("success")
                        .short("s")
                        .help("hex color to set on success")
                        .default_value("#00ff00")
                )
                .arg(
                    Arg::with_name("fail")
                        .long("fail")
                        .short("f")
                        .help("hex color to set on fail")
                        .default_value("#ff0000")
                )
                .arg(
                    Arg::with_name("cmd")
                        .help("command to run")
                        .required(true)
                        .multiple(true)
                        .last(true)
                ),
        );
    let matches = app.clone().get_matches();

    // TODO: better error when env var not set - MCL - 2020-10-30
    let client = Client::new(API_BASE, &env::var("GOVEE_KEY").expect("Missing GOVEE_KEY env var"));

    match matches.subcommand() {
        ("toggle", Some(toggle_matches)) => {
            toggle(&client, toggle_matches).unwrap_or_exit("Could not toggle power state");
        },
        ("check", Some(check_matches)) => {
            check(&client, check_matches).unwrap_or_exit("Could not check given command");
        }
        // If we weren't given a subcommand we know how to handle, display
        // the help message and exit.
        _ => {
            app.print_help().expect("Unable to display help message");
        }
    };
}

fn toggle(client: &Client, matches: &ArgMatches) -> Result<()> {
    let mut desired_state = PowerState::On;

    if matches.is_present("off") {
        desired_state = PowerState::Off;
    }

    for device in client.devices()?.devices {
        client.toggle(&device, desired_state.clone())?;
    }

    Ok(())
}

fn check(client: &Client, matches: &ArgMatches) -> Result<()> {
    let success = Rgb::from_hex_str(matches.value_of("success").unwrap())?;
    let fail = Rgb::from_hex_str(matches.value_of("fail").unwrap())?;

    let parsed: Vec<&str> = matches.values_of("cmd").unwrap().collect();

    // FIXME: figure out proper error to return here - MCL - 2020-10-30
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

    for device in client.devices()?.devices {
        client.set_color(&device, &color)?;
    }

    std::process::exit(res.code().expect("could not get status code"));
}
