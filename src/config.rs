use std::fs;
use std::io;

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

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("generation of a timestamp failed\n which means the system time is broken")
            .as_secs();

        let mut conf_local_name = timestamp.to_string();
        let mut conf_remote_names = Vec::new();
        let mut conf_dir_name = String::from("./");

        content.lines().for_each(|line| {
            // skip empty lines
            if line.is_empty() {
                return;
            }

            let mut parts = line.split("=");
            let key = parts.next().unwrap().trim();
            let value = parts.next().unwrap().trim();

            match key {
                "local_name" => {
                    conf_local_name = value.to_string();
                }
                "remote_names" => {
                    conf_remote_names = value.split(",").map(|s| s.trim().to_string()).collect();
                }
                "dir_name" => {
                    conf_dir_name = value.to_string();
                    if let Err(e) = std::fs::read_dir(&conf_dir_name) {
                        panic!(
                            "Could not read specified directory: {}\n{}",
                            conf_dir_name, e
                        );
                    }
                }
                _ => {
                    panic!("Unknown key in config file: {}", key);
                }
            }
        });

        let config = Self {
            local_name: conf_local_name,
            remote_names: conf_remote_names,
            dir_name: conf_dir_name,
        };

        Ok(config)
    }
}
