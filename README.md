# Etebase Kotlin / Android

Rust-backed Kotlin bindings for Etebase end-to-end encrypted synchronization, generated
with [UniFFI](https://mozilla.github.io/uniffi-rs/) and loaded through
[JNA](https://github.com/java-native-access/jna). This maintained fork builds the Android
AAR (`client`) and desktop JVM JAR (`client-jvm`) from source; it does not claim that its
version is published to Maven Central. API documentation: <https://docs.etebase.com>.

## Usage

```kotlin
import com.etebase.client.*
import okhttp3.OkHttpClient

val client = createClient(OkHttpClient(), "https://etebase.example.com/")
val account = Account.login(client, "username", "password")
val saved = account.save(encryptionKey)          // restore with Account.restore(client, saved, encryptionKey)

val collections = account.collectionManager()
val calendar = collections.create("etebase.vevent", ItemMetadata(name = "Calendar"), ByteArray(0))
collections.upload(calendar)

val items = collections.itemManager(calendar)
val event = items.create(ItemMetadata(name = "Meeting", mtime = System.currentTimeMillis()), ics.toByteArray())
items.batch(listOf(event))
val changes = collections.list("etebase.vevent", FetchOptions(stoken = savedStoken))
try {
    items.transaction(listOf(event))
} catch (e: EtebaseException.Conflict) {
    // changed on the server meanwhile: fetch and merge
}
```

- Errors are `EtebaseException` subclasses (`Unauthorized`, `Conflict`, `Connection`, …),
  each carrying the message from Rust. Rust panics surface as `InternalException`.
- `Collection.markDeleted()` / `Item.markDeleted()` create encrypted sync tombstones; upload
  the object afterwards.
- Native memory is released by a `Cleaner` when an object becomes unreachable (on Android
  `SystemCleaner` from API 34, JNA's cleaner below). Objects are `AutoCloseable`; `close()`
  releases them immediately. Do not use an object after closing it.
- Objects may be used from any thread. Calls on the same object run one after another,
  including network calls, so do not share one `Item` between a long upload and the UI.
- `createClient` uses the included `OkHttpClientBridge`. Any other transport can implement
  the `HttpClient` interface; exceptions it throws become `EtebaseException.Connection`.
- Account sessions (`Account.save`) and `FileSystemCache` directories written by 2.x remain
  readable.

## Requirements

- JDK 25 and the included Gradle 9.8.2 wrapper; Java 17 bytecode, Kotlin 2.4.21.
- Rust/rustup; `rust-toolchain.toml` selects the toolchain. Cargo dependencies,
  including the maintained `nikossvnk/etebase-rs` revision and UniFFI 0.32.2, are locked.
- A native C compiler and build tools (`build-essential`, `pkg-config` on Ubuntu).
- For Android: SDK Platform 37.0, Build Tools 37.0.0, NDK 30.0.16248370, and
  `ANDROID_HOME`. Android builds require Linux or macOS host NDK tools.

## Build

```sh
# All four Android ABIs, AAR, Android Lint and instrumentation-test compilation:
./gradlew :client:build :client:compileDebugAndroidTestKotlin

# Desktop only; no Android SDK required:
./gradlew -PjvmOnly :client-jvm:build

# Native libraries only:
./build.sh android                  # armeabi-v7a, arm64-v8a, x86, x86_64
./build.sh android arm64-v8a
./build.sh jvm                      # current host
./build.sh jvm aarch64-apple-darwin  # requires matching native toolchain
```

Gradle builds the native library (`libetebase_uniffi`) automatically, then the
`generateUniffiBindings` task runs the workspace's `uniffi-bindgen` on that library to
generate the Kotlin sources (`android = true` for `client` via `uniffi-android.toml`, the
desktop defaults in `uniffi.toml` for `client-jvm`). Generated bindings and binaries live
under each module's `build/generated/`; do not commit them. Android libraries target API 23
and use 16 KB ELF segment alignment, verified by `llvm-readelf` before packaging.

The JVM JAR contains the native library for the current host as a JNA resource
(`<os>-<arch>/`, e.g. `linux-x86-64/libetebase_uniffi.so`). CI combines Linux x86-64/aarch64,
macOS x86-64/aarch64 and Windows x86-64 libraries into one JAR. To load another library, set
`-Duniffi.component.etebase_uniffi.libraryOverride=/absolute/path/to/library`. On recent JDKs,
run with `--enable-native-access=ALL-UNNAMED` to authorize JNA's native access explicitly.

CI assembles downloaded native artifacts with `-PetebaseNativePrebuilt=true`; the bindings
are still generated from those libraries, which requires Rust. Normal source builds must not
use this flag.

R8: `client/consumer-rules.pro` keeps only what JNA and the generated bindings reach by
reflection or from native code; apps need no further rules.

## Tests

JVM JUnit4 tests (`client-jvm/src/test/kotlin`) run against the real native library: error
mapping, `HttpClient` callbacks (including throwing ones), record round trips, and concurrent
or repeated items in batches. With a disposable signup-enabled server they also run
end-to-end tests through OkHttp:

```sh
podman run --rm -p 127.0.0.1:3735:3735 ghcr.io/nikossvnk/etesync-server/test-server:latest
./gradlew -PjvmOnly :client-jvm:test -PetebaseServerUrl=http://127.0.0.1:3735
```

Android service tests create isolated users on such a server; run them with an
emulator/device:

```sh
./gradlew :client:connectedDebugAndroidTest \
  -Pandroid.testInstrumentationRunnerArguments.etebaseServerUrl=http://10.0.2.2:3735
```

`10.0.2.2` addresses the emulator host. For a USB device, use `adb reverse
tcp:3735 tcp:3735` and pass `http://127.0.0.1:3735`. Cleartext access is enabled
only in the instrumentation APK for this disposable server. Never use production
accounts for tests.

GitHub Actions checks every change and runs weekly. Dependabot checks Cargo,
Gradle and action updates weekly; no updates are ignored.
