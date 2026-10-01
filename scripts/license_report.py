"""Generate reproducible dependency inventory and Windows release notices.

Development-only, Python standard library. No transcript or credential access.
"""

import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
TARGET = "x86_64-pc-windows-msvc"
ALLOWED = {
    "MIT", "MIT OR Apache-2.0", "Apache-2.0 OR MIT", "MIT/Apache-2.0",
    "Unlicense OR MIT", "Unlicense/MIT", "Zlib OR Apache-2.0 OR MIT",
    "MIT OR Apache-2.0 OR LGPL-2.1-or-later",
    "Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT",
    "(MIT OR Apache-2.0) AND Unicode-3.0",
}


def generate() -> dict[Path, str]:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=True,
    )
    data = json.loads(result.stdout)
    windows_result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1",
         "--filter-platform", TARGET],
        cwd=ROOT, capture_output=True, text=True, encoding="utf-8", check=True,
    )
    packages = sorted(
        (p for p in data["packages"] if p["source"] is not None),
        key=lambda p: (p["name"], p["version"]),
    )
    active = {node["id"] for node in json.loads(windows_result.stdout)["resolve"]["nodes"]}
    inventory = [
        "# Dependency license inventory", "",
        "Generated from Cargo.lock by `python scripts/license_report.py`.", "",
        "This lists all locked third-party packages, including target-specific dependencies.",
        "MIT is the selected option for each OR expression; Unicode-3.0 additionally applies to unicode-ident.",
        "This inventory is not a transfer of copyright or an exhaustive legal opinion.", "",
        "| Package | Declared license | Selected option | Windows graph |",
        "| --- | --- | --- | --- |",
    ]
    notices = [
        "# Third-party notices", "",
        f"Generated for the locked `{TARGET}` dependency graph.",
        "Include this file and LICENSE with the Windows binary.",
        "These notices retain upstream license and attribution text; other targets need their own notice bundle.",
        "MIT is selected where offered as an alternative. Unicode-3.0 is also retained where required.", "",
    ]
    for package in packages:
        license_expression = package.get("license")
        if license_expression not in ALLOWED:
            raise SystemExit(f"Review required: {package['name']}: {license_expression}")
        selected = "MIT AND Unicode-3.0" if "Unicode" in license_expression else "MIT"
        name, version = package["name"], package["version"]
        inventory.append(
            f"| [{name} {version}](https://crates.io/crates/{name}/{version}) "
            f"| {license_expression} | {selected} | {'yes' if package['id'] in active else 'no'} |"
        )
        if package["id"] not in active:
            continue
        directory = Path(package["manifest_path"]).parent
        candidates = sorted(
            p for p in directory.iterdir() if p.is_file()
            and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))
        )
        if not candidates:
            raise SystemExit(f"Missing release notices for {name} {version}")
        notices += [f"## {name} {version}", "", f"Declared: `{license_expression}`. Selected: `{selected}`.", ""]
        for path in candidates:
            notices += [f"### {path.name}", "", "```text", path.read_text(encoding="utf-8").strip(), "```", ""]
    inventory += ["", f"Inventory: {len(packages)} locked third-party packages.", "",
                  "Target-specific crates are not vendored by this project. Before adding binaries for another target,",
                  "review that target's compiled dependencies and upstream licensing qualifications, including Apple SDK requirements.", ""]
    return {
        ROOT / "docs" / "dependency-licenses.md": "\n".join(inventory),
        ROOT / "THIRD_PARTY_NOTICES.md": "\n".join(notices),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Fail if checked-in output differs")
    options = parser.parse_args()
    for path, text in generate().items():
        if options.check:
            if not path.exists() or path.read_text(encoding="utf-8") != text:
                raise SystemExit(f"Regenerate {path.name} with python scripts/license_report.py")
        else:
            path.write_text(text, encoding="utf-8", newline="\n")
    print("Dependency licenses and Windows notices checked." if options.check else "Dependency inventory and Windows notices generated.")


if __name__ == "__main__":
    main()
