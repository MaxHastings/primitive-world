"""Paired random-origin ecology trials; no genome ranking or cross-world breeding."""
import argparse
from contextlib import contextmanager
import json
from pathlib import Path
import random
import shutil
import struct
import subprocess
import sys
import time

import prepare

CONDITIONS = (("baseline", 1.0, .01), ("uniform", 0.0, .01),
              ("higher_regeneration", 1.0, .03))
TICKS = 16384


def cases():
    result = []
    for i in range(8):
        order = CONDITIONS[i % 3:] + CONDITIONS[:i % 3]
        for name, contrast, regeneration in order:
            result.append(dict(label=f"seed{i}-{name}", bank=f"seed{i}.bank.json",
                condition=name, bank_seed=9043201+i,
                job=dict(seed=9043101+i, rotation=i % 4, population=1000,
                         ticks=TICKS, contrast=contrast, regeneration=regeneration,
                         evolving_landscape=False)))
    return result


def ratio(n, d):
    return n / d if d else None


def float32_settings(value):
    """Both serializers describe f32 settings, but one promotes floats to f64."""
    if isinstance(value, dict):
        return {k: float32_settings(v) for k, v in value.items()}
    if isinstance(value, list):
        return [float32_settings(v) for v in value]
    return prepare.f32(value) if isinstance(value, float) else value


def measurements(report, result):
    d = result["diagnostics"]
    last = report["history"][-1]
    outlived = result["tick"] == TICKS and result["living"] > 0
    return dict(outlived_founders=outlived,
        founding_indicator=outlived and last["living"] > last["juveniles"]
            and d["maximum_depth"] >= 3 and d["births_to_descendant_parents"] > 0,
        juvenile_food_encounter_fraction=ratio(d["juvenile_food_present_ticks"], d["juvenile_processed_ticks"]),
        juvenile_collect_fraction=ratio(d["juvenile_collect_action_ticks"], d["juvenile_processed_ticks"]),
        juvenile_collect_when_food_present=ratio(d["juvenile_food_present_collect_ticks"], d["juvenile_food_present_ticks"]),
        juvenile_digestion_energy_per_tick=ratio(d["juvenile_ingested_milli"] * 8 / 1000, d["juvenile_processed_ticks"]),
        basic_metabolic_cost_per_tick=report["initial_settings"]["metabolic_cost"],
        mean_birth_energy=ratio(d["birth_energy_milli"] / 1000, d["births"]),
        mean_energy_at_maturity=ratio(d["energy_at_maturity_milli"] / 1000, d["matured_descendants"]),
        energy_history=[dict(tick=r["tick"], living=r["living"], juveniles=r["juveniles"],
            energy=r["energy"], mean_living_energy=ratio(r["energy"], r["living"]),
            carried_food=r["carried_food"], vegetation=r["vegetation"], harvested=r["harvested"])
            for r in report["history"]])


def command(directory, case):
    job, label = case["job"], case["label"]
    return [str(directory / "world.exe"), "--headless", "--families", "--static-landscape",
        "--founders", str(directory / case["bank"]), "--seed", str(job["seed"]),
        "--environment-rotation", str(job["rotation"]), "--population", str(job["population"]),
        "--ticks", str(job["ticks"]), "--sample", "256",
        "--habitat-contrast", str(job["contrast"]), "--regeneration", str(job["regeneration"]),
        "--save-checkpoint", str(directory / f"{label}.checkpoint"),
        "--output", str(directory / f"{label}.json")]


@contextmanager
def exclusive_run(directory):
    # OS lock releases on process exit/crash; the harmless lock file is retained.
    with (directory / "runner.lock").open("a+b") as lock:
        if lock.tell() == 0:
            lock.write(b"0"); lock.flush()
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
                lock.seek(0); msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def save_state(directory, state):
    temporary = directory / "summary.next.json"
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(state, stream, indent=2, allow_nan=False)
        stream.write("\n")
    temporary.replace(directory / "summary.json")


