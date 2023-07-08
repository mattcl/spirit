use std::{collections::HashSet, process::Command};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use govee_rs::{
    models::{Devices, PowerState},
    GoveeClient, DEFAULT_API_URL,
};

use crate::settings::Settings;

/// A command-line interface for controlling sets of govee lights.
#[derive(Parser)]
#[command(author, version)]
pub struct Cli {
    /// The govee api key.
    #[arg(short, long, env = "GOVEE_KEY", hide_env_values = true)]
    govee_key: String,

    /// Operate on all devices regardless of config.
    #[arg(short, long)]
    all: bool,

    /// The device name. May be specified multiple times.
    ///
    /// If not provided will operate on all devices specified by the config.
    #[arg(short, long, conflicts_with = "all")]
    device: Vec<String>,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn run() -> Result<()> {
        let cli = Self::parse();

        let settings = Settings::new()
            .context("Could not load spirit.toml file")?
            .ok_or_else(|| anyhow!("spirit.toml evaluated to an empty settings object"))?;

        let client = GoveeClient::new(DEFAULT_API_URL, &cli.govee_key)?;

        cli.command
            .run(
                &client,
                &settings,
                &cli.get_devices(&client, &settings).await?,
            )
            .await
    }

    async fn get_devices(&self, client: &GoveeClient, settings: &Settings) -> Result<Devices> {
        let mut devices = client.devices().await?;

        if !self.all {
            if !self.device.is_empty() {
                let device_names: HashSet<&String> = self.device.iter().collect();
                devices.devices.retain(|d| device_names.contains(&d.name));

                if devices.is_empty() {
                    bail!("No devices matched");
                }

                Ok(devices)
            } else {
                let device_names = settings.device_settings();

                devices
                    .devices
                    .retain(|d| device_names.get(&d.name).is_some());

                if devices.is_empty() {
                    bail!("No devices matched");
                }

                Ok(devices)
            }
        } else {
            Ok(devices)
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    Info(Info),
    Toggle(Toggle),
    Check(Check),
}

impl Commands {
    pub async fn run(
        &self,
        client: &GoveeClient,
        settings: &Settings,
        devices: &Devices,
    ) -> Result<()> {
        match self {
            Self::Info(cmd) => cmd.run(client, settings, devices).await,
            Self::Toggle(cmd) => cmd.run(client, settings, devices).await,
            Self::Check(cmd) => cmd.run(client, settings, devices).await,
        }
    }
}

/// Display info about a set of devices.
#[derive(Args)]
pub struct Info;

impl Info {
    pub async fn run(
        &self,
        client: &GoveeClient,
        _settings: &Settings,
        devices: &Devices,
    ) -> Result<()> {
        for device in devices.iter() {
            println!("{:#?}", client.state(device).await?);
        }
        Ok(())
    }
}

/// Toggle the power state of a set of devices.
#[derive(Args)]
pub struct Toggle {
    /// Toggle devices on.
    #[arg(long, conflicts_with = "off")]
    on: bool,

    /// Toggle devices off.
    #[arg(long)]
    off: bool,

    /// Set this color for toggled devices.
    #[arg(short, long, conflicts_with = "off")]
    color: Option<String>,
}

impl Toggle {
    pub async fn run(
        &self,
        client: &GoveeClient,
        settings: &Settings,
        devices: &Devices,
    ) -> Result<()> {
        if self.off {
            for device in devices.iter() {
                client.turn(device, PowerState::Off).await?;
            }

            return Ok(());
        }

        let device_settings = settings.device_settings();

        let force = self.color.as_deref();
        let default = settings.default.as_deref();

        for device in devices.iter() {
            if let Some(color) = device_settings.default_color(&device.name, force, default)? {
                client.color(device, color).await?;
            } else {
                client.turn(device, PowerState::On).await?;
            }
        }

        Ok(())
    }
}

/// Run a command, altering the color of a set of devices based on exit code.
///
/// This is binary decision where the success color corresponds to exit code 0
/// and the fail color to all other exit codes.
#[derive(Args)]
pub struct Check {
    /// Set this color on success.
    #[arg(short, long, env = "SPIRIT_SUCCESS_COLOR")]
    success: Option<String>,

    /// Set this color on fail.
    #[arg(short, long, env = "SPIRIT_FAIL_COLOR")]
    fail: Option<String>,

    /// The command to run
    #[arg(last = true)]
    cmd: Vec<String>,
}

impl Check {
    pub async fn run(
        &self,
        client: &GoveeClient,
        settings: &Settings,
        devices: &Devices,
    ) -> Result<()> {
        let success = self.success.as_deref();
        let fail = self.fail.as_deref();

        let device_settings = settings.device_settings();

        let parsed: Vec<&String> = self.cmd.iter().collect();

        let (cmd, args) = parsed.split_first().expect("command was empty");

        let res = Command::new(cmd).args(args).status()?;

        for device in devices.iter() {
            let color = if res.success() {
                device_settings.success_color(&device.name, success)?
            } else {
                device_settings.fail_color(&device.name, fail)?
            }
            .unwrap();
            client.color(device, color).await?;
        }

        std::process::exit(res.code().expect("could not get status code"));
    }
}
