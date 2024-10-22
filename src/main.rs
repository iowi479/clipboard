mod config;
mod file_handlers;
mod global_hotkeys;
mod logfile;
mod utils;

use logfile::log_and_panic;

fn main() {
    let conf = config::Config::load().unwrap_or_else(|e| {
        log_and_panic(&format!("Could not load config file {}", &e));
        unreachable!();
    });

    let file_handler = file_handlers::FileHandler::new(conf);
    let action_sender = file_handlers::provide_file_handler(file_handler);

    global_hotkeys::set_action_sender(action_sender).unwrap_or_else(|e| {
        log_and_panic(&format!("Could not set action-sender {}", &e));
        unreachable!();
    });

    let mut listener = global_hotkeys::KeyboardListener::new();
    listener.handle_input_events();
}
