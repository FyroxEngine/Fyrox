use crate::{
    renderer::framework::gl::types::{GLchar, GLenum, GLsizei, GLuint},
    utils::log::{Log, MessageKind},
};
use std::ffi::CStr;

#[allow(clippy::all)]
pub(in crate) mod gl;

macro_rules! check_gl_error {
    () => {
        crate::renderer::framework::check_gl_error_internal(line!(), file!())
    };
}

pub mod framebuffer;
pub mod geometry_buffer;
pub mod gpu_program;
pub mod gpu_texture;
pub mod state;

pub fn check_gl_error_internal(line: u32, file: &str) {
    unsafe {
        let error_code = gl::GetError();
        if error_code != gl::NO_ERROR {
            let code = match error_code {
                gl::INVALID_ENUM => "GL_INVALID_ENUM",
                gl::INVALID_VALUE => "GL_INVALID_VALUE",
                gl::INVALID_OPERATION => "GL_INVALID_OPERATION",
                gl::STACK_OVERFLOW => "GL_STACK_OVERFLOW",
                gl::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
                gl::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
                _ => "Unknown",
            };

            Log::writeln(
                MessageKind::Error,
                format!(
                    "{} error has occurred! At line {} in file {}, stability is not guaranteed!",
                    code, line, file
                ),
            );

            if gl::GetDebugMessageLog::is_loaded() {
                let mut max_message_length = 0;
                gl::GetIntegerv(gl::MAX_DEBUG_MESSAGE_LENGTH, &mut max_message_length);

                let mut max_logged_messages = 0;
                gl::GetIntegerv(gl::MAX_DEBUG_LOGGED_MESSAGES, &mut max_logged_messages);

                let buffer_size = max_message_length * max_logged_messages;

                let mut message_buffer: Vec<GLchar> = Vec::with_capacity(buffer_size as usize);
                message_buffer.set_len(buffer_size as usize);

                let mut sources: Vec<GLenum> = Vec::with_capacity(max_logged_messages as usize);
                sources.set_len(max_logged_messages as usize);

                let mut types: Vec<GLenum> = Vec::with_capacity(max_logged_messages as usize);
                types.set_len(max_logged_messages as usize);

                let mut ids: Vec<GLuint> = Vec::with_capacity(max_logged_messages as usize);
                ids.set_len(max_logged_messages as usize);

                let mut severities: Vec<GLenum> = Vec::with_capacity(max_logged_messages as usize);
                severities.set_len(max_logged_messages as usize);

                let mut lengths: Vec<GLsizei> = Vec::with_capacity(max_logged_messages as usize);
                lengths.set_len(max_logged_messages as usize);

                let message_count = gl::GetDebugMessageLog(
                    max_logged_messages as u32,
                    buffer_size,
                    sources.as_mut_ptr(),
                    types.as_mut_ptr(),
                    ids.as_mut_ptr(),
                    severities.as_mut_ptr(),
                    lengths.as_mut_ptr(),
                    message_buffer.as_mut_ptr(),
                );

                if message_count == 0 {
                    Log::writeln(
                        MessageKind::Warning,
                        "Debug info is not available - run with OpenGL debug flag!".to_owned(),
                    );
                }

                let mut message = message_buffer.as_ptr();

                for i in 0..message_count as usize {
                    let source = sources[i];
                    let ty = types[i];
                    let severity = severities[i];
                    let id = ids[i];
                    let len = lengths[i] as usize;

                    let source_str = match source {
                        gl::DEBUG_SOURCE_API => "API",
                        gl::DEBUG_SOURCE_SHADER_COMPILER => "Shader Compiler",
                        gl::DEBUG_SOURCE_WINDOW_SYSTEM => "Window System",
                        gl::DEBUG_SOURCE_THIRD_PARTY => "Third Party",
                        gl::DEBUG_SOURCE_APPLICATION => "Application",
                        gl::DEBUG_SOURCE_OTHER => "Other",
                        _ => "Unknown",
                    };

                    let type_str = match ty {
                        gl::DEBUG_TYPE_ERROR => "Error",
                        gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated Behavior",
                        gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined Behavior",
                        gl::DEBUG_TYPE_PERFORMANCE => "Performance",
                        gl::DEBUG_TYPE_PORTABILITY => "Portability",
                        gl::DEBUG_TYPE_OTHER => "Other",
                        _ => "Unknown",
                    };

                    let severity_str = match severity {
                        gl::DEBUG_SEVERITY_HIGH => "High",
                        gl::DEBUG_SEVERITY_MEDIUM => "Medium",
                        gl::DEBUG_SEVERITY_LOW => "Low",
                        gl::DEBUG_SEVERITY_NOTIFICATION => "Notification",
                        _ => "Unknown",
                    };

                    let str_msg = CStr::from_ptr(message);

                    Log::writeln(MessageKind::Information,
                                 format!("OpenGL message\nSource: {}\nType: {}\nId: {}\nSeverity: {}\nMessage: {:?}\n",
                                         source_str,
                                         type_str,
                                         id,
                                         severity_str,
                                         str_msg));

                    message = message.add(len);
                }
            } else {
                Log::writeln(
                    MessageKind::Warning,
                    "Debug info is not available - glGetDebugMessageLog is not available!"
                        .to_owned(),
                );
            }
        }
    }
}
