#!/bin/bash
# Build/package only. Does not install tools, fetch LFS, publish, notarize or change OS security.
set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "$(uname -s)" != Darwin || "$(uname -m)" != arm64 ]]; then
  echo 'This helper requires a native Apple Silicon Mac (not Rosetta or Linux).' >&2
  exit 1
fi
if [[ ${WARP_LOCAL_SKIP_METAL+x} ]]; then
  echo 'WARP_LOCAL_SKIP_METAL must be unset; it creates an unusable empty shader library.' >&2
  exit 1
fi
if [[ ${1:-} != '' && ${1:-} != '--offline' ]]; then
  echo 'Usage: script/build-intranet-macos.sh [--offline]' >&2
  exit 1
fi
for tool in cargo rustc xcrun plutil codesign ditto sips iconutil otool; do
  command -v "$tool" >/dev/null || { echo "Missing prerequisite: $tool" >&2; exit 1; }
done
xcrun --find metal >/dev/null
xcrun --find metallib >/dev/null
if [[ -z ${INTRANET_THIRD_PARTY_LICENSES:-} || ! -s "$INTRANET_THIRD_PARTY_LICENSES" ]]; then
  echo "Set INTRANET_THIRD_PARTY_LICENSES to generated Cargo attribution (see docs/intranet/MACOS_ARM64_BUILD.md)." >&2
  exit 1
fi

# Keep this array nonempty: macOS /bin/bash 3.2 rejects empty arrays under set -u.
cargo_args=(+1.92.0)
if [[ -n ${INTRANET_VENDOR_CONFIG:-} ]]; then
  [[ -f "$INTRANET_VENDOR_CONFIG" ]] || { echo 'Vendor config not found' >&2; exit 1; }
  cargo_args+=(--config "$INTRANET_VENDOR_CONFIG")
fi
build_args=(--locked --release -p warp --bin warp-oss --target aarch64-apple-darwin)
if [[ ${1:-} == '--offline' ]]; then
  build_args+=(--frozen)
fi
# Avoid unexpectedly consuming many cores. Callers may opt into a larger approved builder.
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-11.0}
cargo "${cargo_args[@]}" build "${build_args[@]}"

binary="${CARGO_TARGET_DIR:-target}/aarch64-apple-darwin/release/warp-oss"
[[ -x "$binary" ]] || { echo "Missing executable: $binary" >&2; exit 1; }
# A fresh directory prevents stale signed resources from entering a new artifact.
output_root=${INTRANET_OUTPUT_DIR:-dist}
mkdir -p "$output_root"
out=$(mktemp -d "$output_root/intranet-$(git rev-parse --short=12 HEAD)-XXXXXX")
app="$out/Warp Intranet.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/licenses"
cp "$binary" "$app/Contents/MacOS/warp-oss"
cp LICENSE-AGPL LICENSE-MIT "$app/Contents/Resources/licenses/"
cp crates/network_policy/LICENSE "$app/Contents/Resources/licenses/network-policy-LICENSE.txt"
cp "$INTRANET_THIRD_PARTY_LICENSES" "$app/Contents/Resources/licenses/THIRD_PARTY_LICENSES.txt"
for notice in \
  crates/warp_terminal/src/model/LICENSE-ALACRITTY \
  app/assets/bundled/fonts/hack/LICENSE.md \
  app/assets/bundled/fonts/roboto/LICENSE.txt \
  app/assets/bundled/bootstrap/bash-preexec-LICENSE.md \
  app/src/server/telemetry/LICENSE-RUDDER-SDK-RUST.txt \
  app/src/code_review/GITHUB-DESKTOP-LICENSE; do
  { printf "\n--- %s ---\n" "$notice"; cat "$notice"; } >> "$app/Contents/Resources/licenses/THIRD_PARTY_LICENSES.txt"
done

cat > "$app/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Warp Intranet</string>
<key>CFBundleDisplayName</key><string>Warp Intranet</string>
<key>CFBundleIdentifier</key><string>dev.warp.WarpIntranet</string>
<key>CFBundleExecutable</key><string>warp-oss</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>CFBundleIconFile</key><string>intranet.icns</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
<key>NSHighResolutionCapable</key><true/>
<key>UIDesignRequiresCompatibility</key><true/>
<key>NSHumanReadableCopyright</key><string>© 2026 Denver Technologies, Inc. Modified intranet distribution; not an official Warp release.</string>
</dict></plist>
PLIST
plutil -replace LSMinimumSystemVersion -string "$MACOSX_DEPLOYMENT_TARGET" "$app/Contents/Info.plist"
plutil -lint "$app/Contents/Info.plist"

iconset="$out/intranet.iconset"
mkdir -p "$iconset"
icon_source=app/channels/oss/icon/no-padding/512x512.png
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$icon_source" --out "$iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$icon_source" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/intranet.icns"
# This app has no DockTile plug-in or external agent resources. Release assets are embedded.
identity=${INTRANET_CODESIGN_IDENTITY:--}
codesign --force --sign "$identity" "$app"
codesign --verify --deep --strict --verbose=2 "$app"
{
  echo "source=$(git rev-parse HEAD)"
  echo "tree_dirty=$(git status --porcelain | wc -l | tr -d ' ')"
  echo 'target=aarch64-apple-darwin'
  echo "deployment_target=$MACOSX_DEPLOYMENT_TARGET"
  echo "signing_identity=$identity"
  echo "build_jobs=$CARGO_BUILD_JOBS"
  echo "release_opt_level=${CARGO_PROFILE_RELEASE_OPT_LEVEL:-manifest-default}"
  echo "release_debug=${CARGO_PROFILE_RELEASE_DEBUG:-manifest-default}"
  echo "release_lto=${CARGO_PROFILE_RELEASE_LTO:-manifest-default}"
  echo "release_codegen_units=${CARGO_PROFILE_RELEASE_CODEGEN_UNITS:-manifest-default}"
  shasum -a 256 Cargo.lock
  rustc +1.92.0 -Vv
  xcodebuild -version
  otool -L "$app/Contents/MacOS/warp-oss"
} > "$out/BUILD-INFO.txt"
if otool -L "$app/Contents/MacOS/warp-oss" | grep -E '/opt/homebrew/|/usr/local/|/Users/' >/dev/null; then
  echo 'Non-system dylib dependency found; artifact is NOT relocatable. Inspect BUILD-INFO.txt.' >&2
  exit 1
fi
ditto -c -k --sequesterRsrc --keepParent "$app" "$out/Warp-Intranet-macos-arm64.app.zip"
(cd "$out" && shasum -a 256 Warp-Intranet-macos-arm64.app.zip BUILD-INFO.txt > SHA256SUMS)
printf 'Candidate artifact: %s\nNot notarized; native/offline acceptance is still required.\n' "$out"
