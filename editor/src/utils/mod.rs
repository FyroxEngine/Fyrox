use crate::WindowEvent;
use fyrox::gui::file_browser::FileSelectorMessage;
use fyrox::gui::window::WindowMessage;
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    event::Event,
    gui::{
        file_browser::{FileBrowserMode, FileSelectorBuilder, Filter},
        message::MessageDirection,
        widget::{WidgetBuilder, WidgetMessage},
        window::{Window, WindowBuilder},
        BuildContext, UiNode, UserInterface,
    },
};

pub mod path_fixer;

pub fn is_slice_equal_permutation<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    if a.is_empty() && !b.is_empty() {
        false
    } else {
        // TODO: Find a way to do this faster.
        for source in a.iter() {
            let mut found = false;
            for other in b.iter() {
                if other == source {
                    found = true;
                    break;
                }
            }
            if !found {
                return false;
            }
        }
        true
    }
}

pub fn window_content(window: Handle<UiNode>, ui: &UserInterface) -> Handle<UiNode> {
    ui.node(window)
        .cast::<Window>()
        .map(|w| w.content())
        .unwrap_or_default()
}

pub fn enable_widget(handle: Handle<UiNode>, state: bool, ui: &UserInterface) {
    ui.send_message(WidgetMessage::enabled(
        handle,
        MessageDirection::ToWidget,
        state,
    ));
}

pub fn normalize_os_event(
    result: &mut Event<()>,
    frame_position: Vector2<f32>,
    frame_size: Vector2<f32>,
) {
    if let Event::WindowEvent { event, .. } = result {
        match event {
            WindowEvent::Resized(size) => {
                size.width = frame_size.x as u32;
                size.height = frame_size.y as u32;
            }
            WindowEvent::Moved(position) => {
                position.x -= frame_position.x as i32;
                position.y -= frame_position.y as i32;
            }
            WindowEvent::CursorMoved { position, .. } => {
                position.x -= frame_position.x as f64;
                position.y -= frame_position.y as f64;
            }
            WindowEvent::Touch(touch) => {
                touch.location.x -= frame_position.x as f64;
                touch.location.y -= frame_position.y as f64;
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                new_inner_size.width = frame_size.x as u32;
                new_inner_size.height = frame_size.y as u32;
            }
            _ => (),
        }
    }
}

pub fn create_file_selector(
    ctx: &mut BuildContext,
    extension: &'static str,
    mode: FileBrowserMode,
) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0)).open(false),
    )
    .with_filter(Filter::new(move |path| {
        if let Some(ext) = path.extension() {
            ext.to_string_lossy().as_ref() == extension
        } else {
            path.is_dir()
        }
    }))
    .with_mode(mode)
    .build(ctx)
}

pub fn open_file_selector(file_selector: Handle<UiNode>, ui: &UserInterface) {
    ui.send_message(FileSelectorMessage::root(
        file_selector,
        MessageDirection::ToWidget,
        Some(std::env::current_dir().unwrap()),
    ));

    ui.send_message(WindowMessage::open_modal(
        file_selector,
        MessageDirection::ToWidget,
        true,
    ));
}
