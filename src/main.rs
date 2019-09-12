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
    source::Source,
    buffer::{Buffer, BufferKind}
};
use std::{
    path::Path,
    time::Duration,
    sync::{Arc, Mutex}
};

fn main() {
    let context = Context::new().unwrap();
    let buffer = Buffer::new(Path::new("data/Sonic_Mayhem_Collapse.wav"), BufferKind::Normal).unwrap();
    let source = Source::new(Arc::new(Mutex::new(buffer)));
    context.lock().unwrap().add_source(source);


    std::thread::sleep(Duration::new(20, 0));
}
