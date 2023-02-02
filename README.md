# RVCD

VCD wave file viewer written in Rust.

With egui, rvcd can be compiled to win/linux/web.

## Usage

TODO

## Compile

### Windows / Linux

```
# run rvcd
cargo run
# compile to release executable
cargo build --release
```

### Web

```
# install wasm target
rustup target add wasm32-unknown-unknown
# install cli tool
cargo install trunk wasm-bindgen-cli
# dynamical run in debug
trunk serve
# build release static files
trunk build --release
```
