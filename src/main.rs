mod config;
mod file_handlers;
mod global_hotkeys;

fn main() {
    let conf = config::Config::load().expect("Could not load config file");

    let file_handler = file_handlers::FileHandler::new(conf);
    let action_sender = file_handlers::provide_file_handler(file_handler);

    global_hotkeys::set_action_sender(action_sender);

    let mut listener = global_hotkeys::KeyboardListener::new();
    listener.handle_input_events();
}
