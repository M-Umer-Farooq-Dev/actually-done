"""Save maintainer adoption aggregates locally; never instrument CLI users."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

BASE = "https://api.github.com/repos/M-Umer-Farooq-Dev/actually-done"


def collect(traffic=False):
    headers = {"User-Agent": "actually-done-maintainer-metrics", "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28"}
    if traffic:
        credential = subprocess.run(["git", "-c", "credential.interactive=false", "credential", "fill"], input="protocol=https\nhost=github.com\n\n", text=True, capture_output=True)
        if credential.returncode != 0:
            raise RuntimeError("GitHub credentials unavailable; authenticate locally or omit --traffic")
        fields = dict(line.split("=", 1) for line in credential.stdout.splitlines() if "=" in line)
        if not fields.get("password"):
            raise RuntimeError("GitHub token unavailable")
        headers["Authorization"] = "Bearer " + fields["password"]

    def get(path):
        try:
            with urlopen(Request(BASE + path, headers=headers), timeout=20) as response:
                return json.load(response)
        except HTTPError as error:
            raise RuntimeError(f"GitHub API HTTP {error.code} for {path or 'repository'}") from None
        except URLError:
            raise RuntimeError("GitHub network request failed") from None

    repo = get("")
    releases = []
    page = 1
    while True:
        batch = get(f"/releases?per_page=100&page={page}")
        releases.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    snapshot = {"captured_at": datetime.now(timezone.utc).isoformat(), "repository": repo["html_url"], "stars": repo["stargazers_count"], "forks": repo["forks_count"], "release_assets": [{"tag": release["tag_name"], "name": asset["name"], "download_count": asset["download_count"]} for release in releases if not release["draft"] for asset in release["assets"]], "traffic": None, "limits": ["Asset downloads and clones are not unique users or confirmed CLI use.", "GitHub traffic is a rolling 14-day window; overlapping snapshots must not be summed.", "No runtime telemetry, background scheduler, or transcript access."]}
    if traffic:
        snapshot["traffic"] = {"views": get("/traffic/views"), "clones": get("/traffic/clones"), "referrers": get("/traffic/popular/referrers"), "paths": get("/traffic/popular/paths")}
    return snapshot


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--traffic", action="store_true", help="Use local GitHub credentials to fetch private aggregates")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    try:
        snapshot = collect(args.traffic)
        path = args.out or Path(".metrics") / (datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + ".json")
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("x", encoding="utf-8") as output:
            json.dump(snapshot, output, indent=2)
            output.write("\n")
        print(f"Saved aggregates to {path}; missing traffic is null, not zero.")
    except (RuntimeError, OSError, ValueError) as error:
        parser.exit(2, f"Snapshot failed: {error}\n")
