# `cargo-target-dep`

Used to `cargo build` a different target from a build script.
Created to build Wasm binaries for use in [Spin](https://github.com/fermyon/spin) tests.

## Example Usage

`build.rs`
```rust
fn main() {
    // Builds the binary target in `tests/wasm-test/` into `wasm-test.wasm`
    cargo_target_dep::build_target_dep("tests/wasm-test", "wasm-test.wasm")
        .release()
        .target("wasm32-wasi")
        .build();
}
```
