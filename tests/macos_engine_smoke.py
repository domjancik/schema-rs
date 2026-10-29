"""Native CI: output-disabled HTTP serving, graceful exit and undo recovery."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

binary = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory() as folder:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    base = "http://127.0.0.1:" + str(port)
    env = dict(os.environ)
    env.pop("SCHEMA_TOKEN", None)

    def state():
        with urllib.request.urlopen(base + "/api/state", timeout=2) as response:
            return json.load(response)

    def command(value):
        current = state()
        request = urllib.request.Request(base + "/api/command", method="POST",
            data=json.dumps({"expected_revision": current["snapshot"]["revision"], "command": value}).encode(),
            headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=2) as response:
            assert response.status == 200

    original = None
    for cycle in range(2):
        with tempfile.TemporaryFile() as log:
            child = subprocess.Popen([str(binary), "desktop", "--no-open", "--project", folder,
                                      "--bind", "127.0.0.1:" + str(port)], env=env, stdout=log, stderr=log)
            try:
                for attempt in range(100):
                    if child.poll() is not None:
                        raise RuntimeError("Engine exited before serving")
                    try:
                        current = state()
                        break
                    except OSError:
                        time.sleep(0.1)
                else:
                    raise RuntimeError("Engine did not serve UI")
                assert current["project"]["output"]["enabled"] is False
                with urllib.request.urlopen(base + "/", timeout=2) as response:
                    assert response.status == 200
                if cycle == 0:
                    original = current["project"]["fixtures"][0]["dimmer"]
                    fixture = current["project"]["fixtures"][0]["id"]
                    command({"type": "set_dimmer", "fixture": fixture, "value": 0.37})
                else:
                    assert current["project"]["fixtures"][0]["dimmer"] == 0.37
                    assert current["snapshot"]["can_undo"] is True
                    command({"type": "undo"})
                    assert state()["project"]["fixtures"][0]["dimmer"] == original
                child.send_signal(signal.SIGINT)
                assert child.wait(timeout=15) == 0
                checkpoint = json.loads((Path(folder) / ".session/session.json").read_text())
                assert checkpoint["project"]["output"]["enabled"] is False
                assert len(checkpoint["undo"] if cycle == 0 else checkpoint["redo"]) == 1
            except Exception:
                log.seek(0)
                print(log.read().decode(errors="replace"))
                raise
            finally:
                if child.poll() is None:
                    child.kill()
                    child.wait()
print("Native HTTP, graceful exit, session and undo recovery passed; output disabled")
