#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
usage() {
    echo 'Usage: build.sh android [armeabi-v7a|arm64-v8a|x86|x86_64] | jvm [rust-target]' >&2
    exit 2
}
mode=${1:-android}

if [[ "$mode" == jvm ]]; then
    target=${2:-$(rustc -vV | grep '^host:' | cut -d' ' -f2)}
    rustup target add "$target"
    cargo build --locked --release --target "$target" -p etebase-uniffi
    case "$target" in
        x86_64-unknown-linux-gnu) prefix=linux-x86-64; library=libetebase_uniffi.so ;;
        aarch64-unknown-linux-gnu) prefix=linux-aarch64; library=libetebase_uniffi.so ;;
        x86_64-apple-darwin) prefix=darwin-x86-64; library=libetebase_uniffi.dylib ;;
        aarch64-apple-darwin) prefix=darwin-aarch64; library=libetebase_uniffi.dylib ;;
        x86_64-pc-windows-msvc) prefix=win32-x86-64; library=etebase_uniffi.dll ;;
        *) echo "Unsupported desktop target: $target" >&2; exit 2 ;;
    esac
    # JNA loads native libraries from <os>-<arch>/ classpath resources.
    destination="client-jvm/build/generated/resources/$prefix"
    mkdir -p "$destination"
    cp "target/$target/release/$library" "$destination/$library"
    exit
fi
[[ "$mode" == android ]] || usage
sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}
: "${sdk:?Set ANDROID_HOME to the Android SDK directory}"
ndk="$sdk/ndk/30.0.16248370"
case "$(uname -s)" in
    Linux) host=linux-x86_64 ;;
    Darwin) host=darwin-x86_64 ;;
    *) echo 'Build Android native libraries on Linux or macOS.' >&2; exit 1 ;;
esac
tools="$ndk/toolchains/llvm/prebuilt/$host/bin"
[[ -x "$tools/clang" ]] || { echo 'Install ndk;30.0.16248370 with sdkmanager' >&2; exit 1; }
selected=${2:-all}
case "$selected" in all|armeabi-v7a|arm64-v8a|x86|x86_64) ;; *) echo "Unknown Android ABI: $selected" >&2; exit 2 ;; esac
for spec in 'armeabi-v7a armv7-linux-androideabi armv7a-linux-androideabi' 'arm64-v8a aarch64-linux-android aarch64-linux-android' 'x86 i686-linux-android i686-linux-android' 'x86_64 x86_64-linux-android x86_64-linux-android'; do
    read -r abi target compiler <<< "$spec"
    [[ "$selected" == all || "$selected" == "$abi" ]] || continue
    rustup target add "$target"
    target_env=${target//-/_}
    target_upper=$(printf '%s' "$target_env" | tr '[:lower:]' '[:upper:]')
    env "CC_$target_env=$tools/${compiler}23-clang" \
        "AR_$target_env=$tools/llvm-ar" \
        "CARGO_TARGET_${target_upper}_LINKER=$tools/${compiler}23-clang" \
        "CARGO_TARGET_${target_upper}_RUSTFLAGS=-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384" \
        cargo build --locked --release --target "$target" -p etebase-uniffi
    library="target/$target/release/libetebase_uniffi.so"
    # Reject native binaries that cannot load on Android devices with 16 KB pages.
    "$tools/llvm-readelf" -lW "$library" | awk '
        $1 == "LOAD" { found = 1; if ($NF != "0x4000" && $NF != "0x10000") bad = 1 }
        END { exit (!found || bad) }'
    mkdir -p "client/build/generated/jniLibs/$abi"
    cp "$library" "client/build/generated/jniLibs/$abi/libetebase_uniffi.so"
done
