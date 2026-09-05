"""Small persistence helpers for observational tools; no evolution policy."""
from contextlib import contextmanager
import json
import sys


@contextmanager
def exclusive_run(directory):
    """OS lock is released on exit/crash; keep the harmless lock file."""
    with (directory / "runner.lock").open("a+b") as lock:
        if lock.tell() == 0:
            lock.write(b"0")
            lock.flush()
        lock.seek(0)
        if sys.platform == "win32":
            import msvcrt
            msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl
            fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        try:
            yield
        finally:
            if sys.platform == "win32":
                lock.seek(0)
                msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def save_state(directory, state):
    temporary = directory / "summary.next.json"
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(state, stream, indent=2, allow_nan=False)
        stream.write("\n")
    temporary.replace(directory / "summary.json")
