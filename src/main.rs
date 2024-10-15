use std::{fs, io};

// tmp-file-name: clipboard-<creator-name>-<timestamp>.tmp

fn read_from_clipboard() -> String {
    todo!()
}

fn write_to_clipboard(text: &str) {
    todo!()
}

struct Config {
    local_name: String,
    remote_names: Vec<String>,
    dir_name: String,
}

/// reads all filenames of the files in the config.dir_name directory. Here every osfile is
/// included.
fn get_all_files(config: &Config) -> Result<Vec<String>, std::io::Error> {
    let files: Vec<_> = fs::read_dir(&config.dir_name)?
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();

    Ok(files)
}

fn get_file_to_load(config: &Config) -> Option<String> {
    let files = get_all_files(&config).expect("could not read files");

    let mut most_recent_file = None;
    let mut most_recent_timestamp = None;

    'files: for file_name in files.iter().map(|f| f.as_str()) {
        let mut file = file_name;

        if !file.starts_with("clipboard-") {
            // not a clipboard file
            continue;
        }

        file = &file["clipboard-".len()..];

        for remote_name in &config.remote_names {
            if file.starts_with(remote_name) {
                file = &file[remote_name.len()..];

                // check if char after remote_name is a '-'. Otherwise remote_names that contain
                // others as a prefix are wrongly recognized
                if file.starts_with("-") {
                    file = &file[1..];
                    // found a file created by a instance we are listening to
                    // now this needs to be checked if it is the most recent file

                    if !file.ends_with(".tmp") {
                        eprintln!("file \"{}\" does not end with .tmp", file);
                        eprintln!("this should not happen unless manual files are created");
                        eprintln!("skipping this file\n");
                        continue 'files;
                    }

                    let timestamp_str = &file[..file.len() - ".tmp".len()];
                    let timestamp = timestamp_str
                        .parse::<u64>()
                        .expect("could not parse timestamp");

                    // found a newer file
                    if timestamp > most_recent_timestamp.unwrap_or(0) {
                        most_recent_timestamp = Some(timestamp);
                        most_recent_file = Some(file_name);
                    }
                }
            }
        }
    }

    most_recent_file.map(|f| f.to_string())
}

fn generate_file(config: &Config, text: &str) -> io::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("generation of a timestamp failed\n which means the system time is broken")
        .as_secs();

    let file_name = format!(
        "clipboard-{}-{}.tmp",
        config.local_name,
        timestamp.to_string()
    );

    let file_path = format!("{}/{}", config.dir_name, file_name);

    // check if there is already a file created from this instance. If so, delete it.
    try_delete_own_file(&config);

    fs::write(&file_path, text)
}

fn try_delete_own_file(config: &Config) {
    let files = get_all_files(&config).expect("could not read files");

    for file_name in files.iter().map(|f| f.as_str()) {
        if file_name.starts_with(format!("clipboard-{}", config.local_name).as_str()) {
            let file_path = format!("{}/{}", config.dir_name, file_name);
            fs::remove_file(&file_path).expect("could not delete file");

            // there should only be one file created by this instance
            break;
        }
    }
}

fn main() {
    let conf = Config {
        local_name: String::from("ubuntu"),
        remote_names: vec![String::from("win"), String::from("ubuntu")],
        dir_name: String::from("./"),
    };

    generate_file(&conf, "Hello, world!").expect("could not generate file");
    let f = get_file_to_load(&conf);
    dbg!(f);
}
