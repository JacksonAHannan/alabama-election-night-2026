#!/usr/bin/env python3
"""Publish only public Madison source documents; credentials stay in gh's store."""
import fcntl
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REMOTE = "https://github.com/JacksonAHannan/alabama-election-night-2026.git"
BRANCH = "observations"


def command(*args, cwd, check=True):
    return subprocess.run(
        [str(a) for a in args], cwd=cwd, check=check, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120,
        env={**os.environ, "GIT_TERMINAL_PROMPT": "0", "GH_PROMPT_DISABLED": "1"},
    ).stdout.strip()


def publish(root=ROOT, remote=REMOTE, dispatch=True):
    runtime = root / "runtime"
    runtime.mkdir(exist_ok=True)
    # Separate repository: never checkout, stage or commit the working project.
    with (runtime / "relay-publisher.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        repo = runtime / "relay-publisher"
        repo.mkdir(exist_ok=True)
        if not (repo / ".git").exists():
            command("git", "init", "--initial-branch", BRANCH, cwd=repo)
            command("git", "remote", "add", "origin", remote, cwd=repo)
        if command("git", "remote", "get-url", "origin", cwd=repo) != remote:
            raise RuntimeError("Relay repository has an unexpected remote")
        command("git", "config", "user.name", "Election results relay", cwd=repo)
        command("git", "config", "user.email", "192483868+JacksonAHannan@users.noreply.github.com", cwd=repo)
        if command("git", "ls-remote", "--heads", "origin", BRANCH, cwd=repo):
            command("git", "fetch", "--depth=1", "origin", BRANCH, cwd=repo)
            head = subprocess.run(["git", "rev-parse", "--verify", "HEAD"], cwd=repo, capture_output=True).returncode
            if head:
                command("git", "checkout", "-B", BRANCH, "FETCH_HEAD", cwd=repo)
            else:
                command("git", "merge", "--ff-only", "FETCH_HEAD", cwd=repo)
        command(root / "target/debug/alabama-election-night", "export-relay", "--root", root,
                "--output", repo / "madison.json", cwd=root)
        command("git", "add", "--", "madison.json", cwd=repo)
        if command("git", "diff", "--cached", "--name-only", cwd=repo):
            command("git", "commit", "-m", "Update accepted Madison source observation", cwd=repo)
        command("git", "push", "origin", f"HEAD:refs/heads/{BRANCH}", cwd=repo)
        revision = command("git", "rev-parse", "HEAD", cwd=repo)
        marker = runtime / "relay-dispatched-commit"
        # Retry a failed dispatch even when the observation itself has not changed.
        if dispatch and (not marker.exists() or marker.read_text().strip() != revision):
            command("gh", "workflow", "run", "pages.yml", "--repo",
                    "JacksonAHannan/alabama-election-night-2026", "--ref", "main", cwd=root)
            marker.write_text(revision + "\n")
        print(f"Public Madison relay uploaded: {revision}", flush=True)


if __name__ == "__main__":
    try:
        publish()
    except subprocess.CalledProcessError as error:
        raise SystemExit(f"Relay command failed: {error.stderr.strip()}") from error
