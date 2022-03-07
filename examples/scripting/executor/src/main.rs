use fyrox::engine::executor::Executor;
use game::GamePlugin;

fn main() {
    let mut executor = Executor::new();
    executor.add_plugin(GamePlugin::new());
    executor.run()
}
