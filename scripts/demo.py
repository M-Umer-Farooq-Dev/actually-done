"""Run a synthetic passing-test/PARTIAL demo without accessing real sessions."""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile


def run(binary, record=None):
    with tempfile.TemporaryDirectory(prefix="actually-done-demo-") as directory:
        repo = Path(directory)
        subprocess.run(["git", "init", "-b", "main", str(repo)], check=True, capture_output=True)
        (repo / "auth.py").write_text("def allowed():\n    return True\n", encoding="utf-8")
        (repo / "test_auth.py").write_text("from auth import allowed\nassert allowed() is True\nprint('1 synthetic assertion passed')\n", encoding="utf-8")
        records = [
            {"type": "user", "cwd": str(repo), "message": {"role": "user", "content": "Update auth.py."}},
            {"type": "assistant", "cwd": str(repo), "message": {"role": "assistant", "content": [{"type": "text", "text": "Done. Updated `auth.py`. We should rotate the key later."}]}},
        ]
        transcript = repo / "session.jsonl"
        transcript.write_text("".join(json.dumps(record) + "\n" for record in records), encoding="utf-8")
        test = f'"{Path(sys.executable).as_posix()}" "{(repo / "test_auth.py").as_posix()}"'
        result = subprocess.run([binary, str(repo), "--provider", "claude", "--session", str(transcript), "--test", test, "--json"], capture_output=True, text=True)
        if result.returncode != 1:
            raise RuntimeError(f"Demo expected exit 1, got {result.returncode}: {result.stderr}")
        receipt = json.loads(result.stdout)
        assert receipt["verdict"] == "PARTIAL" and receipt["tests"]["status"] == "passed"
        assert "auth.py" in receipt["git"]["untracked"]
        assert any("rotate" in finding["text"] for finding in receipt["leftovers"])
        summary = {"provider": receipt["provider"], "verdict": receipt["verdict"], "exit_code": receipt["exit_code"], "tests": receipt["tests"]["status"], "untracked_claim": "auth.py", "leftovers": [finding["text"] for finding in receipt["leftovers"]]}
        print(json.dumps(summary, indent=2))
        if record:
            record.parent.mkdir(parents=True, exist_ok=True)
            record.write_text(json.dumps({"caption": "Captured stdout from the synthetic demo; JSON receipt field summary, not an animated recording.", "summary": summary}, indent=2) + "\n", encoding="utf-8")
        return summary


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", default="actually-done")
    parser.add_argument("--record", type=Path, help="Save the verified field summary for sharing")
    options = parser.parse_args()
    run(options.binary, options.record)
