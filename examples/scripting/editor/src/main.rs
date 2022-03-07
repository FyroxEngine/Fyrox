use fyrox::event_loop::EventLoop;
use fyroxed::Editor;
use game::GamePlugin;

fn main() {
    let event_loop = EventLoop::new();
    let mut editor = Editor::new(&event_loop);
    editor.add_game_plugin(GamePlugin::new());
    editor.run(event_loop)
}
