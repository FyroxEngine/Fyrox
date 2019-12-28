use crate::{
    hrtf::HrtfRenderer,
    source::{Source, Status}
};

pub enum Renderer {
    /// Stateless default renderer.
    Default,
    /// Can be used *only* with mono sounds, stereo sounds will be rendered through
    /// default renderer.
    HrtfRenderer(HrtfRenderer),
}

pub(in crate) fn render_source_default(source: &mut Source, mix_buffer: &mut [(f32, f32)]) {
    if source.get_status() != Status::Playing {
        return;
    }

    if let Some(buffer) = source.get_buffer() {
        if let Ok(mut buffer) = buffer.lock() {
            if buffer.is_empty() {
                return;
            }

            for (left, right) in mix_buffer {
                if source.get_status() != Status::Playing {
                    break;
                }

                let (raw_left, raw_right) = source.next_sample_pair(&mut buffer);

                *left += source.left_gain * raw_left;
                *right += source.right_gain * raw_right;
            }
        };
    }
}
