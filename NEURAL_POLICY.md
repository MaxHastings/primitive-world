# Optional recurrent policy

Primitive World keeps the authored local-rule policy as its default. `--neural`
enables a compact shared policy with 12 local observations, a 16-unit tanh
recurrent state, and seven action logits. Every agent owns its hidden state;
the policy receives no population-wide state and cannot create groups directly.
The action mask is still supplied by the authored affordances, so a neural
policy can choose among actions that are physically possible without bypassing
resource accounting, social range, births, or death.

The shipped weights are a neutral baseline for exercising the complete path.
They are not trained competence. Export them with:

```powershell
cargo run --release -- --headless --neural-export policy.json
```

The current release exports and loads weights, but does not claim a complete
on-policy training loop. A correct trainer needs contiguous per-agent
pre-action observations, hidden states, physical masks, elapsed rewards, and
held-out evaluation; those are deliberately left for the next experiment
rather than inferred from aggregate telemetry.
