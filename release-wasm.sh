#! /bin/bash

cargo build --target wasm32-unknown-unknown --release
wasm-bindgen target/wasm32-unknown-unknown/release/co2-app.wasm --out-dir web-bindgen --target web
wasm-opt -Oz -o web-bindgen/co2-app_bg.wasm web-bindgen/co2-app_bg.wasm
