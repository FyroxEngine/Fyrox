use fyrox::engine::executor::Executor;
use game::GameConstructor;

fn main() {
    let mut executor = Executor::new();
    executor.add_plugin(GameConstructor);
    executor.run()
}
