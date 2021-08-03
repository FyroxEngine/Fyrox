## WebAssembly Example

Simple example showing how to use rg3d with WebAssembly.

### Build instructions

1. Make sure you have `wasm32-unknown-unknown` target installed in rustup (if not, do: `rustup target add wasm32-unknown-unknown`)
2. Make sure you have `wasm-pack` installed (if not, do: `cargo install wasm-pack`)
3. Make sure you have `basic-http-server` installed (if not do: `cargo install basic-http-server`)
4. Finally, execute this:

```shell
wasm-pack build --target web
basic-http-server
```

If everything has succeeded, open web browser at http://localhost:4000/ and you should a scene with 3d model which 
can be rotated using A D keys. 