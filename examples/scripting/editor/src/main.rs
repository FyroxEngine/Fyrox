use fyrox::event_loop::EventLoop;
use fyroxed_base::{Editor, StartupData};
use game::GamePlugin;

fn main() {
    let event_loop = EventLoop::new();
    let mut editor = Editor::new(
        &event_loop,
        Some(StartupData {
            working_directory: Default::default(),
            scene: "data/scene.rgs".into(),
        }),
    );
    editor.add_game_plugin(GamePlugin::new());
    editor.run(event_loop)
}
