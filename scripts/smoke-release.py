#!/usr/bin/env python3
"""Extract and exercise a release in USB-free demo mode on its native OS."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import socket
import shutil
import subprocess
import tarfile
import tempfile
import time
import tomllib
import urllib.error
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BASE = "http://127.0.0.1:7676"
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def request(path, data=None, headers=None):
    req = urllib.request.Request(
        BASE + path,
        data=json.dumps(data).encode() if data is not None else None,
        headers=headers or {},
    )
    with HTTP.open(req, timeout=3) as response:
        return response.read(), response.headers


def check(condition, message):
    if not condition:
        raise RuntimeError(message)


def smoke(platform):
    windows = platform == "windows-x64"
    check(windows == (os.name == "nt"), "Run the package on its native OS")
    # Refuse to interact with an existing controller service, even in a local run.
    with socket.socket() as port:
        port.bind(("127.0.0.1", 7676))
    with (ROOT / "Cargo.toml").open("rb") as manifest:
        version = tomllib.load(manifest)["package"]["version"]
    name = f"w-connect-v{version}-{platform}"
    archive = ROOT / "dist" / (name + (".zip" if windows else ".tar.gz"))
    checksum = archive.with_name(archive.name + ".sha256").read_bytes()
    check(b"\r" not in checksum, "Checksum must use LF on every platform for Linux release verification")
    expected = checksum.decode("utf-8").split()[0]
    with archive.open("rb") as stream:
        check(hashlib.file_digest(stream, "sha256").hexdigest() == expected, "Archive checksum mismatch")
    with tempfile.TemporaryDirectory(prefix="w-connect-smoke-") as temporary:
        temporary = Path(temporary)
        if windows:
            with zipfile.ZipFile(archive) as bundle:
                bundle.extractall(temporary)
        else:
            with tarfile.open(archive) as bundle:
                bundle.extractall(temporary, filter="data")
        package = temporary / name
        check((package / "LICENSE").is_file(), "Missing project license")
        check(any((package / "licenses").iterdir()), "Missing dependency licenses")
        build_info = json.loads((package / "BUILD-INFO.json").read_text())
        check(build_info["version"] == version, "Package version mismatch")
        binary = package / ("w-connect.exe" if windows else "target/release/w-connect")
        if windows:
            helper = package / "sensors/w-connect-sensors.exe"
            sample = subprocess.run([str(helper)], capture_output=True, text=True, timeout=15, check=True)
            sensors = json.loads(sample.stdout)
            check({"cpu", "gpu", "source"} <= sensors.keys(), "Invalid Windows helper output")
            # The installer places the helper beside the controller executable.
            shutil.copy2(helper, package / helper.name)
            print("Windows sensor helper JSON contract passed (hardware readings may be unavailable)")
        config = temporary / "config/config.json"
        env = dict(os.environ, W_CONNECT_CONFIG=str(config))
        with (temporary / "app.log").open("w+", encoding="utf-8") as log:
            process = subprocess.Popen([str(binary), "--demo"], cwd=package, env=env, stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 20
                while True:
                    check(process.poll() is None, "Application exited during startup")
                    try:
                        state = json.loads(request("/api/state")[0])
                        if len(state["channels"]) == 4 and (not windows or state["sensors"]["source"].startswith("LibreHardwareMonitor")):
                            break
                    except (OSError, urllib.error.URLError):
                        pass
                    check(time.monotonic() < deadline, "Application did not become ready")
                    time.sleep(0.2)
                check(state["demo"] and state["version"] == version, "Wrong running mode or version")
                check(not state["controller"]["connected"], "Demo must not connect to USB")
                for path in ("/", "/app.js", "/style.css", "/favicon.svg", "/fonts/Geist.woff2", "/fonts/GeistMono.woff2"):
                    body, headers = request(path)
                    check(len(body) > 100, f"Missing embedded asset {path}")
                    check("Content-Security-Policy" in headers, f"Missing CSP on {path}")
                catalog = json.loads(request("/api/devices")[0])
                check(catalog["demo"] and not catalog["devices"], "Demo must not discover USB")
                check(len(catalog["models"]) == 7, "Missing controller profiles")
                headers = {"Content-Type": "application/json", "X-W-Connect": "1"}
                for invalid in ({}, dict(headers, Origin="https://example.com")):
                    try:
                        request("/api/emergency", {}, invalid)
                    except urllib.error.HTTPError as error:
                        check(error.code == 403, "Unexpected rejection status")
                    else:
                        raise RuntimeError("Invalid mutation was accepted")
                check(json.loads(request("/api/emergency", {}, headers)[0])["emergency"], "Override failed")
                check(not json.loads(request("/api/resume", {}, headers)[0])["emergency"], "Resume failed")
                draft = state["config"]
                draft["channels"][0]["name"] = "Release smoke test"
                saved = json.loads(request("/api/config", draft, headers)[0])
                check(saved["config"] == draft, "Saved config was not returned")
                check(json.loads(config.read_text()) == draft, "Config was not persisted")
                check(not draft["controller"]["allow_untested"], "Experimental drivers must default off")
                print(f"{platform}: extracted package, assets, discovery, origin protection, override and save passed")
            except Exception:
                log.flush()
                log.seek(0)
                print(log.read())
                raise
            finally:
                if windows and process.poll() is None:
                    subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], check=False, capture_output=True)
                else:
                    process.terminate()
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", required=True, choices=("linux-x64", "windows-x64"))
    smoke(parser.parse_args().platform)
