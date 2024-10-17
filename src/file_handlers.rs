use crate::config::Config;
use crate::global_hotkeys::LOADED_CLIPBOARD;
use std::{
    fs, io,
    sync::mpsc::{self, Sender},
    thread,
};

pub struct FileHandler {
    config: Config,
}

pub enum ClipboardAction {
    TryLoad,
    Store(String),
}

impl FileHandler {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// reads all filenames of the files in the config.dir_name directory. Here every osfile is
    /// included.
    fn get_all_files(&self) -> Result<Vec<String>, std::io::Error> {
        let files: Vec<_> = fs::read_dir(&self.config.dir_name)?
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect();

        Ok(files)
    }

    fn get_file_to_load(&self) -> Option<String> {
        let files = self.get_all_files().expect("could not read files");

        let mut most_recent_file = None;
        let mut most_recent_timestamp = None;

        'files: for file_name in files.iter().map(|f| f.as_str()) {
            let mut file = file_name;

            if !file.starts_with("clipboard-") {
                // not a clipboard file
                continue;
            }

            file = &file["clipboard-".len()..];

            for remote_name in &self.remote_names {
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

    fn generate_file(&self, text: &str) -> io::Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("generation of a timestamp failed\n which means the system time is broken")
            .as_secs();

        let file_name = format!(
            "clipboard-{}-{}.tmp",
            self.local_name,
            timestamp.to_string()
        );

        let file_path = format!("{}/{}", self.dir_name, file_name);

        // check if there is already a file created from this instance. If so, delete it.
        self.try_delete_own_file();

        fs::write(&file_path, text)
    }

    fn try_delete_own_file(&self) {
        let files = self.get_all_files().expect("could not read files");

        for file_name in files.iter().map(|f| f.as_str()) {
            if file_name.starts_with(format!("clipboard-{}", self.local_name).as_str()) {
                let file_path = format!("{}/{}", self.dir_name, file_name);
                fs::remove_file(&file_path).expect("could not delete file");

                // there should only be one file created by this instance
                break;
            }
        }
    }
}

pub fn provide_file_handler(handler: FileHandler) -> Sender<ClipboardAction> {
    // TODO: channels...
    let (action_sender, action_receiver) = mpsc::channel();
    let loaded_clipboard = &LOADED_CLIPBOARD;

    thread::spawn(move || loop {
        let action = action_receiver.recv();
        if let Err(e) = action {
            eprintln!("could not receive action: {}", e);
            eprintln!("stopping file handler");
            break;
        }

        match action.unwrap() {
            ClipboardAction::TryLoad => match handler.get_file_to_load() {
                None => {
                    *loaded_clipboard.lock().unwrap() = None;
                }
                Some(file_name) => {
                    let file_path = format!("{}/{}", handler.dir_name, file_name);
                    let content = fs::read_to_string(&file_path).expect("could not read file");
                    *loaded_clipboard.lock().unwrap() = Some(content);
                    handler.try_delete_own_file();
                }
            },
            ClipboardAction::Store(content) => {
                handler
                    .generate_file(&content)
                    .expect("could not generate file");
            }
        }
    });
    action_sender
}
