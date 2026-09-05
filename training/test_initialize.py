import math
import random
import unittest
import initialize
import prepare


def outputs(genes, inputs):
    state = [math.tanh(sum(genes[h*93+k]*x for k, x in enumerate(inputs)) + genes[h*93+92])
             for h in range(16)]
    return [sum(genes[1488+o*17+h]*v for h, v in enumerate(state)) + genes[1488+o*17+16]
            for o in range(16)]


class InitializerTests(unittest.TestCase):
    def test_shape_variation_provenance_and_reproducibility(self):
        bank = initialize.starter_bank(count=16)
        prepare.bank_genomes(bank)
        self.assertEqual(bank, initialize.starter_bank(count=16))
        self.assertEqual(len({tuple(g) for g in bank["genomes"]}), 16)
        self.assertEqual(bank["provenance"]["kind"], "authored_initializer_not_evolved")

    def test_disclosed_low_reserve_collection_and_high_reserve_reproduction(self):
        for genome in initialize.starter_bank(count=32)["genomes"]:
            for energy, expected in [(.2, 1), (.98, 5)]:
                inputs = [0.0] * 76
                inputs[0], inputs[2] = energy, .2
                out = outputs(genome, inputs)
                self.assertEqual(max(range(6), key=lambda i: out[i]), expected)
                self.assertGreater(40 / (1 + math.exp(-out[8])), 30)

    def test_local_response_is_reversible_and_genes_remain_mutable(self):
        genome = initialize.starter_bank(count=8)["genomes"][0]
        inputs = [0.0] * 76
        inputs[23] = .5
        self.assertGreater(outputs(genome, inputs)[6], 0)
        inputs[23], inputs[29] = 0, .5
        self.assertLess(outputs(genome, inputs)[6], 0)
        children, provenance, _ = prepare.breed([genome[:] for _ in range(8)],
                                                [(0, 0, 0, 0)] * 8, random.Random(12))
        self.assertTrue(any(child != genome for child, p in zip(children, provenance) if p["kind"] == "mutant"))


if __name__ == "__main__":
    unittest.main()
