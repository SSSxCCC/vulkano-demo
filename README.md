# vulkano-demo

## Run desktop client

```
cargo run -p hello-triangle -F desktop
```

## Build android app

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
cargo ndk -t arm64-v8a -o android-project/app/src/main/jniLibs/ build -p hello-triangle
cd android-project
./gradlew build
./gradlew installDebug
```

## Enable rust analyzer hightlight and autocomplete for android source codes

Add the following setting to ".vscode/settings.json":

```
"rust-analyzer.cargo.target": "aarch64-linux-android",
```
