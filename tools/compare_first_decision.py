"""Compare first-step decisions of copied v45 and converted v46 checkpoints."""
from collections import Counter
from pathlib import Path
import struct
import sys


def blocks(path: Path):
    with path.open("rb") as file:
        file.read(20)
        meta_len = struct.unpack("<I", file.read(4))[0]
        file.read(meta_len)
        result = {}
        for index in range(18):
            size = struct.unpack("<Q", file.read(8))[0]
            if index in (0, 6, 7):
                result[index] = file.read(size)
            else:
                file.seek(size, 1)
        return result


def live(body):
    return {i for i in range(len(body) // 312) if struct.unpack_from("<I", body, i * 312 + 40)[0] == 1}


def selected(decisions, i):
    return struct.unpack_from("<I", decisions, i * 752 + 24)[0]


def inputs(decisions, i):
    return struct.unpack_from("<107f", decisions, i * 752 + 324)


def main():
    a, b = [blocks(Path(p)) for p in sys.argv[1:3]]
    common = sorted(live(a[0]) & live(b[0]))
    print("shared organisms", len(common))
    print("v45 actions", Counter(selected(a[7], i) for i in common))
    print("v46 actions", Counter(selected(b[7], i) for i in common))
    differences = []
    for i in common:
        old, new = inputs(a[7], i), inputs(b[7], i)
        delta = max(abs(x - y) for x, y in zip(old, new))
        differences.append((delta, i))
    print("input deltas max/mean", max(x for x, _ in differences), sum(x for x, _ in differences) / len(differences))
    for delta, i in sorted(differences, reverse=True)[:12]:
        old, new = inputs(a[7], i), inputs(b[7], i)
        channels = [(j, round(old[j], 3), round(new[j], 3)) for j in range(107) if abs(old[j] - new[j]) > 0.1]
        print(i, "actions", selected(a[7], i), selected(b[7], i), "delta", round(delta, 3), "channels", channels[:8])


if __name__ == "__main__":
    main()
