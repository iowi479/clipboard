mod config;
mod file_handlers;
mod global_hotkeys;
mod logfile;
mod utils;

use logfile::log_and_panic;

fn main() {
    let conf = config::Config::load();
    if let Err(e) = conf {
        log_and_panic(&format!("Could not load config file {}", &e));
        unreachable!();
    }

    let conf = conf.unwrap();

    let file_handler = file_handlers::FileHandler::new(conf);
    let action_sender = file_handlers::provide_file_handler(file_handler);

    global_hotkeys::set_action_sender(action_sender);

    let mut listener = global_hotkeys::KeyboardListener::new();
    listener.handle_input_events();
}
