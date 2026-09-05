import math
import random
import unittest
import prepare


class PreparationTests(unittest.TestCase):
    def test_provided_initial_pool_is_partitioned_without_added_noise(self):
        genomes = [[i / 100] * prepare.GENES for i in range(32)]
        provided = prepare.make_bank(genomes, "explicit-starter")
        config = dict(seed=91, families=8)
        for island in range(4):
            bank = prepare.initial_island(config, island, provided)
            self.assertEqual(bank["genomes"], genomes[island*8:(island+1)*8])
        before = prepare.initial_island(config, 0)
        rng = random.Random(91)
        self.assertEqual(before["genomes"], [prepare.random_genome(rng) for _ in range(8)])

    def test_random_initialization_has_shape_bounds_and_no_action_template(self):
        first = prepare.random_genome(random.Random(42))
        self.assertEqual(len(first), 1760)
        self.assertEqual(first, prepare.random_genome(random.Random(42)))
        self.assertTrue(all(abs(v) <= .5 for v in first))
        self.assertNotEqual(first[1488+16], first[1488+17+16])

    def test_extinct_families_still_supply_selection_information(self):
        families = [dict(initial_founders=2, late_descendant_body_ticks=40,
                         mature_descendant_body_ticks=10, descendant_body_ticks=100,
                         founder_body_ticks=500),
                    dict(initial_founders=2, late_descendant_body_ticks=0,
                         mature_descendant_body_ticks=0, descendant_body_ticks=0,
                         founder_body_ticks=700)]
        report = dict(requested_ticks=2048, elapsed_ticks=1500, termination_reason="extinction",
                      family_report=dict(families=families))
        scores = prepare.family_scores(report)
        self.assertGreater(scores[0], scores[1])
        self.assertEqual(scores[0][0], 40 / (2 * 1024))
        report["elapsed_ticks"] = 100
        self.assertEqual(prepare.family_scores(report), scores, "Early death must not inflate normalized fitness")

    def test_extinction_does_not_discard_elites_or_stop_mutation(self):
        genomes = [[i/10] * 1760 for i in range(8)]
        scores = [(0, 0, 0, i) for i in range(8)]
        children, ancestry, ranking = prepare.breed(genomes, scores, random.Random(42))
        self.assertEqual(ranking[:2], [7, 6])
        self.assertEqual(children[:2], [genomes[7], genomes[6]])
        self.assertEqual(len(children), 8)
        self.assertTrue(any(p["kind"] == "mutant" for p in ancestry))
        self.assertTrue(any(p["kind"] == "random" for p in ancestry))
        self.assertTrue(all(-4 <= x <= 4 for g in children for x in g))

    def test_difficulty_needs_repeated_family_competence_not_elapsed_rounds(self):
        good = [(1, .1, 1, 1)] * 8
        first = prepare.adapt(0, 0, 0, good)
        self.assertEqual(first, (0, 1, 0))
        self.assertEqual(prepare.adapt(*first, good), (1, 0, 0))
        bad = [(0, 0, 0, .5)] * 8
        first = prepare.adapt(1, 0, 0, bad)
        self.assertEqual(first, (1, 0, 1))
        self.assertEqual(prepare.adapt(*first, bad), (0, 0, 0))
        self.assertEqual(prepare.adapt(0, 0, 2, bad)[0], 0)

    def test_feeding_and_birth_diagnostics_cannot_change_selection_scores(self):
        family = dict(initial_founders=8, late_descendant_body_ticks=10,
                      mature_descendant_body_ticks=20, descendant_body_ticks=30,
                      founder_body_ticks=40)
        report = dict(requested_ticks=2048, family_report=dict(families=[family]))
        before = prepare.family_scores(report)
        family.update(collected_milli=10**12, births=10**6, matured_descendants=10**6,
                      juvenile_starvation_deaths=10**6, birth_energy_milli=10**12)
        self.assertEqual(prepare.family_scores(report), before)

    def test_old_wrong_shape_and_nonfinite_banks_fail(self):
        bank = prepare.make_bank([[0.0] * prepare.GENES], "test")
        for key, value in [("model", "physiology-v2"), ("version", 3), ("genomes", [[0.0]])]:
            with self.assertRaises(AssertionError):
                prepare.bank_genomes(dict(bank, **{key: value}))
        for value in [math.nan, math.inf, 4.1]:
            with self.assertRaises(AssertionError):
                prepare.make_bank([[value] * prepare.GENES], "bad")


if __name__ == "__main__":
    unittest.main()
