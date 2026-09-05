import unittest
from pathlib import Path
import watch_survivors as watch


class WatchTests(unittest.TestCase):
    def test_checkpoint_resume_does_not_override_world_or_reset_genes(self):
        cmd = watch.command(Path("world.exe"), Path("new"), None, 42,
                            checkpoint=Path("paused.checkpoint"), speed="16x")
        self.assertIn("--checkpoint", cmd)
        for option in ["--founders", "--seed", "--population", "--ticks"]:
            self.assertNotIn(option, cmd)
        self.assertEqual(cmd[cmd.index("--view-speed")+1], "16x")

    def test_command_has_no_headless_mode_or_tick_limit(self):
        cmd = watch.command(Path("world.exe"), Path("new"), Path("bank.json"), 42)
        self.assertIn("--watch-loop", cmd)
        self.assertNotIn("--watch-output", cmd)
        self.assertEqual(cmd[cmd.index("--view-speed")+1], "MAX")
        self.assertNotIn("--headless", cmd)
        self.assertNotIn("--ticks", cmd)

    def test_physical_edits_carry_to_next_world(self):
        settings = dict(population=900, resource_regeneration=.003, metabolic_cost=.08,
            movement_energy_cost=.02, motor_response_gain=6, habitat_contrast=1,
            environment_rotation=2, evolving_landscape=False, force_enabled=False,
            communication_enabled=True)
        cmd = watch.command(Path("world.exe"), Path("new"), Path("bank.json"), 42, settings)
        self.assertEqual(cmd[cmd.index("--metabolic-cost")+1], "0.08")
        self.assertIn("--static-landscape", cmd)
        self.assertIn("--no-force", cmd)
        self.assertNotIn("--no-signals", cmd)
        del settings["environment_rotation"]  # Rust omits zero rotation from JSON.
        cmd = watch.command(Path("world.exe"), Path("new"), Path("bank.json"), 42, settings)
        self.assertEqual(cmd[cmd.index("--environment-rotation")+1], "0")
        settings["population"] = 0
        with self.assertRaises(AssertionError):
            watch.command(Path("world.exe"), Path("new"), Path("bank.json"), 42, settings)


if __name__ == "__main__":
    unittest.main()
