# Tauri Packaging Matrix (Task-10 Phase 3)

This document defines the packaging and troubleshooting flow for desktop and mobile targets.

## Command Matrix

| Target | Purpose | Command |
|---|---|---|
| Desktop preflight | Verify host dependencies and toolchain | `make tauri-preflight` |
| Desktop build | Compile Tauri app without native bundling | `make tauri-build-desktop` |
| Android init | Generate Android project (`src-tauri/gen/android`) | `make tauri-android-init` |
| Android debug build | Build APK in debug mode | `make tauri-android-build` |
| Android dev run | Run Android dev target | `make tauri-android-dev` |
| iOS dev run | Run iOS dev target (macOS only) | `make tauri-ios-dev` |

## Linux Desktop Prerequisites

Tauri v2 on Linux requires GTK/WebKit development packages discoverable by `pkg-config`.
Typical Debian/Ubuntu packages:

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev
```

Validated on current host:

- `make tauri-preflight` passes after installing GTK/WebKit deps.
- `make tauri-build-desktop` passes and outputs `src-tauri/target/debug/chaos-bot-app`.

Common failure signatures (if dependencies are missing):

- `The system library 'pango' required by crate 'pango-sys' was not found`
- `The system library 'gdk-3.0' required by crate 'gdk-sys' was not found`
- `PKG_CONFIG_PATH environment variable is not set`

If packages are installed into non-standard locations, export `PKG_CONFIG_PATH` before build.

## Android Prerequisites

`make tauri-android-init` requires Java and Android SDK toolchain:

- `JAVA_HOME` configured or Java available in `PATH` (Debian trixie validated with `openjdk-21-jdk`)
- Android SDK / NDK + platform tools
- Android Studio commandline tooling

Validated on current host:

- `make tauri-android-init` passes with local SDK/NDK and Java 21.
- `make tauri-android-build` passes and outputs:
  - `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`

Common failure signature:

- `Java not found in PATH ... and JAVA_HOME environment variable not set`

Android Gradle bridge note:

- `src-tauri/package.json` contains a `tauri` script proxy to `frontend-react` CLI.
- This is required because generated Android Gradle `rustBuild*` tasks execute `npm run tauri ...` from `src-tauri`.

## iOS Prerequisites

iOS requires macOS + Xcode and must be executed on macOS runners.
Linux hosts should treat iOS commands as non-runnable by design.

## Signing and Release Inputs

Desktop/mobile release signing should use environment-injected secrets in CI:

- Desktop: platform-specific signing credentials (if enabled)
- Android: keystore file, alias, passwords
- iOS: Apple team signing identities/profiles

No secrets should be committed to repository.

## CI Guidance

For CI progression (recommended sequence):

1. Desktop preflight + desktop `--no-bundle` build on Linux runner.
2. Android init/build on runner/image with Android SDK + JDK.
3. iOS build on macOS runner.
