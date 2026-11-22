import subprocess
import sys
import unittest
from unittest.mock import Mock

from bench_terminal import (
    finish_helper, measurements, pixel_measurements,
    sample_scroll_positions, validate_scroll_positions,
)


class ScrollTest(unittest.TestCase):
    def test_nonzero_position_stuck_after_warmup_is_rejected(self):
        samples = [{"time": stamp, "offset": 150} for stamp in [10, 10.5, 11]]
        with self.assertRaisesRegex(RuntimeError, "did not change"):
            validate_scroll_positions(samples, 10, 12)

    def test_only_measured_movement_counts(self):
        samples = [{"time": 9, "offset": 20}, {"time": 10, "offset": 150},
                   {"time": 11, "offset": 150}, {"time": 12, "offset": 200}]
        with self.assertRaises(RuntimeError):
            validate_scroll_positions(samples, 10, 12)

    def test_reversal_can_return_to_the_starting_offset(self):
        samples = [{"time": 10, "offset": 150}, {"time": 10.5, "offset": 200},
                   {"time": 11, "offset": 150}]
        validate_scroll_positions(samples, 10, 12)

    def test_missing_or_single_sample_is_rejected(self):
        for samples in [[], [{"time": 10, "offset": 150}]]:
            with self.assertRaises(RuntimeError):
                validate_scroll_positions(samples, 10, 12)

    def test_sampler_observes_movement_throughout_the_interval(self):
        clock = [10.0]
        offsets = iter([150, 200, 150, 100])

        def sleep(seconds):
            clock[0] += seconds

        def position():
            return {"pane": {"scroll": {"offset_from_bottom": next(offsets)}}}

        samples = sample_scroll_positions(12, position, lambda: clock[0], sleep)
        self.assertEqual([sample["time"] for sample in samples], [10, 10.5, 11, 11.5])
        self.assertEqual([sample["offset"] for sample in samples], [150, 200, 150, 100])
        validate_scroll_positions(samples, 10, 12)
        self.assertEqual(clock[0], 12)

    def test_slow_readback_cannot_supply_out_of_interval_movement(self):
        clock = [10.0]

        def position():
            clock[0] = 13
            return {"pane": {"scroll": {"offset_from_bottom": 200}}}

        self.assertEqual(sample_scroll_positions(12, position, lambda: clock[0]), [])

    def test_helper_is_stopped_explicitly_after_measurement(self):
        process = subprocess.Popen(
            [sys.executable, "-c", "import sys; sys.stdin.buffer.read()"], stdin=subprocess.PIPE,
        )
        try:
            self.assertIsNone(process.poll())
            finish_helper(process, "test helper")
            self.assertEqual(process.returncode, 0)
            self.assertTrue(process.stdin.closed)
        finally:
            if process.poll() is None:
                process.kill()
            process.wait()
            process.stdin.close()

    def test_early_successful_exit_is_not_valid_input_coverage(self):
        process = Mock()
        process.poll.return_value = 0
        with self.assertRaisesRegex(RuntimeError, "before measurement ended"):
            finish_helper(process, "test helper")
        process.stdin.close.assert_not_called()

    def test_helper_failure_on_shutdown_is_reported(self):
        process = Mock()
        process.poll.return_value = None
        process.wait.return_value = 1
        with self.assertRaisesRegex(RuntimeError, "failed"):
            finish_helper(process, "test helper")
        process.stdin.close.assert_called_once()
        process.wait.assert_called_once_with(timeout=5)


class MeasurementsTest(unittest.TestCase):
    def test_pixel_stalls_are_independent_of_window_frame_counts(self):
        records = [{"start": 9}, {"time": 9.5}, {"time": 11}, {"time": 20}, {"end": 23}]
        result = pixel_measurements(records, 10, 22)
        self.assertEqual(result["pixel_changes"], 2)
        self.assertEqual(result["pixel_max_silence_ms"], 9000)

    def test_unchanged_pixels_count_the_whole_interval(self):
        records = [{"start": 9}, {"time": 9.5}, {"end": 23}]
        result = pixel_measurements(records, 10, 22)
        self.assertEqual(result["pixel_changes"], 0)
        self.assertEqual(result["pixel_max_silence_ms"], 12000)

    def test_incomplete_pixel_capture_is_not_a_frozen_terminal(self):
        for records in [[], [{"start": 9}, {"end": 23}],
                        [{"start": 9}, {"time": 11}, {"end": 15}]]:
            with self.assertRaises(RuntimeError):
                pixel_measurements(records, 10, 22)

    def test_a_frozen_phase_counts_the_entire_interval(self):
        result = measurements([], 10, 22, 2, 3)
        self.assertEqual(result["frames"], 0)
        self.assertEqual(result["fps"], 0)
        self.assertIsNone(result["p95_ms"])
        self.assertIsNone(result["max_ms"])
        self.assertEqual(result["max_silence_ms"], 12000)

    def test_one_frame_does_not_hide_a_long_boundary_stall(self):
        result = measurements([(11, 2)], 10, 22, 2, 3)
        self.assertEqual(result["frames"], 1)
        self.assertEqual(result["p95_ms"], 2)
        self.assertEqual(result["max_gap_ms"], 0)
        self.assertEqual(result["max_silence_ms"], 11000)

    def test_internal_gaps_and_frame_cost_are_distinct(self):
        result = measurements([(10.5, 2), (11, 20), (20, 3)], 10, 22, 2, 5)
        self.assertEqual(result["fps"], 0.25)
        self.assertEqual(result["p95_ms"], 20)
        self.assertEqual(result["max_gap_ms"], 9000)
        self.assertEqual(result["max_silence_ms"], 9000)
        self.assertAlmostEqual(result["over_16_7_pct"], 100 / 3)
        self.assertEqual(result["cpu_pct"], 25)


if __name__ == "__main__":
    unittest.main()
