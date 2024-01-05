use std::fs::{self, File};
use std::path::PathBuf;
use std::{io, num};

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No config directory")]
    NoConfigDirectory,

    #[error("Invalid config")]
    InvalidConfig,

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    ParseInt(#[from] num::ParseIntError),
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub timeout: u64,
    pub default: Option<String>,
}

impl Config {
    fn directory() -> Result<PathBuf, Error> {
        let mut path = dirs::config_dir().ok_or(Error::NoConfigDirectory)?;
        path.push(env!("CARGO_PKG_NAME"));
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn load() -> Result<Self, Error> {
        let mut path = Self::directory()?;
        path.push("config.json");
        let file = File::open(path)?;
        let config = serde_json::from_reader(file)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Error> {
        let mut path = Self::directory()?;
        path.push("config.json");
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: 1,
            default: None,
        }
    }
}

#[derive(Subcommand)]
pub enum Subcommands {
    List,
    Get { key: String },
    Set { key: String, value: String },
}

impl Subcommands {
    pub fn run(&self, config: &Config) -> Result<(), Error> {
        let Value::Object(mut object) = serde_json::to_value(config)? else {
            return Err(Error::InvalidConfig);
        };

        match self {
            Subcommands::List => {
                for (key, value) in object.iter() {
                    println!("{}: {}", key, value);
                }
            }
            Subcommands::Get { key } => {
                let value = &object[key];
                println!("{}: {}", key, value);
            }
            Subcommands::Set { key, value } => {
                object[key] = value.parse()?;
                let config: Config = serde_json::from_value(Value::Object(object))?;
                config.save()?;
            }
        }

        Ok(())
    }
}