def register(directory, origin):
    old = prepare.read(origin / "registration.json")
    exe = origin / "world.exe"
    assert prepare.sha(exe) == old["executable_sha256"]
    directory.mkdir(parents=True, exist_ok=False)
    shutil.copy2(exe, directory / "world.exe")
    shutil.copy2(origin / "registration.json", directory / "runtime-origin.json")
    files = ["world.exe", "runtime-origin.json"]
    for name, expected in old["source_hashes"].items():
        source = prepare.ROOT / Path(name.replace("\\", "/"))
        assert prepare.sha(source) == expected, f"Runtime source differs: {name}"
        target = directory / "runtime-source" / Path(name.replace("\\", "/"))
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        files.append(target.relative_to(directory).as_posix())
    for name in ["founding_ecology.py", "prepare.py", "FOUNDING_ECOLOGY_PLAN.md"]:
        target = directory / "replay" / "training" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(Path(__file__).parent / name, target)
        files.append(target.relative_to(directory).as_posix())
    for i in range(8):
        rng = random.Random(9043201+i)
        bank = prepare.make_bank([prepare.random_genome(rng) for _ in range(256)],
                                 f"random-founding-seed{9043201+i}")
        prepare.write_new(directory / f"seed{i}.bank.json", bank)
        files.append(f"seed{i}.bank.json")
    registered = cases()
    for case in registered:
        name = f"{case['label']}.command.json"
        prepare.write_new(directory / name, command(directory, case))
        files.append(name)
    registration = dict(schema=1, experiment="random_origin_founding_ecology",
        selection=False, final_validation=False, cases=registered,
        runtime_executable_sha256=old["executable_sha256"],
        files={name: prepare.sha(directory / name) for name in files})
    prepare.write_new(directory / "registration.json", registration)
    return registration


def verify(directory, registration):
    assert registration["cases"] == cases()
    assert registration["selection"] is False
    for name, expected in registration["files"].items():
        assert prepare.sha(directory / name) == expected, f"Changed artifact: {name}"
    for name, source in [("founding_ecology.py", Path(__file__)), ("prepare.py", Path(prepare.__file__))]:
        assert prepare.sha(source) == registration["files"][f"replay/training/{name}"], "Use the frozen replay runner"


def inspect_case(directory, case):
    label = case["label"]
    output, checkpoint = directory / f"{label}.json", directory / f"{label}.checkpoint"
    report = prepare.read(output)
    result = prepare.validate(report, case["job"], prepare.read(directory / case["bank"]), include_scores=False)
    with checkpoint.open("rb") as stream:
        header = stream.read(24)
        assert header[:12] == b"PRIMWORLD015"
        seed, tick, size = struct.unpack("<III", header[12:])
        assert seed == case["job"]["seed"] and tick == result["tick"]
        assert float32_settings(json.loads(stream.read(size))) == float32_settings(report["final_settings"])
    result.update(measurements(report, result), report_sha256=prepare.sha(output),
        checkpoint_sha256=prepare.sha(checkpoint), condition=case["condition"], seed=case["job"]["seed"])
    return result


def main():
    if sys.flags.optimize:
        raise RuntimeError("Do not disable integrity checks with python -O")
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--directory", type=Path, required=True)
    ap.add_argument("--runtime-origin", type=Path)
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--plan-only", action="store_true")
    args = ap.parse_args()
    directory = args.directory.resolve()
    if args.resume:
        assert args.runtime_origin is None, "Resume uses registered runtime"
        registration = prepare.read(directory / "registration.json")
    else:
        assert args.runtime_origin is not None
        registration = register(directory, args.runtime_origin.resolve())
    with exclusive_run(directory):
        verify(directory, registration)
        state = prepare.read(directory / "summary.json") if args.resume else dict(
            status="registered", selection=False, final_validation=False, results={})
        if args.plan_only:
            save_state(directory, state)
            print(f"Registered {len(cases())} cases in {directory}", flush=True)
            return
        state.update(status="running")
        state.pop("error", None)
        try:
            for case in registration["cases"]:
                label = case["label"]
                state["active"] = label
                save_state(directory, state)
                if label in state["results"]:
                    assert inspect_case(directory, case) == state["results"][label], "Changed completed case"
                    continue
                output = directory / f"{label}.json"
                checkpoint = directory / f"{label}.checkpoint"
                if output.exists() or checkpoint.exists():
                    # Recover only a fully completed, valid case. Partial files
                    # are evidence: inspect_case must fail without overwriting.
                    result = inspect_case(directory, case)
                else:
                    log = directory / f"{label}.log"
                    suffix = 0
                    while log.exists():
                        suffix += 1; log = directory / f"{label}.resume{suffix}.log"
                    print(f"Starting {label}", flush=True)
                    with log.open("x", encoding="utf-8") as stream:
                        subprocess.run(prepare.read(directory / f"{label}.command.json"),
                            stdout=stream, stderr=subprocess.STDOUT, check=True,
                            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
                    result = inspect_case(directory, case)
                state["results"][label] = result
                state["completed_cases"] = len(state["results"])
                save_state(directory, state)
                print(json.dumps(dict(case=label, tick=result["tick"], living=result["living"],
                    depth=result["diagnostics"]["maximum_depth"], founding=result["founding_indicator"])), flush=True)
            verify(directory, registration)
            state.update(status="complete_unpromoted")
            state.pop("active", None)
        except BaseException as error:
            state.update(status="interrupted_or_error", error=repr(error))
            raise
        finally:
            state["updated_at_unix"] = time.time()
            save_state(directory, state)


if __name__ == "__main__":
    main()
