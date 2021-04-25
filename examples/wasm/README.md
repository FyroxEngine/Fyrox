## WebAssembly Example

Simple example showing how to use rg3d with WebAssembly.

### Build instructions

1. Make sure you have `wasm32-unknown-unknown` target installed in rustup (if not, do: `rustup target add wasm32-unknown-unknown`)
2. Make sure you have `wasm-pack` installed (if not, do: `cargo install wasm-pack`)
3. Make sure you have `npm` installed (if not, install it from here: https://nodejs.org/en/download/)
4. Finally execute this:

```shell
wasm-pack build
cd www
npm install
npm run start
```

If everything has succeded, open web browser at http://localhost:8080/ and you should see a white cube which 
can be rotated using A D keys.