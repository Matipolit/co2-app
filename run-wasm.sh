#! /bin/bash

cargo build --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/debug/co2-app.wasm --out-dir web-bindgen --target web
python -m http.server 8080 --bind 127.0.0.1 --directory web-bindgen/
