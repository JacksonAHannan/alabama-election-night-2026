"""Exercise relay retries/idempotence against an isolated local Git remote."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("publisher", Path(__file__).resolve().parents[1] / "deployment/publish-relay.py")
publisher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publisher)


class RelayPublisher(unittest.TestCase):
    def test_only_public_snapshot_pushed_and_failed_dispatch_retried(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder)
            root = base / "project"
            (root / "target/debug").mkdir(parents=True)
            (root / "sentinel").write_text("Unrelated working-tree content")
            binary = root / "target/debug/alabama-election-night"
            binary.write_text("#!/usr/bin/env python3\nimport pathlib,sys\npathlib.Path(sys.argv[sys.argv.index('--output')+1]).write_text('{\"public\":true}')\npathlib.Path(sys.argv[sys.argv.index('--county-output')+1]).write_text('{\"public_counties\":true}')\n")
            binary.chmod(0o755)
            remote = base / "remote.git"
            subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
            real_command = publisher.command
            dispatches = []

            def command(*args, **kwargs):
                if args[0] == "gh":
                    dispatches.append(args)
                    if len(dispatches) == 1:
                        raise subprocess.CalledProcessError(1, args, stderr="Temporary dispatch failure")
                    return ""
                return real_command(*args, **kwargs)

            with patch.object(publisher, "command", side_effect=command):
                with self.assertRaises(subprocess.CalledProcessError):
                    publisher.publish(root, str(remote))
                self.assertFalse((root / "runtime/relay-dispatched-commit").exists())
                publisher.publish(root, str(remote))
                first = (root / "runtime/relay-dispatched-commit").read_text()
                publisher.publish(root, str(remote))
            self.assertEqual(len(dispatches), 2)
            self.assertEqual((root / "runtime/relay-dispatched-commit").read_text(), first)
            files = real_command("git", "ls-tree", "--name-only", "observations", cwd=remote)
            self.assertEqual(files.splitlines(), ["county-feeds.json", "madison.json"])
            self.assertEqual(real_command("git", "rev-list", "--count", "observations", cwd=remote), "1")
            self.assertEqual((root / "sentinel").read_text(), "Unrelated working-tree content")
            self.assertFalse((root / ".git").exists())
            with self.assertRaisesRegex(RuntimeError, "unexpected remote"):
                publisher.publish(root, str(base / "wrong.git"), dispatch=False)


if __name__ == "__main__":
    unittest.main()
