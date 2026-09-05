"""Serial transfer of actual late-survivor genomes; no family fitness or action rewards."""
import argparse
import hashlib
import json
from pathlib import Path
import random
import shutil
import struct
import subprocess
import sys

import prepare
from founding_ecology import exclusive_run, save_state


def gene_hash(genome):
    return hashlib.sha256(struct.pack(f"<{len(genome)}f", *genome)).hexdigest()


def seed_next(sample, seed, name):
    """Keep each sampled genome exactly once, then balanced mutated replicas."""
    parents = prepare.bank_genomes(sample)
    assert len(parents) == len(sample["bodies"]) <= 64
    rng = random.Random(seed)
    genomes = [p[:] for p in parents]
    provenance = [dict(parent=i, kind="exact", changed_weights=0,
                       source_gene_sha256=gene_hash(p)) for i, p in enumerate(parents)]
    order = list(range(len(parents)))
    while len(genomes) < 256:
        rng.shuffle(order)
        for i in order:
            if len(genomes) == 256:
                break
            child = [prepare.f32(max(-4, min(4, v + rng.uniform(-.03, .03))))
                     if rng.random() < .02 else prepare.f32(v) for v in parents[i]]
            genomes.append(child)
            provenance.append(dict(parent=i, kind="mutated_replica",
                changed_weights=sum(prepare.f32(a) != b for a, b in zip(parents[i], child)),
                source_gene_sha256=gene_hash(parents[i])))
    bank = prepare.make_bank(genomes, name)
    bank.update(source_seed=sample["source_seed"], source_tick=sample["source_tick"],
        transfer=dict(seed=seed, source_bodies=sample["bodies"], provenance=provenance,
            note="Body age, energy, food and recurrent state reset by normal world initialization. Genes retain mutations; ancestry depth is local to each world."))
    return bank


def case_job(config, seed, rotation):
    return dict(seed=seed, rotation=rotation, population=1024, ticks=config["ticks"],
                contrast=1.0, regeneration=.01, evolving_landscape=True)


def validate_capture(sample, report):
    genes = prepare.bank_genomes(sample)
    observer = report["survivor_observer"]
    assert observer is not None
    assert sample["source_seed"] == report["seed"]
    assert sample["source_tick"] == observer["source_tick"] <= report["elapsed_ticks"]
    assert 0 <= report["elapsed_ticks"] - sample["source_tick"] <= 160
    assert observer["period"] == 128
    assert len(genes) == len(sample["bodies"]) == observer["sampled_bodies"]
    assert len(genes) == min(64, sample["source_population"])
    assert sample["source_population"] == observer["source_population"]
    assert len({b["slot"] for b in sample["bodies"]}) == len(genes)
    return dict(tick=sample["source_tick"], population=sample["source_population"],
        sampled=len(genes), descendants=sum(b["ancestry_depth"] > 0 for b in sample["bodies"]),
        distinct_genomes=len({gene_hash(g) for g in genes}),
        gene_hashes=[gene_hash(g) for g in genes])


def run_case(directory, label, bank_path, job, capture):
    bank = prepare.read(bank_path)
    report_path = directory / f"{label}.json"
    sample_path = directory / f"{label}.survivors.json"
    receipt_path = directory / f"{label}.receipt.json"
    command = [str(directory / "world.exe"), "--headless", "--families", "--founders", str(bank_path),
        "--seed", str(job["seed"]), "--environment-rotation", str(job["rotation"]),
        "--population", str(job["population"]), "--ticks", str(job["ticks"]), "--sample", "256",
        "--output", str(report_path)]
    if capture:
        command += ["--survivors", str(sample_path), "--survivor-sample", "128"]
    command_path = directory / f"{label}.command.json"
    log_path = directory / f"{label}.log"
    if receipt_path.exists():
        receipt = prepare.read(receipt_path)
        assert receipt["input_sha256"] == prepare.sha(bank_path)
        assert receipt["command"] == command == prepare.read(command_path)
        assert receipt["report_sha256"] == prepare.sha(report_path)
        if capture:
            assert receipt["sample_sha256"] == prepare.sha(sample_path)
    else:
        # Never silently reuse an interrupted/changed world or overwrite evidence.
        assert not any(p.exists() for p in [report_path, sample_path, command_path, log_path]), \
            f"Uncommitted partial case {label}; preserve it and inspect before restarting."
        prepare.write_new(command_path, command)
        print(f"Running {label}", flush=True)
        with log_path.open("x", encoding="utf-8") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    report = prepare.read(report_path)
    result = prepare.validate(report, job, bank, include_scores=False)
    result.update(label=label, input_bank=bank_path.name, input_sha256=prepare.sha(bank_path))
    if capture:
        result["capture"] = validate_capture(prepare.read(sample_path), report)
    if not receipt_path.exists():
        prepare.write_new(receipt_path, dict(command=command, input_sha256=prepare.sha(bank_path),
            report_sha256=prepare.sha(report_path),
            sample_sha256=prepare.sha(sample_path) if capture else None))
    return result


def put_bank(path, bank):
    if path.exists():
        assert prepare.read(path) == bank, f"Changed carryover bank: {path}"
    else:
        prepare.write_new(path, bank)


