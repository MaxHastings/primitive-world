import unittest
from inspect_bank_direction import movement, probe


class DirectionProbeTests(unittest.TestCase):
    def test_zero_brain_does_not_invent_motion(self):
        self.assertEqual(movement([0.0]*1518, probe(50, 2, 1, .2)), [0.0, 0.0])

    def test_bias_uses_correct_motor_outputs_and_speed_bound(self):
        genome = [0.0]*1518
        genome[1280+6*17+16] = -1
        genome[1280+7*17+16] = 1
        vx, vy = movement(genome, probe(50, 2, None, 0))
        self.assertLess(vx, 0)
        self.assertGreater(vy, 0)
        self.assertAlmostEqual(vx, -vy)
        self.assertAlmostEqual((vx*vx+vy*vy)**.5, 1.2)

    def test_mirrored_cues_change_only_food_side(self):
        right, left = probe(50, 2, 1, .02), probe(50, 2, 3, .02)
        self.assertEqual([i for i, (a, b) in enumerate(zip(right, left)) if a != b], [18, 24, 30, 36])
        self.assertEqual(right[2], 0)
        self.assertEqual(right[1], .25)


if __name__ == '__main__':
    unittest.main()
