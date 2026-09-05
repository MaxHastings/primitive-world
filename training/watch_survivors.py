"""Launch one native viewer: extinction transitions in place; closing saves and stops."""
import argparse
from pathlib import Path
import subprocess

import prepare


def command(exe, world, bank, seed, settings=None, *, checkpoint=None, speed="MAX"):
    args = [str(exe), "--watch-loop", str(world), "--view-speed", speed]
    if checkpoint is not None:
        return args + ["--checkpoint", str(checkpoint)]
    args += ["--founders", str(bank), "--seed", str(seed)]
    if settings is None:
        args += ["--population", "1024"]
    else:
        assert settings["population"] > 0, "Initial bodies must be positive"
        for option, key in [("--population", "population"), ("--regeneration", "resource_regeneration"),
            ("--metabolic-cost", "metabolic_cost"), ("--movement-cost", "movement_energy_cost"),
            ("--motor-gain", "motor_response_gain"), ("--habitat-contrast", "habitat_contrast"),
            ("--environment-rotation", "environment_rotation")]:
            value = settings.get(key, 0) if key == "environment_rotation" else settings[key]
            args += [option, str(value)]
        for flag, key in [("--static-landscape", "evolving_landscape"), ("--no-force", "force_enabled"),
                          ("--no-signals", "communication_enabled")]:
            if not settings[key]:
                args.append(flag)
    return args


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, required=True)
    origin = parser.add_mutually_exclusive_group(required=True)
    origin.add_argument("--initial-bank", type=Path)
    origin.add_argument("--checkpoint", type=Path)
    parser.add_argument("--exe", type=Path, default=prepare.ROOT / "target/feeding-audit/release/primitive_world.exe")
    parser.add_argument("--seed", type=int, default=9054001)
    parser.add_argument("--view-speed", choices=["1x", "2x", "4x", "8x", "16x", "MAX"], default="MAX")
    args = parser.parse_args()
    directory = args.directory.resolve()
    if directory.exists():
        raise FileExistsError(f"Choose a new loop directory: {directory}")
    directory.parent.mkdir(parents=True, exist_ok=True)
    if args.initial_bank:
        prepare.bank_genomes(prepare.read(args.initial_bank))
    cmd = command(args.exe.resolve(strict=True), directory,
        args.initial_bank.resolve(strict=True) if args.initial_bank else None, args.seed,
        checkpoint=args.checkpoint.resolve(strict=True) if args.checkpoint else None,
        speed=args.view_speed)
    print("Opening one window. Extinction advances in place; speed/camera stay unchanged. Close to save and stop.", flush=True)
    # Only one launch. No timeout, external generation loop, or automatic reopening.
    subprocess.run(cmd, check=True)


if __name__ == "__main__":
    main()
