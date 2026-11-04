# Etebase Java / Android

Rust-backed Java bindings for Etebase end-to-end encrypted synchronization. This
maintained fork builds the Android AAR and desktop JVM JAR from source; it does not
claim that its version is published to Maven Central. API documentation:
<https://docs.etebase.com>.

## Requirements

- JDK 25 and the included Gradle 9.8.2 wrapper; Java 17 bytecode.
- Rust/rustup; `rust-toolchain.toml` selects the toolchain. Cargo dependencies,
  including the maintained `nikossvnk/etebase-rs` revision, are locked.
- A native C compiler and build tools (`build-essential`, `pkg-config` on Ubuntu).
- For Android: SDK Platform 37.0, Build Tools 37.0.0, NDK 30.0.16248370, and
  `ANDROID_HOME`. Android builds require Linux or macOS host NDK tools.

## Build

```sh
# All four Android ABIs, AARs, JVM tests and Android Lint:
./gradlew :client:build :client:compileDebugAndroidTestJavaWithJavac

# Desktop only; no Android SDK required:
./gradlew -PjvmOnly :client-jvm:build

# Native libraries only:
./build.sh android                  # armeabi-v7a, arm64-v8a, x86, x86_64
./build.sh android arm64-v8a
./build.sh jvm                      # current host
./build.sh jvm aarch64-apple-darwin  # requires matching native toolchain
```

Gradle builds native code automatically. Android libraries target API 23 and use
16 KB ELF segment alignment, verified by `llvm-readelf` before packaging. Generated
bindings and binaries live under each module's `build/generated/`; do not commit
them. A clean Gradle build regenerates Java even when Cargo artifacts remain cached.

The JVM JAR contains the native library built for the current host. CI combines
Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64 libraries into one JAR.
The loader extracts its matching library from the classpath. For an external
library, set `-Dcom.etebase.client.native.path=/absolute/path/to/library`.
On recent JDKs, run with `--enable-native-access=ALL-UNNAMED` to explicitly authorize JNI.

CI can assemble downloaded native artifacts with
`-PetebaseNativePrebuilt=true`; generated Java and matching native resources must
already exist. Normal source builds must not use this flag.

## API changes in 2.4.0

`Collection.markDeleted()` and `Item.markDeleted()` create encrypted sync tombstones.
Upload the modified object through its manager afterward. Their generated
`delete()` method releases a native handle; it does **not** delete synced content.
Migrate callers from the old tombstone `delete()` API before using this version.
Do not access an object after releasing its handle. Account sessions and cache
formats remain compatible with Etebase.

## Tests

JVM JUnit4 tests exercise real JNI loading, exception conversion, Unicode/null
metadata, and 64-bit timestamps. Android service tests create isolated users on a
signup-enabled test server; they no longer depend on a saved private session.
Start a disposable server and run with an emulator/device:

```sh
podman run --rm -p 127.0.0.1:3735:3735 ghcr.io/nikossvnk/etesync-server/test-server:latest
./gradlew :client:connectedDebugAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.etebaseServerUrl=http://10.0.2.2:3735
```

`10.0.2.2` addresses the emulator host. For a USB device, use `adb reverse
tcp:3735 tcp:3735` and pass `http://127.0.0.1:3735`. Cleartext access is enabled
only in the instrumentation APK for this disposable server. Never use production
accounts for tests.

GitHub Actions checks every change and runs weekly. Dependabot checks Cargo,
Gradle and action updates weekly; no updates are ignored.
