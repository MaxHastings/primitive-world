# Implementation checks

Current model: `primitive-v42-climate-care`; checkpoint 58, founder bank 21.

- Preserve the original renderer, overlay, world scale, controller, sensing and physics.
- No world-age ramps in settings, UI or biological parameters.
- Test juvenile dependence and ordinary-transfer maturation at multiple world ages.
- Test repeated climate phases over millions of ticks and actual GPU vegetation response.
- Verify refuges, barren edges, gradual depletion and checkpoint continuation.
- Run the restored Rust suite, strict Clippy, formatting and Python tools.
- Inspect the actual wallpaper framebuffer independently of biological tests.

See [recovery](recovery.md).