def register(directory, exe, config):
    directory.mkdir(parents=True, exist_ok=False)
    shutil.copy2(exe, directory / "world.exe")
    files = [directory / "world.exe"]
    root = prepare.ROOT
    sources = list((root / "src").glob("*.rs")) + list((root / "shaders").glob("*.wgsl"))
    sources += [root / "Cargo.toml", root / "Cargo.lock"]
    for source in sources:
        dest = directory / "runtime-source" / source.relative_to(root)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, dest)
        files.append(dest)
    for name in ["survivor_loop.py", "prepare.py", "founding_ecology.py", "SURVIVOR_LOOP_PLAN.md"]:
        dest = directory / "replay" / "training" / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(Path(__file__).parent / name, dest)
        files.append(dest)
    rng = random.Random(config["seed"])
    # Unique train/evaluation seeds fixed before observing any results.
    seeds = rng.sample(range(1, 2**32), config["lines"] * config["rounds"] + 2)
    registration = dict(schema=1, experiment="actual_survivor_serial_transfer", config=config,
        train_seeds=seeds[:-2], evaluation_seeds=seeds[-2:],
        evaluation_rounds=sorted({0, config["rounds"] // 2, config["rounds"]}),
        files={p.relative_to(directory).as_posix(): prepare.sha(p) for p in files})
    prepare.write_new(directory / "registration.json", registration)
    return registration


def execute(directory, registration):
    for name, digest in registration["files"].items():
        assert prepare.sha(directory / name) == digest, f"Changed frozen file: {name}"
    for name in ["survivor_loop.py", "prepare.py", "founding_ecology.py"]:
        assert prepare.sha(Path(__file__).parent / name) == registration["files"][f"replay/training/{name}"]
    cfg = registration["config"]
    state = dict(status="running", config=cfg, training=[], evaluation=[], completed_rounds=0)
    save_state(directory, state)
    try:
        for line in range(cfg["lines"]):
            rng = random.Random(cfg["seed"] + 1000003 * line)
            put_bank(directory / f"round0-line{line}.bank.json", prepare.make_bank(
                [prepare.random_genome(rng) for _ in range(256)], f"random-origin-line{line}"))
        for round_number in range(cfg["rounds"] + 1):
            if round_number in registration["evaluation_rounds"]:
                for line in range(cfg["lines"]):
                    for e, seed in enumerate(registration["evaluation_seeds"]):
                        result = run_case(directory, f"eval-r{round_number}-l{line}-e{e}",
                            directory / f"round{round_number}-line{line}.bank.json",
                            case_job(cfg, seed, e), False)
                        result.update(round=round_number, line=line, evaluation_seed=seed)
                        state["evaluation"].append(result)
                        save_state(directory, state)
            if round_number == cfg["rounds"]:
                break
            for line in range(cfg["lines"]):
                label = f"train-r{round_number}-l{line}"
                seed = registration["train_seeds"][round_number * cfg["lines"] + line]
                result = run_case(directory, label,
                    directory / f"round{round_number}-line{line}.bank.json",
                    case_job(cfg, seed, round_number % 4), True)
                sample_path = directory / f"{label}.survivors.json"
                next_bank = seed_next(prepare.read(sample_path), seed ^ 0xa53c917b,
                                     f"survivor-round{round_number+1}-line{line}")
                next_bank["transfer"]["source_sample_sha256"] = prepare.sha(sample_path)
                target = directory / f"round{round_number+1}-line{line}.bank.json"
                put_bank(target, next_bank)
                result.update(round=round_number, line=line, next_bank=target.name,
                              next_sha256=prepare.sha(target))
                state["training"].append(result)
                print(f"  extinct/limit tick={result['tick']}; sample={result['capture']['sampled']} "
                      f"(descendants={result['capture']['descendants']}); next={target.name}", flush=True)
                save_state(directory, state)
            state["completed_rounds"] = round_number + 1
            save_state(directory, state)
        state["status"] = "complete_unpromoted"
        save_state(directory, state)
    except BaseException as exc:
        state.update(status="interrupted_or_failed", error=str(exc))
        save_state(directory, state)
        raise


def main():
    if sys.flags.optimize:
        raise RuntimeError("Integrity checks require Python without -O")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--exe", type=Path, default=prepare.ROOT / "target/feeding-audit/release/primitive_world.exe")
    parser.add_argument("--rounds", type=int, default=8)
    parser.add_argument("--lines", type=int, default=4)
    parser.add_argument("--ticks", type=int, default=8192)
    parser.add_argument("--seed", type=int, default=20260905)
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()
    directory = args.directory.resolve()
    if args.resume:
        registration = prepare.read(directory / "registration.json")
    else:
        assert 1 <= args.rounds <= 1000 and 1 <= args.lines <= 8
        assert 1 <= args.ticks <= 200000
        config = dict(rounds=args.rounds, lines=args.lines, ticks=args.ticks, seed=args.seed)
        registration = register(directory, args.exe.resolve(), config)
    with exclusive_run(directory):
        execute(directory, registration)


if __name__ == "__main__":
    main()
