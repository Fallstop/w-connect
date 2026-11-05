#!/usr/bin/env python3
"""Build release archives from an explicit allowlist; Python 3.11+, no extra packages."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def version():
    with (ROOT / "Cargo.toml").open("rb") as manifest:
        value = tomllib.load(manifest)["package"]["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", value):
        raise ValueError("Release version must be major.minor.patch")
    if os.environ.get("GITHUB_REF_TYPE") == "tag":
        if os.environ.get("GITHUB_REF_NAME") != f"v{value}":
            raise ValueError("Git tag must exactly match the Cargo.toml version")
    notes = ROOT / ".github" / "release-notes" / f"v{value}.md"
    if not notes.is_file() or not notes.read_text(encoding="utf-8").strip():
        raise ValueError(f"Missing release notes: {notes.name}")
    return value


def copy_file(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def collect_licenses(destination, target):
    metadata = json.loads(command("cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", target))
    for package in metadata["packages"]:
        if package["source"] is None:
            continue
        source = Path(package["manifest_path"]).parent
        files = {
            path for path in source.rglob("*")
            if path.is_file() and path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))
        }
        if package["license_file"]:
            files.add(source / package["license_file"])
        if not files:
            raise ValueError(f"No license text found for {package['name']}")
        package_dir = destination / f"{package['name']}-{package['version']}"
        for path in sorted(files):
            copy_file(path, package_dir / path.relative_to(source))


def package(args):
    release_version = version()
    name = f"w-connect-v{release_version}-{args.platform}"
    output = ROOT / "dist"
    output.mkdir(exist_ok=True)
    windows = args.platform == "windows-x64"
    target = "x86_64-pc-windows-msvc" if windows else "x86_64-unknown-linux-gnu"
    binary = Path(args.binary) if args.binary else ROOT / "target/release" / ("w-connect.exe" if windows else "w-connect")
    with tempfile.TemporaryDirectory(prefix="w-connect-package-") as temporary:
        staging = Path(temporary) / name
        staging.mkdir()
        for filename in ("README.md", "LICENSE"):
            copy_file(ROOT / filename, staging / filename)
        for directory in ("docs", "assets"):
            shutil.copytree(ROOT / directory, staging / directory)
        copy_file(ROOT / "windows/README.md", staging / "windows/README.md")
        if windows:
            copy_file(binary, staging / "w-connect.exe")
            sensors = Path(args.sensors) if args.sensors else ROOT / "target/release/sensors"
            for filename in (
                "w-connect-sensors.exe", "LibreHardwareMonitor-LICENSE.txt",
                "LibreHardwareMonitor-THIRD-PARTY-NOTICES.txt", "DotNet-LICENSE.txt",
                "DotNet-THIRD-PARTY-NOTICES.txt",
            ):
                copy_file(sensors / filename, staging / "sensors" / filename)
            copy_file(ROOT / "scripts/install-windows.ps1", staging / "scripts/install-windows.ps1")
        else:
            copy_file(binary, staging / "target/release/w-connect")
            (staging / "target/release/w-connect").chmod(0o755)
            for filename in ("install-linux.sh", "install-usb-access.sh"):
                copy_file(ROOT / "scripts" / filename, staging / "scripts" / filename)
        collect_licenses(staging / "licenses", target)
        info = {
            "version": release_version,
            "commit": command("git", "rev-parse", "HEAD"),
            "platform": args.platform,
            "compiler": command("rustc", "--version"),
        }
        (staging / "BUILD-INFO.json").write_text(json.dumps(info, indent=2) + "\n", encoding="utf-8")
        if windows:
            archive = output / f"{name}.zip"
            with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9, strict_timestamps=False) as bundle:
                for path in sorted(staging.rglob("*")):
                    if path.is_file():
                        bundle.write(path, path.relative_to(staging.parent))
        else:
            archive = output / f"{name}.tar.gz"
            with tarfile.open(archive, "w:gz", compresslevel=9) as bundle:
                bundle.add(staging, arcname=name)
    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    archive.with_name(archive.name + ".sha256").write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
    print(f"Packaged {archive.name} ({archive.stat().st_size:,} bytes)\nSHA-256: {digest}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Validate version/tag and release notes only")
    parser.add_argument("--platform", choices=("linux-x64", "windows-x64"))
    parser.add_argument("--binary", help="Override the already-built native application path")
    parser.add_argument("--sensors", help="Override the published Windows helper directory")
    args = parser.parse_args()
    if args.check:
        print(f"Release metadata verified: v{version()}")
    elif not args.platform:
        parser.error("--platform is required unless --check is used")
    else:
        package(args)
