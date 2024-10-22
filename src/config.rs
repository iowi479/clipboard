use anyhow::{anyhow, bail, Context, Result};

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
    pub fn load() -> Result<Self> {
        let content = std::fs::read_to_string(CONFIG_FILE_NAME)
            .with_context(|| format!("Looking for config-file at: {}", CONFIG_FILE_NAME))?;

        let mut conf_local_name = None;
        let mut conf_remote_names = None;
        let mut conf_dir_name = None;

        for (i, line) in content.lines().enumerate() {
            // skip empty lines or comments
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            let mut parts = line.split("=");
            let key = parts
                .next()
                .with_context(|| {
                    format!("no key provided on line {} in config file:\n{}", i, line)
                })?
                .trim();

            let value = parts
                .next()
                .with_context(|| {
                    format!("no value provided on line {} in config file:\n{}", i, line)
                })?
                .trim();

            if parts.next().is_some() {
                bail!("too many parts on line {} in config file:\n{}", i, line);
            }

            if value.is_empty() {
                bail!("no value provided on line {} in config file:\n{}", i, line);
            }

            match key {
                "local_name" => {
                    if conf_local_name.is_some() {
                        bail!("local_name is a duplicate");
                    }
                    conf_local_name = Some(value.to_string());
                }
                "remote_names" => {
                    if conf_remote_names.is_some() {
                        bail!("remote_names is a duplicate");
                    }
                    conf_remote_names =
                        Some(value.split(",").map(|s| s.trim().to_string()).collect());
                }
                "dir_name" => {
                    if conf_dir_name.is_some() {
                        bail!("dir_name is a duplicate");
                    }

                    conf_dir_name = Some(value.to_string());

                    std::fs::read_dir(value).with_context(|| {
                        format!("Could not read specified directory: {}", value)
                    })?;
                }
                _ => {
                    bail!(
                        "unknown key {} on line {} in config file:\n{}",
                        key,
                        i,
                        line
                    );
                }
            }
        }

        let local_name = conf_local_name.ok_or_else(|| anyhow!("local_name not provided"))?;
        let remote_names = conf_remote_names.ok_or_else(|| anyhow!("remote_names not provided"))?;

        for r_name in &remote_names {
            if r_name == &local_name {
                bail!("remote_names contains local_name which is invalid");
            }
        }

        let config = Self {
            local_name,
            remote_names,
            dir_name: conf_dir_name.ok_or_else(|| anyhow!("dir_name not provided"))?,
        };

        Ok(config)
    }
}
