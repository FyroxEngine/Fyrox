#[macro_use]
extern crate winapi;
extern crate byteorder;

pub mod error;
pub mod decoder;
pub mod buffer;
pub mod source;
pub mod device;
pub mod context;
pub mod pool;

use crate::{
    context::Context,
    source::{Source, SourceKind},
    buffer::{Buffer, BufferKind},
};
use std::{
    path::Path,
    time::Duration,
    sync::{Arc, Mutex},
};

fn main() {
    let context = Context::new().unwrap();
    let buffer = Buffer::new(Path::new("data/Sonic_Mayhem_Collapse.wav"), BufferKind::Stream).unwrap();
    let source = Source::new(SourceKind::Flat, Arc::new(Mutex::new(buffer)));
    context.lock().unwrap().add_source(source);

    loop {
        context.lock().unwrap().update().unwrap();
        std::thread::sleep(Duration::from_millis(30));
    }
}
