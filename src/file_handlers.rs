use crate::config::Config;
use crate::global_hotkeys::LOADED_CLIPBOARD;
use crate::logfile::{log, log_and_panic};
use crate::utils::get_timestamp;
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
    fn get_all_files(&self) -> io::Result<Vec<String>> {
        let files: Vec<_> = fs::read_dir(&self.config.dir_name)?
            .map(|entry| {
                entry
                    .unwrap_or_else(|e| {
                        log_and_panic(&format!(
                            "could not read a file name. Something went wrong with the filesystem {}",
                            e
                        ));
                        unreachable!();
                    })
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        Ok(files)
    }

    fn get_file_to_load(&self) -> Option<String> {
        let files = self.get_all_files().unwrap_or_else(|e| {
            log_and_panic(&format!("could not read files {}", e));
            unreachable!();
        });

        let mut most_recent_file = None;
        let mut most_recent_timestamp = None;

        'files: for original_file_name in files.iter().map(|f| f.as_str()) {
            let mut file = original_file_name;

            if !file.starts_with("clipboard-") {
                // not a clipboard file
                continue;
            }

            file = &file["clipboard-".len()..];

            for remote_name in &self.config.remote_names {
                if file.starts_with(remote_name) {
                    file = &file[remote_name.len()..];

                    // check if char after remote_name is a '-'. Otherwise remote_names that contain
                    // others as a prefix are wrongly recognized
                    if file.starts_with("-") {
                        file = &file[1..];
                        // found a file created by a instance we are listening to
                        // now this needs to be checked if it is the most recent file

                        if !file.ends_with(".tmp") {
                            log(&format!(
                                "file \"{}\" does not end with .tmp\n
                                this should not happen unless manual files are created\n
                                skipping this file\n",
                                original_file_name
                            ));
                            continue 'files;
                        }

                        let timestamp_str = &file[..file.len() - ".tmp".len()];
                        let timestamp = timestamp_str.parse::<u64>().unwrap_or_else(|e| {
                            log_and_panic(&format!(
                                "could not parse timestamp {} {}",
                                timestamp_str, e
                            ));
                            unreachable!();
                        });

                        // found a newer file
                        if timestamp > most_recent_timestamp.unwrap_or(0) {
                            most_recent_timestamp = Some(timestamp);
                            most_recent_file = Some(original_file_name);
                        }
                    }
                }
            }
        }

        most_recent_file.map(|f| f.to_string())
    }

    fn generate_file(&self, text: &str) -> io::Result<()> {
        let file_name = format!(
            "clipboard-{}-{}.tmp",
            self.config.local_name,
            get_timestamp()
        );

        let file_path = format!("{}/{}", self.config.dir_name, file_name);

        // check if there is already a file created from this instance. If so, delete it.
        self.try_delete_own_file()?;

        fs::write(&file_path, text)
    }

    fn try_delete_own_file(&self) -> io::Result<()> {
        let files = self.get_all_files()?;

        for file_name in files.iter().map(|f| f.as_str()) {
            if file_name.starts_with(format!("clipboard-{}-", self.config.local_name).as_str()) {
                let file_path = format!("{}/{}", self.config.dir_name, file_name);
                fs::remove_file(&file_path)?;

                // there should only be one file created by this instance
                break;
            }
        }
        Ok(())
    }

    fn try_delete_file(&self, file_path: &str) -> io::Result<()> {
        fs::remove_file(&file_path)
    }
}

pub fn provide_file_handler(handler: FileHandler) -> Sender<ClipboardAction> {
    let (action_sender, action_receiver) = mpsc::channel();
    let loaded_clipboard = &LOADED_CLIPBOARD;

    thread::spawn(move || loop {
        let action = action_receiver.recv();
        if let Err(e) = action {
            log(&format!(
                "could not receive action: {}\nstopping file handler\n",
                e
            ));
            break;
        }

        match action.unwrap() {
            ClipboardAction::TryLoad => match handler.get_file_to_load() {
                None => {
                    *loaded_clipboard.lock().unwrap() = None;
                }
                Some(file_name) => {
                    let file_path = format!("{}/{}", handler.config.dir_name, file_name);
                    let content = fs::read_to_string(&file_path).unwrap_or_else(|e| {
                        log_and_panic(&format!("could not read file {} {}", file_path, e));
                        unreachable!();
                    });

                    *loaded_clipboard.lock().unwrap() = Some(content);
                    if let Err(e) = handler.try_delete_own_file() {
                        log_and_panic(&format!("could not delete own file: {}\n", e));
                    }

                    if let Err(e) = handler.try_delete_file(&file_name) {
                        log_and_panic(&format!("could not delete file: {}\n", e));
                    }
                }
            },
            ClipboardAction::Store(content) => {
                if let Err(e) = handler.generate_file(&content) {
                    log_and_panic(&format!("could not generate file: {}", e));
                }
            }
        }
    });
    action_sender
}
