mod config;

use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use futures::TryStreamExt;
use thiserror::Error;

#[derive(Error, Debug)]
enum Error {
    #[error("Speaker not found: {0}")]
    SpeakerNotFound(String),

    #[error("No default speaker set")]
    NoDefaultSpeaker,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();
    let timeout = Duration::from_secs(config.timeout);

    match cli.command {
        Subcommands::Play { name } => {
            let name = name.or(config.default).ok_or(Error::NoDefaultSpeaker)?;

            let speaker = sonor::find(&name, timeout)
                .await?
                .ok_or(Error::SpeakerNotFound(name))?;

            speaker.play().await?;
            println!("Playing on {}", speaker.name().await?);
        }
        Subcommands::Pause { name } => {
            let name = name.or(config.default).ok_or(Error::NoDefaultSpeaker)?;

            let speaker = sonor::find(&name, timeout)
                .await?
                .ok_or(Error::SpeakerNotFound(name))?;

            speaker.pause().await?;
            println!("Pausing {}", speaker.name().await?);
        }
        Subcommands::Volume { volume } => {
            let volume = volume.clamp(0, 100);
            let mut speakers = sonor::discover(timeout).await?;

            while let Some(speaker) = speakers.try_next().await? {
                speaker.set_volume(volume).await?;
            }

            println!("Set volume to {}%", volume);
        }
        Subcommands::Track { name } => {
            let name = name.or(config.default).ok_or(Error::NoDefaultSpeaker)?;

            let speaker = sonor::find(&name, timeout)
                .await?
                .ok_or(Error::SpeakerNotFound(name))?;

            match speaker.track().await? {
                None => println!("No track playing"),
                Some(track) => {
                    let track = track.track();

                    match track.creator() {
                        None => println!("Playing {}", track.title()),
                        Some(creator) => println!("Playing {} by {}", track.title(), creator),
                    }
                }
            }
        }
        Subcommands::Group { name } => {
            let name = name.or(config.default).ok_or(Error::NoDefaultSpeaker)?;

            let main = sonor::find(&name, timeout)
                .await?
                .ok_or(Error::SpeakerNotFound(name))?;

            let name = main.name().await?;

            let speakers = sonor::discover(timeout)
                .await?
                .try_collect::<Vec<_>>()
                .await?;

            for speaker in speakers {
                if speaker.device().url() != main.device().url() {
                    speaker.join(&name).await?;
                }
            }

            println!("Grouped all speakers to {}", name);
        }
        Subcommands::Ungroup => {
            let mut speakers = sonor::discover(timeout).await?;

            while let Some(speaker) = speakers.try_next().await? {
                speaker.leave().await?;
            }

            println!("Ungrouped all speakers");
        }
        Subcommands::Config { command } => {
            command.run(&config)?;
        }
    }

    Ok(())
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    Play {
        name: Option<String>,
    },
    Pause {
        name: Option<String>,
    },
    Volume {
        volume: u16,
    },
    Track {
        name: Option<String>,
    },
    Group {
        name: Option<String>,
    },
    Ungroup,
    Config {
        #[command(subcommand)]
        command: config::Subcommands,
    },
}
