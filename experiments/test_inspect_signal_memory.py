import unittest
from inspect_signal_memory import response


class SignalPulseTests(unittest.TestCase):
    def test_disconnected_event_has_no_effect(self):
        genome = [0.0]*1518
        self.assertEqual(response(genome, [0.0]*63, .5, 8), response(genome, [0.0]*63, 0, 8))

    def test_connected_event_changes_motor_then_disappears_without_recurrence(self):
        genome = [0.0]*1518
        genome[12] = 1
        genome[1280+6*17] = 1
        trace = response(genome, [0.0]*63, .5, 8)
        self.assertGreater(trace[0][1][0], 0)
        self.assertTrue(all(step[1] == [0.0, 0.0] for step in trace[1:]))

    def test_recurrence_can_retain_a_signal_after_the_pulse_ends(self):
        genome = [0.0]*1518
        genome[12] = 1
        genome[63] = .9
        genome[1280+6*17] = 1
        trace = response(genome, [0.0]*63, .5, 8)
        self.assertGreater(trace[-1][1][0], 0)
        self.assertLess(trace[-1][1][0], trace[0][1][0])


if __name__ == '__main__':
    unittest.main()
