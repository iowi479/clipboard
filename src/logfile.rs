use crate::utils::get_timestamp;
use std::fs;
use std::io::Write;

static LOGFILE: &str = "log-clipboard-current.tmp";

pub fn log(content: &str) {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(LOGFILE)
        .expect("could not open logfile");

    file.write(content.as_bytes())
        .expect("could not write to logfile");

    println!("{}", content);
}

pub fn log_and_panic(error: &str) {
    log(error);
    let file_name = format!("crash-clipboard-{}.log", get_timestamp());
    fs::rename(LOGFILE, &file_name).expect("could not rename logfile");
    panic!("See logfile {}", file_name);
}
