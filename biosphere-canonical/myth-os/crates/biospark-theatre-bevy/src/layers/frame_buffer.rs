// THEATRE-FRAMEBUFFER: Thread-safe pixel store between a WebView thread and Bevy.
//
// The WebView background thread writes RGBA frames here at its own cadence (~30fps).
// Bevy's upload system reads and takes the latest frame each game tick.
//
// Uses Option<Vec<u8>> so the upload system can `take()` without copying —
// if no new frame arrived since last tick, the previous Bevy Image is kept as-is.

use std::sync::{Arc, Mutex};

/// Thread-safe RGBA pixel buffer.
///
/// Clone is cheap — all clones share the same underlying `Arc<Mutex<...>>`.
#[derive(Clone)]
pub struct FrameBuffer {
    inner: Arc<Mutex<Option<Vec<u8>>>>,
    pub width: u32,
    pub height: u32,
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
            width,
            height,
        }
    }

    /// Write a new RGBA frame. Called from the WebView thread.
    /// Overwrites any frame that hasn't been consumed by Bevy yet (oldest frame loses).
    pub fn write(&self, rgba: Vec<u8>) {
        debug_assert_eq!(
            rgba.len(),
            self.byte_count(),
            "FrameBuffer::write: pixel count mismatch — expected {} bytes, got {}",
            self.byte_count(),
            rgba.len()
        );
        *self.inner.lock().unwrap() = Some(rgba);
    }

    /// Take the latest frame if one is available. Called from the Bevy upload system.
    /// Returns `None` if no new frame has arrived since the last call.
    pub fn take(&self) -> Option<Vec<u8>> {
        self.inner.lock().unwrap().take()
    }

    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Size of a full RGBA frame in bytes.
    pub fn byte_count(&self) -> usize {
        self.pixel_count() * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_take_returns_frame() {
        let fb = FrameBuffer::new(4, 4);
        let data = vec![255u8; fb.byte_count()];
        fb.write(data.clone());
        assert_eq!(fb.take(), Some(data));
    }

    #[test]
    fn take_without_write_returns_none() {
        let fb = FrameBuffer::new(4, 4);
        assert_eq!(fb.take(), None);
    }

    #[test]
    fn second_take_returns_none() {
        let fb = FrameBuffer::new(4, 4);
        fb.write(vec![0u8; fb.byte_count()]);
        fb.take();
        assert_eq!(fb.take(), None);
    }

    #[test]
    fn clone_shares_buffer() {
        let fb = FrameBuffer::new(4, 4);
        let fb2 = fb.clone();
        fb.write(vec![42u8; fb.byte_count()]);
        let frame = fb2.take().unwrap();
        assert!(frame.iter().all(|&b| b == 42));
    }
}
