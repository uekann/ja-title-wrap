set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

crate_manifest := "ja-title-wrap-core/Cargo.toml"
target_wasm := "ja-title-wrap-core/target/wasm32-unknown-unknown/release/ja_title_wrap_core.wasm"
plugin_wasm := "plugin/ja_title_wrap_core.wasm"

default: build-plugin

build-plugin:
    cargo build --manifest-path {{crate_manifest}} --target wasm32-unknown-unknown --release
    mkdir -p plugin
    cp {{target_wasm}} {{plugin_wasm}}

test:
    cargo test --manifest-path {{crate_manifest}}

demo:
    just build-plugin
    typst compile --root . examples/demo.typ examples/demo.pdf
