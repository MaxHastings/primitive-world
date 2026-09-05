import copy
from pathlib import Path
import unittest
from unittest.mock import patch

import founding_ecology as ecology
import prepare


def fixture():
    bank = prepare.make_bank([[0.0] * prepare.GENES], "fixture")
    job = dict(seed=1, ticks=64, contrast=1.0, regeneration=.01, rotation=0,
               population=1, evolving_landscape=False)
    settings = dict(metabolic_cost=.06, movement_energy_cost=.01, motor_response_gain=4,
        consume_amount=25, conversion_efficiency=8, sensor_radius=24,
        reproduction_cost=50, maturity_age=400, birth_cooldown=240, heterogeneity=.85,
        habitat_contrast=1.0, resource_regeneration=.01, environment_rotation=0,
        population=1, force_enabled=True, communication_enabled=True,
        evolving_landscape=False, founder_genomes=bank["genomes"])
    family = dict.fromkeys([
        "births", "maximum_depth", "matured_descendants", "juvenile_starvation_deaths",
        "adult_descendant_starvation_deaths", "descendant_age_deaths", "descendant_other_deaths",
        "births_to_descendant_parents", "births_below_stationary_maturity_energy", "birth_energy_milli",
        "juvenile_collected_milli", "juvenile_ingested_milli", "collected_milli", "ingested_milli",
        "descendant_spent_milli", "juvenile_collect_action_ticks", "juvenile_processed_ticks",
        "energy_at_maturity_milli", "juvenile_food_present_ticks", "juvenile_food_present_collect_ticks",
        "late_descendant_body_ticks", "mature_descendant_body_ticks", "descendant_body_ticks"], 0)
    family.update(family=0, initial_founders=1, founder_body_ticks=64, last_alive_tick=64)
    history = [dict(tick=t, living=1, events=[0]*8, invalid_outputs=0,
                    action_ticks=[0]*6, signals=0) for t in [0, 64]]
    report = dict(model="primitive-v3", checkpoint_version=15, seed=1, requested_ticks=64,
        elapsed_ticks=64, initial_tick=0, initial_settings=settings, final_settings=copy.deepcopy(settings),
        famine_at=2**32-1, restore_at=2**32-1, history=history, capacity=16384,
        termination_reason="tick_limit", family_report=dict(schema=2, requested_horizon=64, families=[family]))
    return report, job, bank


class FoundingEcologyTests(unittest.TestCase):
    def test_checkpoint_and_report_compare_at_exact_simulator_precision(self):
        checkpoint = dict(cost=.06, genes=[[.01]], enabled=False, population=1000)
        report = dict(cost=prepare.f32(.06), genes=[[prepare.f32(.01)]], enabled=False, population=1000)
        self.assertEqual(ecology.float32_settings(checkpoint), ecology.float32_settings(report))
        report["cost"] = .060001
        self.assertNotEqual(ecology.float32_settings(checkpoint), ecology.float32_settings(report))

    def test_eight_paired_triples_with_only_declared_environment_differences(self):
        cases = ecology.cases()
        self.assertEqual(len(cases), 24)
        self.assertEqual(len({case["label"] for case in cases}), 24)
        for i in range(8):
            group = cases[3*i:3*i+3]
            self.assertEqual({case["bank"] for case in group}, {f"seed{i}.bank.json"})
            self.assertEqual({case["job"]["seed"] for case in group}, {9043101+i})
            self.assertEqual({case["bank_seed"] for case in group}, {9043201+i})
            self.assertEqual({(c["condition"], c["job"]["contrast"], c["job"]["regeneration"])
                              for c in group}, set(ecology.CONDITIONS))
            self.assertTrue(all(c["job"]["ticks"] == 16384 and c["job"]["evolving_landscape"] is False
                                and c["job"]["population"] == 1000 for c in group))

    def test_commands_preserve_body_and_checkpoint_actual_world(self):
        for case in ecology.cases():
            command = ecology.command(Path("trial"), case)
            self.assertIn("--static-landscape", command)
            self.assertIn("--save-checkpoint", command)
            self.assertIn("--headless", command)
            for forbidden in ["--metabolic-cost", "--movement-cost", "--motor-gain", "--no-force",
                              "--no-signals", "--famine-at", "--restore-at", "--export-founders"]:
                self.assertNotIn(forbidden, command)

    def test_observer_validation_never_computes_fitness(self):
        report, job, bank = fixture()
        with patch.object(prepare, "family_scores", side_effect=AssertionError("No fitness in this experiment")):
            result = prepare.validate(report, job, bank, include_scores=False)
        self.assertNotIn("scores", result)
        self.assertIn("scores", prepare.validate(report, job, bank))

    def test_wrong_landscape_and_changed_settings_fail(self):
        report, job, bank = fixture()
        report["initial_settings"]["evolving_landscape"] = True
        report["final_settings"]["evolving_landscape"] = True
        with self.assertRaises(AssertionError):
            prepare.validate(report, job, bank, include_scores=False)
        report, job, bank = fixture()
        report["final_settings"]["metabolic_cost"] = .005
        with self.assertRaises(AssertionError):
            prepare.validate(report, job, bank, include_scores=False)

    def test_existing_trainer_defaults_still_require_evolving_geography_and_return_scores(self):
        report, job, bank = fixture()
        job.pop("evolving_landscape")
        with self.assertRaises(AssertionError):
            prepare.validate(report, job, bank)
        report["initial_settings"]["evolving_landscape"] = True
        report["final_settings"]["evolving_landscape"] = True
        self.assertIn("scores", prepare.validate(report, job, bank))

    def test_measurements_distinguish_alive_from_founding_and_do_not_divide_by_zero(self):
        report, job, bank = fixture()
        result = prepare.validate(report, job, bank, include_scores=False)
        report["history"] = [dict(tick=16384, living=1, juveniles=1, energy=2,
                                  carried_food=0, vegetation=10, harvested=1)]
        result.update(tick=16384, living=1)
        measures = ecology.measurements(report, result)
        self.assertTrue(measures["outlived_founders"])
        self.assertFalse(measures["founding_indicator"])
        self.assertIsNone(measures["mean_birth_energy"])
        self.assertIsNone(measures["juvenile_digestion_energy_per_tick"])
        report["history"][0]["juveniles"] = 0
        result["diagnostics"].update(maximum_depth=3, births_to_descendant_parents=1,
                                      juvenile_ingested_milli=1000, juvenile_processed_ticks=200)
        measures = ecology.measurements(report, result)
        self.assertTrue(measures["founding_indicator"])
        self.assertEqual(measures["juvenile_digestion_energy_per_tick"], .04)
        result["living"] = 0
        self.assertFalse(ecology.measurements(report, result)["outlived_founders"])


if __name__ == "__main__":
    unittest.main()
