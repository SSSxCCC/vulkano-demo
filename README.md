# vulkano-demo

## Run

### Windows

```
cargo run -p minimal -F desktop
```

### Android

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
cargo ndk -t arm64-v8a -o android-project/app/src/main/jniLibs/ build -p minimal
cd android-project
./gradlew build
./gradlew installDebug
```

## Enable rust analyzer hightlight and autocomplete for android source codes

Add the following setting to ".vscode/settings.json":

```
"rust-analyzer.cargo.target": "aarch64-linux-android",
```
