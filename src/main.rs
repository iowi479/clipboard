mod config;
mod file_handlers;
mod global_hotkeys;

fn main() {
    let conf = config::Config::load().expect("Could not load config file");

    let file_handler = file_handlers::FileHandler::new(conf);
    let sender = file_handlers::provide_file_handler(file_handler);

    *global_hotkeys::CLIPBOARD_ACTION_SENDER.lock().unwrap() = Some(sender);

    let mut listener = global_hotkeys::KeyboardListener::new();
    listener.handle_input_events();
}
