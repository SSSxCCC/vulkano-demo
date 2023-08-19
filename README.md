# vulkano-demo

## Run windows client

```
cargo run -p vulkano-windows
```

## Build android library

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
cargo ndk -t arm64-v8a build -p vulkano-android
```
