# Changelog

## Version 3.0.0
Breaking: the bindings are rewritten with Mozilla UniFFI 0.32 and are now Kotlin over JNA. Migrate all callers.
* Native memory is released by a `Cleaner` (Android: `SystemCleaner` on API 34+, JNA's cleaner below) instead of `finalize()`; objects are `AutoCloseable`. The manual `delete()` handle release is replaced by `close()`.
* Native library renamed to `libetebase_uniffi`; the JVM JAR stores it as a JNA resource (`<os>-<arch>/`). JNA 5.19.1 is a new runtime dependency (`net.java.dev.jna:jna:5.19.1@aar` on Android). The `NativeLoader`s and `com.etebase.client.native.path` are gone; use `uniffi.component.etebase_uniffi.libraryOverride`.
* Errors: `EtebaseException` is a sealed class with one subclass per Etebase error (`EtebaseException.Unauthorized`, `.Conflict`, `.Connection`, …, and `.ProgrammingException`, `.TemporaryServerException`, `.ServerException`); `com.etebase.client.exceptions.*` is removed. Rust panics surface as `InternalException` instead of aborting.
* There is no default server: `Client.getDefaultServerUrl()` is removed and the server URL of `Client(serverUrl, httpClient)`/`createClient` is required (non-null).
* `Client.create(okHttpClient, url)` becomes `createClient(okHttpClient, url)` (`OkHttpClientBridge`); custom transports implement the `HttpClient` interface returning `HttpResponse` and throw on failure, replacing `HttpClientBridge` and `Response`.
* Accessors drop `get`: `uid()`, `etag()`, `meta()`, `content()`, `stoken()`, `accessLevel()`, `collectionType()`, `asItem()`, `collectionManager()`, `invitationManager()`, `itemManager(col)`, `memberManager(col)`, `senderUsername()`/`senderPubkey()` (was `getFromUsername`/`getFromPubkey`); `Account.isEtebaseServer()` becomes `isEtebaseServer(client)` (returns a `Boolean`); `Utils.*` become top-level functions.
* `ItemMetadata`, `FetchOptions`, `CollectionMember`, `RemovedCollection`, `UserProfile` and all list responses are immutable data classes (`FetchOptions(stoken = s, limit = 50u)`); list responses expose `data`, `stoken`/`iterator`, `done` and `removedMemberships` as `List`s.
* Arrays become `List`s; `CollectionManager.list(String[])` is `listMulti(List<String>)`; `FileSystemCache._unstable_collectionList/_unstable_itemList` are `collectionList`/`itemList`. Optional trailing arguments (`fetchOptions`, `deps`, `encryptionKey`) default to `null`, replacing the Java overloads. The `String` content helpers are removed; use `toByteArray()`/`String(bytes)`.
* Objects are thread-safe. Passing the same `Item` several times to `batch`, `transaction` or `fetchUpdates` (also as a dependency) is supported, and saved etags update the Kotlin object.
* Account sessions and `FileSystemCache` data written by 2.x are read unchanged.
* Build: generated Kotlin comes from the built library through the workspace's `uniffi-bindgen` (Gradle task `generateUniffiBindings`); flapigen, its typemaps and AST post-processing are removed. Narrow R8 consumer rules for JNA and the bindings. CI runs Rust lints and live-server JVM tests.

## Version 2.4.0
* Build against maintained etebase-rs and upstream flapigen, including the repeated-chunk fix.
* Upgrade Gradle, Android tooling, Rust and Java dependencies; require Android API 23 or newer.
* Generate bindings and native libraries during Gradle builds, with locked Cargo dependencies and verified 16 KB Android ELF alignment.
* Package host-native desktop libraries in the JVM JAR and validate JNI Unicode, nullable metadata, timestamp and exception boundaries.
* Rename Collection/Item tombstone operations to markDeleted(); generated delete() releases native handles. Migrate all callers before upgrading.
* Replace private-session Android test fixtures with isolated users on a configurable disposable server; add weekly dependency/build checks.

## Version 2.3.2
* Fix crash for Android devices older than Android 7 due to flapigen issue

## Version 2.3.1
* Fix issue with custom urls lacking a trailing slash
* Update etebase dependency

## Version 2.3.0
* Fix crashes on Lineage for MicroG which was having issue with OptionalLong.
* Remove usages of Optional throughout the lib (was never exposed in the API).
* Remove library desugaring (we can do it now that we no longer use Optional).

## Version 2.2.0
* Collection manager list: support passing arrays of collection types

## Version 2.1.0
* Login: automatically init the account if not init
* Have global and immutable collection types (changes the create and list APIs)
* Update etebase dependency

## Version 2.0.0
* Just a version bump for packaging

## Version 0.2.2
* Expose FileSystemCache class for caching etebase object on the FS
* Update etebase dependency

## Version 0.2.1
* Expose fromUsername for invitations
* Update etebase dependency

## Version 0.2.0
* Use new and shorter fingerprint pretty format
* Update etebase dependency

## Version 0.1.4
* Expose access level values as an enum
* Update etebase dependency

## Version 0.1.3
* Update etebase dependency
* HttpException: don't inherit from ConnectionException.
* Item revisions: fix naming of item revisions response functions.
* Add an API function to check if it's an etebase server.
* Remove usage of optional long from our API
* Change markDeleted() to delete()
* Rename getDefaultApiUrl to getDefaultServerUrl.

## Version 0.1.2
* Update etebase dependency
* Clean up exception types

## Version 0.1.1
* Fix getting item revisions
* Add convenience functions around member, invitation and revision listing

## Version 0.1.0
* Initial release
