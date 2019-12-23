use std::{
    sync::Mutex, fs::File,
    io::{Write, self},
};

lazy_static! {
    static ref LOG_FILE: Mutex<File> = {
         Mutex::new(File::create("rg3d.log").unwrap())
    };
}

pub struct Log {}

impl Log {
    pub fn write(msg: String) {
        let _ = io::stdout().write_all(msg.as_bytes());
        let _ = LOG_FILE.lock().unwrap().write_all(msg.as_bytes());
    }

    pub fn writeln(mut msg: String) {
        msg.push('\n');
        Self::write(msg)
    }
}