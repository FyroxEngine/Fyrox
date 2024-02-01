use fyrox::event_loop::EventLoop;
use fyroxed_base::Editor;

fn main() {
    Editor::new(None).run(EventLoop::new().unwrap())
}
