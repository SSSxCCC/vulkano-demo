# vulkano-demo

## Run windows client

```
cargo run -p vulkano-windows
```

## Build android app

```
rustup target add aarch64-linux-android
cargo install cargo-ndk
cargo ndk -t arm64-v8a -o vulkano-android/android-project/app/src/main/jniLibs/ build -p vulkano-android
cd vulkano-android/android-project
./gradlew build
./gradlew installDebug
```

## Enable rust analyzer hightlight and autocomplete for android sources

Add the following setting to ".vscode/settings.json":

'''
"rust-analyzer.cargo.target": "aarch64-linux-android",
'''
