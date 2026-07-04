// THEATRE-WEBVIEW: WebViewLayer — P5 and HT layers rendered off-screen and streamed
// into a FrameBuffer as RGBA pixel data at ~30fps.
//
// Phase 3 uses a software mock renderer (animated gradient) so the full pipeline
// compiles and runs without a WebView dependency. The mock proves:
//   background thread → FrameBuffer → Bevy Image upload → textured quad → compositor
//
// TODO(phase-3-wry): Replace `run_mock_renderer` with `run_wry_renderer`:
//   1. Call CoInitializeEx(COINIT_APARTMENTTHREADED) on Windows
//   2. Create a hidden winit Window on the background thread
//   3. Build a wry WebView with `with_html(build_p5_html(&glyph_code))`
//   4. Install an IPC handler: receives base64 PNG → decodes → fb.write(rgba)
//   5. Run the winit EventLoop; inject captureFrame() calls via evaluate_script()
//   See `build_p5_html()` below for the JS injection pattern.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use biospark_theatre::{FrameContext, LayerType, OutputHandler, TheatreError};
use myth_wire::ChannelId;

use super::frame_buffer::FrameBuffer;

// ── WebViewLayer ──────────────────────────────────────────────────────────────

/// A P5 or HT layer that renders into a FrameBuffer from a background thread.
///
/// Owns the background thread's lifetime via the `running` flag — dropping the
/// WebViewLayer signals the thread to exit and join naturally.
pub struct WebViewLayer {
    pub layer_type: LayerType,
    /// Shared pixel buffer — clone this to pass to the Bevy upload system.
    pub frame_buffer: FrameBuffer,
    pub channel_id: ChannelId,
    running: Arc<AtomicBool>,
    level: f32,
    tint: [f32; 4],
    muted: bool,
}

impl WebViewLayer {
    /// Spawn a new WebView layer for the given channel.
    ///
    /// Immediately starts the background renderer thread.
    /// `code` is the glyph's rendering code (p5 sketch or HTML document).
    /// `width` / `height` are the capture resolution — typically half the canvas
    /// size (640×360 for a 1280×720 Theatre) to keep frame transfer cheap.
    pub fn new(
        channel_id: ChannelId,
        layer_type: LayerType,
        width: u32,
        height: u32,
        code: String,
    ) -> Self {
        assert!(
            matches!(layer_type, LayerType::P5 | LayerType::Ht),
            "WebViewLayer only handles P5 (p5.js) and HT (HTML) layer types"
        );

        let fb = FrameBuffer::new(width, height);
        let fb_writer = fb.clone();
        let running = Arc::new(AtomicBool::new(true));
        let stop_flag = running.clone();

        thread::Builder::new()
            .name(format!(
                "{}-layer-ch{}",
                layer_type.tag().to_lowercase(),
                channel_id.get()
            ))
            .spawn(move || run_mock_renderer(fb_writer, stop_flag, layer_type, code))
            .expect("failed to spawn WebView renderer thread");

        Self {
            layer_type,
            frame_buffer: fb,
            channel_id,
            running,
            level: 1.0,
            tint: [1.0, 1.0, 1.0, 1.0],
            muted: false,
        }
    }
}

impl OutputHandler for WebViewLayer {
    fn layer_type(&self) -> LayerType {
        self.layer_type
    }

    /// No-op for WebView layers — the background thread generates frames continuously.
    /// The Bevy upload system reads the FrameBuffer directly each game tick.
    fn render(&mut self, _ctx: &FrameContext) -> Result<(), TheatreError> {
        Ok(())
    }

    fn set_level(&mut self, level: f32) {
        self.level = level.clamp(0.0, 1.0);
    }

    fn set_tint(&mut self, tint: [f32; 4]) {
        self.tint = tint;
    }

    fn mute(&mut self) {
        self.muted = true;
    }

    fn unmute(&mut self) {
        self.muted = false;
    }

    fn is_muted(&self) -> bool {
        self.muted
    }
}

impl Drop for WebViewLayer {
    /// Signal the background thread to stop when this layer is dropped.
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

// ── Mock renderer (Phase 3) ───────────────────────────────────────────────────

/// Generates animated RGBA frames at ~30fps and writes them to the FrameBuffer.
///
/// P5 layers → flowing organic wave (blue-violet).
/// HT layers → glowing tile grid (teal-indigo).
///
/// This function runs until `running` is set to false (WebViewLayer dropped).
fn run_mock_renderer(
    fb: FrameBuffer,
    running: Arc<AtomicBool>,
    layer_type: LayerType,
    _code: String,
) {
    const TARGET_FPS: u64 = 30;
    let frame_budget = Duration::from_millis(1000 / TARGET_FPS);

    let mut tick = 0u64;
    while running.load(Ordering::Relaxed) {
        let frame = generate_frame(fb.width, fb.height, tick, layer_type);
        fb.write(frame);
        tick += 1;
        thread::sleep(frame_budget);
    }
}

fn generate_frame(width: u32, height: u32, tick: u64, layer_type: LayerType) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let t = tick as f32 * 0.038;

    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let fx = x as f32 / width as f32;
            let fy = y as f32 / height as f32;

