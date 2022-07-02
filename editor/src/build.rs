use fyrox::{
    core::{parking_lot::Mutex, pool::Handle},
    gui::{
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        scroll_viewer::ScrollViewerBuilder,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
use std::{
    io::Read,
    process::ChildStdout,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct BuildWindow {
    window: Handle<UiNode>,
    active: Arc<AtomicBool>,
    changed: Arc<AtomicBool>,
    log: Arc<Mutex<String>>,
    log_text: Handle<UiNode>,
}

impl BuildWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let log_text;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .can_minimize(false)
            .can_close(false)
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new())
                                .with_text("Please wait while your game is building...")
                                .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_content({
                                    log_text = TextBuilder::new(WidgetBuilder::new()).build(ctx);
                                    log_text
                                })
                                .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Building the Game..."))
            .build(ctx);

        Self {
            window,
            log_text,
            log: Arc::new(Default::default()),
            active: Arc::new(AtomicBool::new(false)),
            changed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn listen(&mut self, mut stdout: ChildStdout, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));

        let log = self.log.clone();
        self.active.store(true, Ordering::SeqCst);
        let reader_active = self.active.clone();
        let log_changed = self.changed.clone();
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            while reader_active.load(Ordering::SeqCst) {
                let mut slice = [0u8; 64];
                if let Ok(size) = stdout.read(&mut slice) {
                    buf.extend_from_slice(&slice[..size]);
                    if let Ok(string) = std::str::from_utf8(&buf) {
                        *log.lock() += string;
                        buf.clear();
                        log_changed.store(true, Ordering::SeqCst);
                    }
                }
            }
        });
    }

    pub fn reset(&mut self, ui: &UserInterface) {
        self.active.store(false, Ordering::SeqCst);
        self.changed.store(false, Ordering::SeqCst);
        self.log.lock().clear();
        ui.send_message(TextMessage::text(
            self.log_text,
            MessageDirection::ToWidget,
            Default::default(),
        ));
        ui.send_message(WindowMessage::close(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn update(&mut self, ui: &UserInterface) {
        if self.changed.load(Ordering::SeqCst) {
            ui.send_message(TextMessage::text(
                self.log_text,
                MessageDirection::ToWidget,
                self.log.lock().clone(),
            ));

            self.changed.store(false, Ordering::SeqCst);
        }
    }
}
