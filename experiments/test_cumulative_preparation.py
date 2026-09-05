"""Offline integrity tests; these do not test ecological adaptation."""
import copy
import unittest

from cumulative_preparation import bank_statistics, packed, physical, validate_report


class IntegrityTests(unittest.TestCase):
    def setUp(self):
        self.bank = dict(name="test", version=3, model="physiology-v2", genomes=[[0.0]*1518])
        self.settings = dict(population=1000, metabolic_cost=.06, founder_name="test", founder_genomes=self.bank["genomes"])
        initial = dict(tick=0, living=1000, events=[0]*8, invalid_outputs=0)
        final = dict(tick=200000, living=900, events=[0, 200, 0, 100, 0, 0, 0, 0], invalid_outputs=0)
        self.report = dict(model="physiology-v2", checkpoint_version=14, seed=808, initial_tick=0,
                           requested_ticks=200000, elapsed_ticks=200000, famine_at=4294967295,
                           restore_at=4294967295, initial_settings=self.settings,
                           final_settings=copy.deepcopy(self.settings), history=[initial, final],
                           travel_observer=dict(stats=dict(invalid_observations=0)), founder_export=None,
                           journey_observer=dict(schema=2, sample_ticks=32, stats=dict(invalid_observations=0)))

    def validate(self, report):
        validate_report(report, self.bank, physical(self.settings), 808, 200000, True)

    def test_valid_result_and_extinction(self):
        self.validate(self.report)
        self.report["history"][-1].update(tick=1024, living=0, events=[0, 1000, 0, 0, 0, 0, 0, 0])
        self.report["elapsed_ticks"] = 1024
        self.validate(self.report)

    def test_wrong_weights_physics_intervention_and_partial_run_are_rejected(self):
        for mutation in [
            lambda r: r["initial_settings"]["founder_genomes"][0].__setitem__(0, .01),
            lambda r: r["initial_settings"].__setitem__("metabolic_cost", .005),
            lambda r: r.__setitem__("famine_at", 1024),
            lambda r: r["history"][-1].__setitem__("tick", 1000),
            lambda r: r["history"][-1].__setitem__("living", 999),
            lambda r: r["history"][-1].__setitem__("invalid_outputs", 1),
            lambda r: r.__setitem__("founder_export", {"Ok": None}),
            lambda r: r["journey_observer"]["stats"].__setitem__("invalid_observations", 1),
        ]:
            report = copy.deepcopy(self.report)
            mutation(report)
            with self.assertRaises(AssertionError):
                self.validate(report)

    def test_float32_roundtrip_and_bank_statistics_are_not_fitness(self):
        self.assertEqual(packed([[.1]]), packed([[.10000000149011612]]))
        stats = bank_statistics(self.bank, self.bank)
        self.assertEqual(stats["mean_genome_rms_difference_from_baseline"], 0)
        self.assertEqual(stats["mean_within_bank_gene_variance"], 0)
        bank = copy.deepcopy(self.bank)
        bank["genomes"].append([.1]*1518)
        self.assertGreater(bank_statistics(bank, self.bank)["mean_within_bank_gene_variance"], 0)
        bank["genomes"][0][0] = float("nan")
        with self.assertRaises(AssertionError):
            bank_statistics(bank, self.bank)


if __name__ == "__main__":
    unittest.main()
