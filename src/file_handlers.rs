use crate::config::Config;
use crate::global_hotkeys::LOADED_CLIPBOARD;
use crate::logfile::{log, log_and_panic};
use crate::utils::get_timestamp;
use anyhow::{Context, Result};
use std::sync::mpsc::Receiver;
use std::sync::Mutex;
use std::{
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
    fn get_all_files(&self) -> Result<Vec<String>> {
        let files: Result<Vec<_>> = std::fs::read_dir(&self.config.dir_name)
            .with_context(|| format!("tried to read {}", self.config.dir_name))?
            .map(|entry| {
                Ok(entry
                    .with_context(|| {
                        "could not read a file name. Something went wrong with the filesystem"
                    })?
                    .file_name()
                    .to_string_lossy()
                    .to_string())
            })
            .collect();

        Ok(files?)
    }

    fn get_file_to_load(&self) -> Result<Option<String>> {
        let files = self.get_all_files()?;

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
                        let timestamp = timestamp_str
                            .parse::<u64>()
                            .with_context(|| "could not parse timestamp")?;

                        // found a newer file
                        if timestamp > most_recent_timestamp.unwrap_or(0) {
                            most_recent_timestamp = Some(timestamp);
                            most_recent_file = Some(original_file_name);
                        }
                    }
                }
            }
        }

        Ok(most_recent_file.map(|f| f.to_string()))
    }

    fn generate_file(&self, text: &str) -> Result<()> {
        let file_name = format!(
            "clipboard-{}-{}.tmp",
            self.config.local_name,
            get_timestamp()
        );

        let file_path = format!("{}/{}", self.config.dir_name, file_name);

        // check if there is already a file created from this instance. If so, delete it.
        self.try_delete_own_file()?;

        std::fs::write(&file_path, text)
            .with_context(|| format!("could not write to file {}", file_path))
    }

    fn try_delete_own_file(&self) -> Result<()> {
        let files = self.get_all_files()?;

        for file_name in files.iter().map(|f| f.as_str()) {
            if file_name.starts_with(format!("clipboard-{}-", self.config.local_name).as_str()) {
                let file_path = format!("{}/{}", self.config.dir_name, file_name);
                std::fs::remove_file(&file_path)?;

                // there should only be one file created by this instance
                break;
            }
        }
        Ok(())
    }

    fn try_delete_file(&self, file_path: &str) -> Result<()> {
        std::fs::remove_file(&file_path)?;
        Ok(())
    }
}

pub fn provide_file_handler(handler: FileHandler) -> Sender<ClipboardAction> {
    let (action_sender, action_receiver) = mpsc::channel();
    let loaded_clipboard = &LOADED_CLIPBOARD;

    thread::spawn(move || action_handler(action_receiver, handler, loaded_clipboard));

    action_sender
}

fn action_handler(
    action_receiver: Receiver<ClipboardAction>,
    handler: FileHandler,
    loaded_clipboard: &Mutex<Option<String>>,
) {
    loop {
        let action = action_receiver.recv();
        if let Err(e) = action {
            log_and_panic(&format!(
                "could not receive action: {}\nstopping file handler\n",
                e
            ));
            break;
        }

        match action.unwrap() {
            ClipboardAction::TryLoad => match handler.get_file_to_load() {
                Err(e) => {
                    log_and_panic(&e.to_string());
                }
                Ok(None) => {
                    *loaded_clipboard.lock().unwrap() = None;
                }
                Ok(Some(file_name)) => {
                    let file_path = format!("{}/{}", handler.config.dir_name, file_name);
                    let content = std::fs::read_to_string(&file_path).unwrap_or_else(|e| {
                        log_and_panic(&format!("could not read file {} {}", file_path, e));
                        unreachable!();
                    });

                    *loaded_clipboard.lock().unwrap() = Some(content);
                    if let Err(e) = handler.try_delete_own_file() {
                        log_and_panic(&format!("could not delete own file: {}\n", e));
                    }

                    if let Err(e) = handler.try_delete_file(&file_name) {
                        log(&format!("could not delete file: {}\nThis is ignored since the program will run fine. But it will leave useless .tmp files behind.", e));
                    }
                }
            },
            ClipboardAction::Store(content) => {
                if let Err(e) = handler.generate_file(&content) {
                    log_and_panic(&format!("could not generate file: {}", e));
                }
            }
        }
    }
}
