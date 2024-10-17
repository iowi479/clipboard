use std::fs;
use std::io;

use crate::logfile::log_and_panic;

pub struct Config {
    pub local_name: String,
    pub remote_names: Vec<String>,
    pub dir_name: String,
}

const CONFIG_FILE_NAME: &str = "config.ini";

// example config.ini:
// local_name=ubuntu
// remote_names=win,ubuntu
// dir_name=./

impl Config {
    pub fn load() -> io::Result<Self> {
        let content = fs::read_to_string(CONFIG_FILE_NAME)?;

        let mut conf_local_name = None;
        let mut conf_remote_names = None;
        let mut conf_dir_name = None;

        content.lines().for_each(|line| {
            // skip empty lines
            if line.is_empty() {
                return;
            }

            let mut parts = line.split("=");
            let key = parts
                .next()
                .unwrap_or_else(|| {
                    log_and_panic(&format!(
                        "no key provided on line in config file:\n{}",
                        line
                    ));
                    unreachable!();
                })
                .trim();
            let value = parts
                .next()
                .unwrap_or_else(|| {
                    log_and_panic(&format!(
                        "no value provided on line in config file:\n{}",
                        line
                    ));
                    unreachable!();
                })
                .trim();

            match key {
                "local_name" => {
                    if conf_local_name.is_some() {
                        log_and_panic("local_name is a duplicate");
                    }
                    conf_local_name = Some(value.to_string());
                }
                "remote_names" => {
                    if conf_remote_names.is_some() {
                        log_and_panic("remote_names is a duplicate");
                    }
                    conf_remote_names =
                        Some(value.split(",").map(|s| s.trim().to_string()).collect());
                }
                "dir_name" => {
                    if conf_dir_name.is_some() {
                        log_and_panic("dir_name is a duplicate");
                    }

                    conf_dir_name = Some(value.to_string());

                    if let Err(e) = std::fs::read_dir(&conf_dir_name.as_ref().unwrap()) {
                        log_and_panic(&format!(
                            "Could not read specified directory: {}\n{}",
                            conf_dir_name.as_ref().unwrap(),
                            e
                        ));
                    }
                }
                _ => {
                    log_and_panic(&format!("Unknown key in config file: {}", key));
                }
            }
        });

        let config = Self {
            local_name: conf_local_name.unwrap_or_else(|| {
                log_and_panic("local_name not provided");
                unreachable!();
            }),
            remote_names: conf_remote_names.unwrap_or_else(|| {
                log_and_panic("remote_names not provided");
                unreachable!()
            }),
            dir_name: conf_dir_name.unwrap_or_else(|| {
                log_and_panic("dir_name not provided");
                unreachable!()
            }),
        };

        Ok(config)
    }
}
