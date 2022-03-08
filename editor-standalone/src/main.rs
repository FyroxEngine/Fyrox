use fyrox::event_loop::EventLoop;
use fyroxed::Editor;

fn main() {
    let event_loop = EventLoop::new();
    let editor = Editor::new(&event_loop, None);
    editor.run(event_loop)
}
