"""Package an already built native binary with its licenses and SHA-256 digest."""

import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "aarch64-apple-darwin"])
    args = parser.parse_args()
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    folder = ROOT / "dist" / f"actually-done-v{version}-{args.target}"
    folder.mkdir(parents=True, exist_ok=True)
    binary_name = "actually-done.exe" if "windows" in args.target else "actually-done"
    binary = ROOT / "target" / args.target / "release" / binary_name
    actual_version = subprocess.run([str(binary), "--version"], check=True, capture_output=True, text=True).stdout.strip()
    if actual_version != f"actually-done {version}":
        raise RuntimeError(f"Binary version does not match package metadata: {actual_version}")
    shutil.copy2(binary, folder / binary_name)
    for name in ["LICENSE", "README.md"]:
        shutil.copy2(ROOT / name, folder / name)
    shutil.copytree(ROOT / "skills" / "actually-done", folder / "skill", dirs_exist_ok=True)
    shutil.copytree(ROOT / "licenses", folder / "licenses", dirs_exist_ok=True)
    subprocess.run([sys.executable, str(ROOT / "scripts/license_report.py"), "--target", args.target, "--notices-out", str(folder / "THIRD_PARTY_NOTICES.md")], check=True)
    if "windows" in args.target:
        archive = folder.parent / (folder.name + ".zip")
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as output:
            for path in sorted(folder.rglob("*")):
                if path.is_file():
                    output.write(path, path.relative_to(folder.parent))
    else:
        archive = folder.parent / (folder.name + ".tar.gz")
        with tarfile.open(archive, "w:gz") as output:
            output.add(folder, arcname=folder.name)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (folder.parent / (archive.name + ".sha256")).write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
    print(archive)


if __name__ == "__main__":
    main()
