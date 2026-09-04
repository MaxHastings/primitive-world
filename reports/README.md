# Generated reports

JSON and log files in this directory are local experiment outputs and are
ignored by Git. Keep only durable interpretation, methodology, or regression
notes in version control. The active kernel contract is documented in the
repository root's `KERNEL_SPEC.md`.

Example:

```powershell
cargo run --release -- --headless --ticks 16000 --seed 1 --sample 2000 --output reports/run.json
```
