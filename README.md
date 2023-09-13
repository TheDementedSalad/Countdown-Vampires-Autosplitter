# Countdown-Vampires-Autosplitter

IGT Autosplitter for Countdown Vampires (1999)

Has door splits and item splits.


## Release

The current release will always be at:

https://github.com/TheDementedSalad/Resident-Evil-Survivor-Autosplitter/releases/latest/download/residentevilsurvivor.wasm


## Compilation

This auto splitter is written in Rust. In order to compile it, you need to
install the Rust compiler: [Install Rust](https://www.rust-lang.org/tools/install).

Afterwards install the WebAssembly target:
```sh
rustup target add wasm32-unknown-unknown --toolchain stable
```

The auto splitter can now be compiled:
```sh
cargo build --release
```

The auto splitter is then available at:
```
target/wasm32-unknown-unknown/release/countdownvampires.wasm
```

Make sure too look into the [API documentation](https://livesplit.org/asr/asr/) for the `asr` crate.

You can use the [debugger](https://github.com/CryZe/asr-debugger) while
developing the auto splitter to more easily see the log messages, statistics,
dump memory and more.
