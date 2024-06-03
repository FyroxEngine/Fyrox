## Build instructions

1. Make sure you have `wasm32-unknown-unknown` target installed in rustup (if not, do: `rustup target add wasm32-unknown-unknown`)
2. Make sure you have `wasm-pack` installed (if not, do: `cargo install wasm-pack`)
3. To build the executor, do: `wasm-pack build --target web --release`

## How to run the game on localhost

1. Make sure you have `basic-http-server` installed (if not, do: `cargo install basic-http-server`). 
2. Clone assets to the `executor-wasm` directory. Alternatively, clone everything except `Cargo.toml` and `src` directory
to the root of your project (`../`).
3. Execute `basic-http-server` in `executor-wasm` directory (or in root folder if you you've used alternative path).

If everything has succeeded, open a web browser at http://localhost:4000/, click "Start" button and your game shoud load.