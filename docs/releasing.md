# Maintaining releases

The `Build and release` workflow runs on every push and pull request, and can be started manually. It tests and builds on Ubuntu 22.04 x64 and Windows Server 2022 x64 (MSVC). Actions are pinned to commits; Dependabot checks their updates monthly. Rust is pinned in `rust-toolchain.toml` and the workflow. Cargo, npm and NuGet dependencies use committed lock files.

Both jobs run formatting, Clippy, Rust tests, UI tests, a release build, packaging, and a smoke test of the extracted archive. The Windows job also builds and executes the self-contained sensor helper. Smoke tests use a temporary config and `--demo`, which prevents USB discovery and control. They require port 7676 to be unused and refuse to run against an existing service. CI does not validate physical hardware, the installation scripts, driver installation or system startup integration.

## Publishing a version

1. Update `Cargo.toml` and its `Cargo.lock` package version. Add `.github/release-notes/vX.Y.Z.md` with changes, installation steps and current validation limits.
2. Commit and push the changes. Check both platform jobs are green.
3. Create an annotated tag on that commit: `git tag -a vX.Y.Z -m 'W-Connect vX.Y.Z'`, then `git push origin vX.Y.Z`.

The tag must exactly match the package version. The tag workflow rebuilds and tests both platforms, verifies their archive checksums, uploads both archives and `SHA256SUMS` to a draft release, then publishes it. Release write permission exists only in the publishing job. A failed platform build prevents publication. Rerunning a job after it already created a draft requires resolving that existing draft first; published assets are never overwritten automatically.

## Local packaging

After `cargo build --release --locked`, with Python 3.11 or newer:

```sh
python3 scripts/package-release.py --platform linux-x64
python3 scripts/smoke-release.py --platform linux-x64
```

On Windows, publish the helper into `target/release/sensors` as described in the README, then use `python scripts/package-release.py --platform windows-x64` and `python scripts/smoke-release.py --platform windows-x64`. Smoke tests need Python 3.12 or newer. The package command also accepts `--binary` and `--sensors` for cross-built artifacts; a smoke test must run on the target OS.

Archives contain the binary, relevant installers, icons, documentation, project and dependency licenses, and `BUILD-INFO.json` recording the version, source commit, compiler and target platform. Packaging uses an explicit allowlist; local research, runtime configs, camera footage, vendor installers, dependency caches and source credentials are excluded. Local packaging uses the current working files; only the clean tagged CI checkout provides a release tied to a committed source snapshot.

Linux packages target the glibc version in Ubuntu 22.04 (2.35). Windows executables are not Authenticode-signed. Neither download requires a browser runtime bundled with the app; the Windows helper includes its .NET runtime. PawnIO remains a separately installed prerequisite.
