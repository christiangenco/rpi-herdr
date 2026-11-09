#!/usr/bin/env python3
"""Build rpi-extension.json (schema_version 1) from packaged archives.

Usage: make-manifest.py <artifacts-dir> > rpi-extension.json
Each artifact is <target>/<archive> with a sibling <target>/library.sha256.
Environment: GITHUB_REPOSITORY, GITHUB_SHA, TAG, NAME, VERSION, SDK_VERSION, PLUGIN_ABI.
"""
import hashlib, json, os, pathlib, sys

root = pathlib.Path(sys.argv[1])
env = os.environ
repo, tag = env["GITHUB_REPOSITORY"], env["TAG"]
targets = {}
for d in sorted(p for p in root.iterdir() if p.is_dir()):
    target = d.name
    archive = next(d.glob("*.tar.gz"))
    data = archive.read_bytes()
    lib = (d / "library.name").read_text().strip()
    targets[target] = {
        "url": f"https://github.com/{repo}/releases/download/{tag}/{archive.name}",
        "format": "tar.gz",
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "library": {"path": lib, "sha256": (d / "library.sha256").read_text().strip()},
    }
json.dump({
    "schema_version": 1,
    "name": env["NAME"],
    "version": env["VERSION"],
    "repository": f"https://github.com/{repo}",
    "tag": tag,
    "source_commit": env["GITHUB_SHA"],
    "sdk": {"version": env["SDK_VERSION"], "plugin_abi": int(env["PLUGIN_ABI"])},
    "targets": targets,
}, sys.stdout, indent=2)
print()
