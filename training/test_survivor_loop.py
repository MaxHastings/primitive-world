import random
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import prepare
import survivor_loop as loop


class SurvivorLoopTests(unittest.TestCase):
    def sample(self):
        rng = random.Random(12)
        bank = prepare.make_bank([prepare.random_genome(rng) for _ in range(3)], "actual-children")
        bank.update(source_seed=321, source_tick=2304, source_population=3,
            bodies=[dict(slot=i+20, lineage_id=90+i, ancestry_depth=i) for i in range(3)])
        return bank

    def test_current_genes_carried_exactly_and_mutations_traceable(self):
        sample = self.sample()
        with patch.object(prepare, "family_scores", side_effect=AssertionError("No family ranking")):
            result = loop.seed_next(sample, 456, "next")
        self.assertEqual(result["genomes"][:3], sample["genomes"])
        self.assertEqual(result["source_tick"], 2304)
        self.assertEqual(result["transfer"]["source_bodies"], sample["bodies"])
        self.assertEqual(result, loop.seed_next(sample, 456, "next"))
        counts = [0] * 3
        for child, record in zip(result["genomes"], result["transfer"]["provenance"]):
            parent = sample["genomes"][record["parent"]]
            counts[record["parent"]] += 1
            self.assertEqual(record["source_gene_sha256"], loop.gene_hash(parent))
            self.assertEqual(sum(a != b for a, b in zip(parent, child)), record["changed_weights"])
            self.assertTrue(all(abs(a-b) <= .030001 for a, b in zip(parent, child)))
        self.assertLessEqual(max(counts) - min(counts), 1)
        self.assertTrue(any(p["changed_weights"] > 0 for p in result["transfer"]["provenance"]))

    def test_bottleneck_of_one_is_allowed_and_not_replaced_by_random_ancestors(self):
        sample = self.sample()
        sample["genomes"] = sample["genomes"][:1]
        sample["bodies"] = sample["bodies"][:1]
        result = loop.seed_next(sample, 1, "one")
        self.assertEqual(len(result["genomes"]), 256)
        self.assertEqual(result["genomes"][0], sample["genomes"][0])
        self.assertTrue(all(p["parent"] == 0 for p in result["transfer"]["provenance"]))

    def test_empty_sample_is_not_silently_restarted(self):
        sample = self.sample()
        sample["genomes"] = []
        with self.assertRaises(AssertionError):
            loop.seed_next(sample, 1, "empty")

    def test_f32_hash_is_serialization_independent(self):
        self.assertEqual(loop.gene_hash([.06]), loop.gene_hash([prepare.f32(.06)]))

    def test_modified_next_bank_rejected(self):
        bank = loop.seed_next(self.sample(), 1, "next")
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / "bank.json"
            loop.put_bank(path, bank)
            loop.put_bank(path, bank)
            bank["genomes"][0][0] += .01
            with self.assertRaises(AssertionError):
                loop.put_bank(path, bank)

    def test_capture_rejects_stale_or_mismatched_genomes(self):
        sample = self.sample()
        report = dict(seed=321, elapsed_ticks=2368, survivor_observer=dict(
            source_tick=2304, source_population=3, sampled_bodies=3, period=128))
        self.assertEqual(loop.validate_capture(sample, report)["descendants"], 2)
        report["elapsed_ticks"] = 3000
        with self.assertRaises(AssertionError):
            loop.validate_capture(sample, report)


if __name__ == "__main__":
    unittest.main()
