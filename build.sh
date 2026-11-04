#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
mode=${1:-android}
if [[ "$mode" == jvm ]]; then
    target=${2:-$(rustc -vV | grep '^host:' | cut -d' ' -f2)}
    rustup target add "$target"
    cargo build --locked --release --target "$target" --no-default-features --features jvm
    case "$target" in
        *-linux-gnu) platform=linux; library=libetebase_jni.so ;;
        *-apple-darwin) platform=macos; library=libetebase_jni.dylib ;;
        *-windows-msvc) platform=windows; library=etebase_jni.dll ;;
        *) echo "Unsupported desktop target: $target" >&2; exit 2 ;;
    esac
    arch=${target%%-*}
    destination="client-jvm/build/generated/resources/com/etebase/client/native/$platform-$arch"
    mkdir -p "$destination"
    cp "target/$target/release/$library" "$destination/$library"
    exit
fi
if [[ "$mode" != android ]]; then
    echo 'Usage: build.sh android [armeabi-v7a|arm64-v8a|x86|x86_64] | jvm [rust-target]' >&2
    exit 2
fi
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
        cargo build --locked --release --target "$target" --no-default-features --features android
    library="target/$target/release/libetebase_jni.so"
    # Reject native binaries that cannot load on Android devices with 16 KB pages.
    "$tools/llvm-readelf" -lW "$library" | awk '
        $1 == "LOAD" { found = 1; if ($NF != "0x4000" && $NF != "0x10000") bad = 1 }
        END { exit (!found || bad) }'
    mkdir -p "client/build/generated/jniLibs/$abi"
    cp "$library" "client/build/generated/jniLibs/$abi/libetebase_jni.so"
done