            let (r, g, b, a) = match layer_type {
                // P5 — flowing organic wave: blue → violet
                LayerType::P5 => {
                    let w1 = ((fx * 7.5 + t).sin() * 0.5 + 0.5)
                        * ((fy * 4.8 - t * 0.55).cos() * 0.5 + 0.5);
                    let w2 = (fx * 2.8 - t * 0.35 + fy * 3.7).sin() * 0.5 + 0.5;
                    let v  = w1 * 0.6 + w2 * 0.4;
                    (
                        (v * 70.0 + 15.0) as u8,
                        (v * 20.0 + 5.0) as u8,
                        (v * 200.0 + 40.0) as u8,
                        190u8,
                    )
                }
                // HT — glowing tile grid: teal grid lines + pulsing cells
                LayerType::Ht => {
                    let cell_x = (fx * 14.0).fract();
                    let cell_y = (fy * 8.0).fract();
                    let grid_x = (fx * 14.0).floor();
                    let grid_y = (fy * 8.0).floor();
                    let is_border = cell_x < 0.06 || cell_y < 0.06;
                    let pulse = (grid_x * 2.3 + grid_y * 3.7 + t).sin() * 0.5 + 0.5;

                    if is_border {
                        (20, 160, 170, 220)
                    } else {
                        (
                            (pulse * 25.0 + 8.0) as u8,
                            (pulse * 90.0 + 20.0) as u8,
                            (pulse * 120.0 + 30.0) as u8,
                            160u8,
                        )
                    }
                }
                _ => (0, 0, 0, 0),
            };

            pixels[idx]     = r;
            pixels[idx + 1] = g;
            pixels[idx + 2] = b;
            pixels[idx + 3] = a;
        }
    }

    pixels
}

// ── Wry integration stub (Phase 3 → wry) ─────────────────────────────────────

/// Build the full HTML document that hosts a p5.js sketch inside a wry WebView.
///
/// Injects `_captureFrame()` at the end of the user's `draw()` call.
/// The IPC handler in `run_wry_renderer` receives base64 PNG strings and decodes
/// them to RGBA bytes before writing to the FrameBuffer.
///
/// Used by `run_wry_renderer` (not yet active — see TODO above).
#[allow(dead_code)]
pub fn build_p5_html(user_code: &str, width: u32, height: u32) -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head>
<style>body{{margin:0;overflow:hidden;background:transparent}}</style>
</head><body>
<script src="https://cdnjs.cloudflare.com/ajax/libs/p5.js/1.9.4/p5.min.js"></script>
<script>
// ── User sketch ───────────────────────────────────────────────────────────
{user_code}

// ── Frame capture (injected by Theatre) ──────────────────────────────────
const _userSetup = typeof setup !== 'undefined' ? setup.bind(window) : null;
const _userDraw  = typeof draw  !== 'undefined' ? draw.bind(window)  : null;

function setup() {{
  createCanvas({width}, {height});
  if (_userSetup) _userSetup();
}}
function draw() {{
  if (_userDraw) _userDraw();
  const c = document.querySelector('canvas');
  if (c) window.ipc.postMessage(c.toDataURL('image/png'));
}}
</script>
</body></html>"#,
        user_code = user_code,
        width = width,
        height = height,
    )
}

/// Build an HTML document wrapper for generic HTML/CSS content.
#[allow(dead_code)]
pub fn build_html_doc(user_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html><head>
<style>
  html,body{{margin:0;padding:0;overflow:hidden;background:transparent}}
  *{{box-sizing:border-box}}
</style>
<script>
// Capture the page as a PNG every ~33ms and send via IPC
setInterval(function() {{
  const canvas = document.createElement('canvas');
  canvas.width  = document.body.scrollWidth;
  canvas.height = document.body.scrollHeight;
  const ctx = canvas.getContext('2d');
  // drawImage of the document is not possible without html2canvas in a real page;
  // in a wry WebView, use WebView2's CapturePreview API instead.
  // This stub exists to show the intended IPC shape.
  window.ipc.postMessage(canvas.toDataURL('image/png'));
}}, 33);
</script>
</head><body>
{user_html}
</body></html>"#,
        user_html = user_html,
    )
}
