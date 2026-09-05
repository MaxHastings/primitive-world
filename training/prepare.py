"""Persistent neuroevolution outside the world: extinction is data, not a reset of search."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import random
import shutil
import struct
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
MODEL, GENES = "primitive-v3", 1760
# Training is capped; 200k is a separate endurance evaluation.
LEVELS = [(2048, 0.0), (4096, 0.0), (8192, .25),
          (16384, .5), (32768, 1.0), (65536, 1.0)]


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_new(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write("\n")


def f32(v):
    return struct.unpack("f", struct.pack("f", v))[0]


def bank_genomes(bank):
    assert bank["model"] == MODEL and bank["version"] == 4
    genomes = bank["genomes"]
    assert 0 < len(genomes) <= 256
    assert all(len(g) == GENES and all(math.isfinite(v) and abs(v) <= 4 for v in g) for g in genomes)
    return genomes


def make_bank(genomes, name):
    bank = dict(model=MODEL, version=4, name=name, source_seed=0, source_tick=0, genomes=genomes)
    bank_genomes(bank)
    return bank


def initial_island(config, island, provided=None):
    if provided is not None:
        genomes = bank_genomes(provided)
        offset = island * config["families"]
        return make_bank([genomes[(offset+j) % len(genomes)][:] for j in range(config["families"])],
                         f"{provided['name']}-island{island}")
    rng = random.Random(config["seed"] + island * 1000003)
    return make_bank([random_genome(rng) for _ in range(config["families"])], f"random-island{island}")


def random_genome(rng):
    # Same exchangeable scale classes as Rust initialization; no action template.
    return [f32(rng.uniform(-1, 1) * (.25 if k < 1488 and k % 93 < 76
                                     else .35 if k < 1488 else .5)) for k in range(GENES)]


def family_scores(report):
    families = report["family_report"]["families"]
    horizon = report["requested_ticks"]
    scores = []
    for family in families:
        founders = family["initial_founders"]
        assert founders > 0
        # Normalization always uses the requested window, never a shorter extinction time.
        scores.append((family["late_descendant_body_ticks"] / (founders * (horizon - horizon // 2)),
                       family["mature_descendant_body_ticks"] / (founders * horizon),
                       family["descendant_body_ticks"] / (founders * horizon),
                       family["founder_body_ticks"] / (founders * horizon)))
    return scores


def feeding_diagnostics(report):
    """Observer summaries only. Never passed to breed() or adapt()."""
    families = report["family_report"]["families"]
    keys = ["births", "matured_descendants", "juvenile_starvation_deaths",
            "adult_descendant_starvation_deaths", "descendant_age_deaths",
            "descendant_other_deaths", "births_to_descendant_parents",
            "births_below_stationary_maturity_energy", "birth_energy_milli",
            "juvenile_collected_milli", "juvenile_ingested_milli", "collected_milli",
            "ingested_milli", "descendant_spent_milli", "juvenile_collect_action_ticks",
            "juvenile_processed_ticks", "energy_at_maturity_milli",
            "juvenile_food_present_ticks", "juvenile_food_present_collect_ticks"]
    totals = {key: sum(f[key] for f in families) for key in keys}
    totals["maximum_depth"] = max((f["maximum_depth"] for f in families), default=0)
    return totals


def verify_completed_rounds(directory, state):
    """Validate ancestry banks before they can seed another, not-yet-run trial."""
    assert len(state["rounds"]) == state["completed_rounds"]
    for number, checkpoint in enumerate(state["rounds"], 1):
        assert checkpoint["round"] == number
        assert read(directory / f"selection-round{number}.json") == checkpoint
        for record in checkpoint["islands"]:
            island = record["island"]
            assert sha(directory / f"round{number-1}-island{island}.bank.json") == record["source_sha256"]
            assert sha(directory / f"round{number}-island{island}.bank.json") == record["next_sha256"]


def breed(genomes, scores, rng):
    assert len(genomes) == len(scores) >= 8
    # Shuffle before sorting so exact fitness ties do not favor low array indices.
    ranking = list(range(len(genomes)))
    rng.shuffle(ranking)
    ranking.sort(key=lambda i: tuple(scores[i]), reverse=True)
    elite_count = max(2, len(genomes) // 4)
    immigrants = max(1, len(genomes) // 20)
    parents = ranking[:max(elite_count, len(genomes) // 2)]
    result = [genomes[i][:] for i in ranking[:elite_count]]
    provenance = [dict(kind="elite", parent=i) for i in ranking[:elite_count]]
    while len(result) < len(genomes) - immigrants:
        parent = rng.choice(parents)
        child = [f32(max(-4, min(4, v + rng.uniform(-.1, .1))))
                 if rng.random() < .02 else v for v in genomes[parent]]
        result.append(child)
        provenance.append(dict(kind="mutant", parent=parent))
    for _ in range(immigrants):
        result.append(random_genome(rng))
        provenance.append(dict(kind="random", parent=None))
    return result, provenance, ranking


def adapt(level, good_streak, poor_streak, frontier_scores):
    established = sum(s[0] >= 1 and s[1] >= .1 for s in frontier_scores) / len(frontier_scores)
    good_streak = good_streak + 1 if established >= .25 else 0
    poor_streak = poor_streak + 1 if all(s[0] == 0 for s in frontier_scores) else 0
    if good_streak >= 2 and level < len(LEVELS) - 1:
        return level + 1, 0, 0
    if poor_streak >= 2 and level > 0:
        return level - 1, 0, 0
    return level, good_streak, poor_streak


def validate(report, job, bank, *, include_scores=True):
    assert report["model"] == MODEL and report["checkpoint_version"] == 15
    assert report["seed"] == job["seed"] and report["requested_ticks"] == job["ticks"]
    assert report["initial_tick"] == 0
    settings = report["initial_settings"]
    assert settings == report["final_settings"]
    fixed = dict(metabolic_cost=.06, movement_energy_cost=.01, motor_response_gain=4,
                 consume_amount=25, conversion_efficiency=8, sensor_radius=24,
                 reproduction_cost=50, maturity_age=400, birth_cooldown=240, heterogeneity=.85)
    fixed.update(habitat_contrast=job["contrast"], resource_regeneration=job["regeneration"])
    for key, value in fixed.items():
        assert math.isclose(settings[key], value, rel_tol=1e-6, abs_tol=1e-8), key
    assert settings.get("environment_rotation", 0) == job["rotation"]
    assert settings["population"] == job["population"]
    assert settings["force_enabled"] and settings["communication_enabled"]
    assert settings["evolving_landscape"] == job.get("evolving_landscape", True)
    expected = bank_genomes(bank)
    assert len(expected) == len(settings["founder_genomes"])
    assert all([f32(v) for v in a] == [f32(v) for v in b] for a, b in zip(expected, settings["founder_genomes"]))
    assert report["famine_at"] == 2**32-1 and report["restore_at"] == 2**32-1
    history = report["history"]
    assert history[0]["tick"] == 0 and history[0]["living"] == job["population"]
    assert all(a["tick"] < b["tick"] for a, b in zip(history, history[1:]))
    for row in history:
        assert row["invalid_outputs"] == 0
        assert job["population"] + row["events"][3] - sum(row["events"][i] for i in [1, 2, 7]) == row["living"]
    end = history[-1]
    assert end["tick"] == job["ticks"] or end["living"] == 0
    assert report["termination_reason"] == ("extinction" if end["living"] == 0 else "tick_limit")
    assert report["elapsed_ticks"] == end["tick"]
    families = report["family_report"]["families"]
    assert report["family_report"]["schema"] == 2
    assert report["family_report"]["requested_horizon"] == job["ticks"]
    assert len(families) == len(expected)
    assert [f["family"] for f in families] == list(range(len(expected)))
    assert sum(f["initial_founders"] for f in families) == job["population"]
    assert sum(f["births"] for f in families) == end["events"][3]
    assert all(0 <= f["late_descendant_body_ticks"] <= f["descendant_body_ticks"]
               and 0 <= f["mature_descendant_body_ticks"] <= f["descendant_body_ticks"] for f in families)
    assert all(f["last_alive_tick"] <= end["tick"] for f in families)
    diagnostics = feeding_diagnostics(report)
    for f in families:
        assert f["matured_descendants"] <= f["births"]
        assert f["births_to_descendant_parents"] <= f["births"]
        assert f["births_below_stationary_maturity_energy"] <= f["births"]
        assert f["juvenile_food_present_collect_ticks"] <= f["juvenile_food_present_ticks"] <= f["juvenile_processed_ticks"]
        assert f["juvenile_collect_action_ticks"] <= f["juvenile_processed_ticks"]
        assert f["juvenile_collected_milli"] <= f["collected_milli"]
        assert f["juvenile_ingested_milli"] <= f["ingested_milli"]
        deaths = sum(f[k] for k in ["juvenile_starvation_deaths", "adult_descendant_starvation_deaths",
                                    "descendant_age_deaths", "descendant_other_deaths"])
        assert deaths <= f["births"]
        if end["living"] == 0:
            assert deaths == f["births"], "Descendant deaths must survive slot reuse"
    result = dict(tick=end["tick"], living=end["living"], births=end["events"][3],
                termination_reason=report["termination_reason"], actions=end["action_ticks"],
                emissions=end["signals"], diagnostics=diagnostics,
                capacity_sample_fraction=sum(r["living"] >= .95 * report["capacity"] for r in history) / len(history))
    if include_scores:
        result["scores"] = family_scores(report)
    return result


def main():
    if sys.flags.optimize:
        raise RuntimeError("Do not disable integrity checks with python -O")
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--exe", type=Path, default=ROOT / "target/release/primitive_world.exe")
    ap.add_argument("--directory", type=Path, required=True)
    ap.add_argument("--seed", type=int, default=20260904)
    ap.add_argument("--rounds", type=int, default=100)
    ap.add_argument("--islands", type=int, default=4)
    ap.add_argument("--families", type=int, default=64)
    ap.add_argument("--replicas", type=int, default=8)
    ap.add_argument("--endurance-ticks", type=int, default=200000)
    ap.add_argument("--initial-bank", type=Path, help="Named compatible starting pool; archived unchanged in the registration")
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--plan-only", action="store_true")
    args = ap.parse_args()
    assert 1 <= args.rounds <= 1000 and 1 <= args.islands <= 8
    assert 8 <= args.families <= 256 and 1 <= args.replicas <= 64
    assert args.families * args.replicas <= 16384
    assert 1 <= args.endurance_ticks <= 200000
    directory = args.directory.resolve()
    config = dict(seed=args.seed, rounds=args.rounds, islands=args.islands, families=args.families,
                  replicas=args.replicas, endurance_ticks=args.endurance_ticks,
                  initializer="provided_bank" if args.initial_bank else "random")
    exe = directory / ("world.exe" if args.exe.suffix == ".exe" else "world")
    if args.resume:
        assert args.initial_bank is None, "Resume uses its archived starting bank; do not supply a replacement"
        registration = read(directory / "registration.json")
        config = registration["config"]
        assert sha(Path(__file__)) == registration["runner_sha256"], "Trainer changed since registration"
        assert sha(ROOT / "training/TRAINING_PLAN.md") == registration["protocol_sha256"]
        assert sha(ROOT / "training/FEEDING_CAMPAIGN.md") == registration["campaign_sha256"]
        assert sha(ROOT / "training/STARTER_PLAN.md") == registration["starter_protocol_sha256"]
        exe = directory / registration["executable_name"]
        state = read(directory / "summary.json")
    else:
        directory.mkdir(parents=True, exist_ok=False)
        if args.initial_bank:
            provided = read(args.initial_bank)
            bank_genomes(provided)
            shutil.copy2(args.initial_bank, directory / "initializer.bank.json")
        shutil.copy2(args.exe.resolve(), exe)
        version = subprocess.check_output([str(exe), "--version"], text=True).strip()
        assert "primitive-v3 / checkpoint 15" in version
        source_paths = list((ROOT / "src").glob("*.rs")) + list((ROOT / "shaders").glob("*.wgsl"))
        source_paths += [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        seed_rng = random.Random(config["seed"])
        used = set()
        def fresh():
            while True:
                s = seed_rng.randrange(1, 2**32)
                if s not in used:
                    used.add(s)
                    return s
        # Pre-register all training cases and separate benchmark/evaluation seeds.
        seeds = [[[fresh() for _ in range(3)] for _ in range(config["islands"])]
                 for _ in range(config["rounds"])]
        registration = dict(schema=2, config=config, model=MODEL, executable_name=exe.name,
                            executable_sha256=sha(exe), version=version, training_seeds=seeds,
                            benchmark_seeds=[fresh(), fresh()], evaluation_seeds=[fresh() for _ in range(4)],
                            source_hashes={str(p.relative_to(ROOT)): sha(p) for p in source_paths},
                            source_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                            runner_sha256=sha(Path(__file__)), protocol_sha256=sha(ROOT / "training/TRAINING_PLAN.md"),
                            campaign_sha256=sha(ROOT / "training/FEEDING_CAMPAIGN.md"),
                            starter_protocol_sha256=sha(ROOT / "training/STARTER_PLAN.md"),
                            initializer_sha256=sha(directory / "initializer.bank.json") if args.initial_bank else None,
                            final_validation=False)
        write_new(directory / "registration.json", registration)
        # Self-contained resume logic, even after the active project evolves.
        replay = directory / "replay/training"
        replay.mkdir(parents=True)
        shutil.copy2(Path(__file__), replay / "prepare.py")
        for name in ["TRAINING_PLAN.md", "FEEDING_CAMPAIGN.md", "STARTER_PLAN.md"]:
            shutil.copy2(ROOT / "training" / name, replay / name)
        state = dict(schema=2, status="planned", completed_rounds=0, trials={}, rounds=[],
                     levels=[0] * config["islands"], good_streaks=[0] * config["islands"],
                     poor_streaks=[0] * config["islands"], final_validation=False)
    assert sha(exe) == registration["executable_sha256"]
    provided = None
    if config["initializer"] == "provided_bank":
        assert sha(directory / "initializer.bank.json") == registration["initializer_sha256"]
        provided = read(directory / "initializer.bank.json")
        bank_genomes(provided)
    verify_completed_rounds(directory, state)
    def save():
        temporary = directory / "summary.pending.json"
        temporary.write_text(json.dumps(state, indent=2, allow_nan=False) + "\n", encoding="utf-8")
        temporary.replace(directory / "summary.json")
    def ensure(path, value):
        if path.exists():
            assert read(path) == value, f"Existing artifact changed: {path}"
        else:
            write_new(path, value)
    def run(label, bank, job):
        bank_path = directory / f"{label}.bank.json"
        ensure(bank_path, bank)
        bank_hash = sha(bank_path)
        output = directory / f"{label}.json"
        command = [str(exe), "--headless", "--families", "--founders", str(bank_path),
                   "--seed", str(job["seed"]), "--ticks", str(job["ticks"]), "--sample", "1024",
                   "--habitat-contrast", str(job["contrast"]), "--regeneration", str(job["regeneration"]),
                   "--environment-rotation", str(job["rotation"]), "--population", str(job["population"]),
                   "--output", str(output)]
        ensure(directory / f"{label}.command.json", command)
        assert sha(exe) == registration["executable_sha256"]
        if label not in state["trials"]:
            state.update(status="running", active=label)
            save()
            if not output.exists():
                log_path = directory / f"{label}.log"
                # Preserve interrupted logs; a new invocation gets a new log, not silent replacement.
                suffix = 0
                while log_path.exists():
                    suffix += 1
                    log_path = directory / f"{label}.resume{suffix}.log"
                print(json.dumps(dict(start=label, **job)), flush=True)
                with log_path.open("x", encoding="utf-8") as log:
                    subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True,
                                   creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
            report = read(output)  # Incomplete/corrupt outputs fail, never disappear during resume.
            result = validate(report, job, bank)
            result.update(report_sha256=sha(output), bank_sha256=bank_hash, job=job)
            assert sha(bank_path) == bank_hash
            state["trials"][label] = result
            save()
            print(json.dumps(dict(finished=label, tick=result["tick"], living=result["living"],
                                  births=result["births"], outcome=result["termination_reason"])), flush=True)
        result = state["trials"][label]
        assert sha(output) == result["report_sha256"] and bank_hash == result["bank_sha256"]
        assert result["job"] == job
        return result

    def pool(banks, name):
        quota = 256 // len(banks)
        return make_bank([g for bank in banks for g in bank["genomes"][:quota]], name)

    # Initial genomes stay on disk even when all of their test bodies die.
    initial = []
    for island in range(config["islands"]):
        bank = initial_island(config, island, provided)
        ensure(directory / f"round0-island{island}.bank.json", bank)
        initial.append(bank)
    initial_pool = pool(initial, "frozen-initial-pool")
    ensure(directory / "initial.bank.json", initial_pool)
    save()
    if args.plan_only:
        print(f"Registered {config['rounds']} rounds; no worlds launched.")
        return
    def benchmarks(round_number, bank):
        for i, seed in enumerate(registration["benchmark_seeds"]):
            run(f"benchmark-r{round_number}-{i}", bank,
                dict(seed=seed, ticks=8192, contrast=1, regeneration=.01, rotation=2*i, population=1000))

    try:
        benchmarks(0, initial_pool)
        # A round may have been committed just before an interrupted benchmark.
        due = list(range(5, state["completed_rounds"] + 1, 5))
        if state["completed_rounds"] == config["rounds"] and config["rounds"] not in due:
            due.append(config["rounds"])
        for completed_round in due:
            saved = [read(directory / f"round{completed_round}-island{i}.bank.json") for i in range(config["islands"])]
            benchmarks(completed_round, pool(saved, f"round{completed_round}-pool"))
        for round_number in range(state["completed_rounds"], config["rounds"]):
            round_results = []
            next_banks = []
            new_levels, new_good, new_poor = [], [], []
            for island in range(config["islands"]):
                bank = read(directory / f"round{round_number}-island{island}.bank.json")
                level = state["levels"][island]
                level_cases = [max(0, level-1), level, min(len(LEVELS)-1, level+1)]
                scores = [[0.0] * 4 for _ in bank["genomes"]]
                frontier = None
                for trial, case_level in enumerate(level_cases):
                    seed = registration["training_seeds"][round_number][island][trial]
                    order = list(range(len(scores)))
                    random.Random(seed).shuffle(order)
                    trial_bank = make_bank([bank["genomes"][i] for i in order],
                                           f"r{round_number}-island{island}-trial{trial}")
                    ticks, contrast = LEVELS[case_level]
                    # Scarcity/recovery variation only after basic persistence is established.
                    regeneration = .01 if level < 3 else [.012, .01, .008][trial]
                    result = run(f"train-r{round_number}-i{island}-t{trial}", trial_bank,
                                 dict(seed=seed, ticks=ticks, contrast=contrast, regeneration=regeneration,
                                      rotation=(round_number+island+trial) % 4,
                                      population=config["families"] * config["replicas"]))
                    restored = [None] * len(scores)
                    for slot, original in enumerate(order):
                        restored[original] = result["scores"][slot]
                        scores[original] = [a + b/3 for a, b in zip(scores[original], result["scores"][slot])]
                    if trial == 1:
                        frontier = restored
                rng = random.Random(config["seed"] + 1000003*island + 999983*(round_number+1))
                children, provenance, ranking = breed(bank_genomes(bank), scores, rng)
                next_bank = make_bank(children, f"round{round_number+1}-island{island}")
                ensure(directory / f"round{round_number+1}-island{island}.bank.json", next_bank)
                next_banks.append(next_bank)
                next_level, good, poor = adapt(level, state["good_streaks"][island],
                                               state["poor_streaks"][island], frontier)
                new_levels.append(next_level); new_good.append(good); new_poor.append(poor)
                round_results.append(dict(island=island, level=level, next_level=next_level,
                                          scores=scores, ranking=ranking, children=provenance,
                                          source_sha256=sha(directory / f"round{round_number}-island{island}.bank.json"),
                                          next_sha256=sha(directory / f"round{round_number+1}-island{island}.bank.json")))
            checkpoint = dict(round=round_number+1, islands=round_results)
            ensure(directory / f"selection-round{round_number+1}.json", checkpoint)
            state["rounds"].append(checkpoint)
            state.update(completed_rounds=round_number+1, levels=new_levels, good_streaks=new_good, poor_streaks=new_poor)
            save()
            print(json.dumps(dict(completed_round=round_number+1, levels=new_levels)), flush=True)
            if (round_number+1) % 5 == 0 or round_number+1 == config["rounds"]:
                benchmarks(round_number+1, pool(next_banks, f"round{round_number+1}-pool"))
        final_banks = [read(directory / f"round{config['rounds']}-island{i}.bank.json") for i in range(config["islands"])]
        candidate = pool(final_banks, f"development-pool-round{config['rounds']}")
        ensure(directory / "candidate.bank.json", candidate)
        for i, seed in enumerate(registration["evaluation_seeds"]):
            for arm in (["candidate", "initial"] if i % 2 == 0 else ["initial", "candidate"]):
                run(f"evaluation-{i}-{arm}", candidate if arm == "candidate" else initial_pool,
                    dict(seed=seed, ticks=config["endurance_ticks"], contrast=1, regeneration=.01,
                         rotation=2*(i % 2), population=1000))
        state.update(status="complete_unpromoted", candidate_sha256=sha(directory / "candidate.bank.json"))
        state.pop("active", None)
        state.pop("error", None)
    except BaseException as error:
        state.update(status="interrupted_or_error", error=repr(error))
        raise
    finally:
        state["updated_at_unix"] = time.time()
        save()


if __name__ == "__main__":
    main()
