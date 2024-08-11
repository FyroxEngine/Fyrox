// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use fyrox_resource::io::FsResourceIo;
use fyrox_sound::buffer::SoundBufferResourceExtension;
use fyrox_sound::{
    buffer::{DataSource, SoundBufferResource},
    context::SoundContext,
    engine::SoundEngine,
    futures::executor::block_on,
    pool::Handle,
    source::{SoundSource, SoundSourceBuilder, Status},
};
use std::{thread, time::Duration};

fn main() {
    // Initialize sound engine with default output device.
    let engine = SoundEngine::new().unwrap();

    // Initialize new sound context.
    let context = SoundContext::new();

    engine.state().add_context(context.clone());

    // Load sound buffer.
    let waterfall_buffer = SoundBufferResource::new_streaming(
        block_on(DataSource::from_file(
            "examples/data/waterfall.ogg",
            // Load from the default resource io (File system)
            &FsResourceIo,
        ))
        .unwrap(),
    )
    .unwrap();

    // Create flat source (without spatial effects) using that buffer.
    let source = SoundSourceBuilder::new()
        .with_buffer(waterfall_buffer)
        .with_status(Status::Playing)
        .with_looping(true)
        .build()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _source_handle: Handle<SoundSource> = context.state().add_source(source);

    thread::sleep(Duration::from_secs(30))
}
