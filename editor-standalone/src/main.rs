use fyrox::event_loop::EventLoop;
use fyroxed_base::Editor;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let editor = Editor::new(&event_loop, None);
    editor.run(event_loop)
}
