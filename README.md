# Co2 app

A GUI app to monitor CO2 and TVOC concentration levels in my room, with a graph showing the CO2 history.  
Created using [Iced](https://github.com/iced-rs/iced) and [Plotters-iced](https://github.com/Joylei/plotters-iced) in Rust.

# Compile

    cargo build --release
    
The binary will be in /target/release/co2-app

# Compile for wasm

Ensure you have installed:
 - [Wasm-bindgen](https://github.com/rustwasm/wasm-bindgen)
 - [Binaryen](https://github.com/WebAssembly/binaryen)

Run:

    ./release-wasm.sh

The output will be in /web-bindgen
