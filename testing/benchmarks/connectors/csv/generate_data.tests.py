"""Focused checks for the deterministic CSV benchmark data generator."""

import contextlib
import importlib.util
import io
import os
import tempfile
import unittest
from pathlib import Path


GENERATOR_PATH = Path(__file__).with_name("generate_data.py")
SPECIFICATION = importlib.util.spec_from_file_location("generate_data", GENERATOR_PATH)
GENERATOR = importlib.util.module_from_spec(SPECIFICATION)
SPECIFICATION.loader.exec_module(GENERATOR)


class GenerateDataTests(unittest.TestCase):
    """Check deterministic generation, idempotence, and per-file repair."""

    def test_generation_and_individual_repairs(self):
        """Preserve valid outputs and repair only each missing or altered output."""
        with tempfile.TemporaryDirectory() as directory:
            data_directory = Path(directory)
            GENERATOR.DATA_DIRECTORY = data_directory
            output = io.StringIO()

            with contextlib.redirect_stdout(output):
                GENERATOR.generate()
            self.assertIn("created:", output.getvalue())
            for filename, expected_hash in GENERATOR.EXPECTED_HASHES.items():
                self.assertEqual(GENERATOR.file_hash(data_directory / filename), expected_hash)

            fixed_time_ns = 1_600_000_000_000_000_000
            for filename in GENERATOR.EXPECTED_HASHES:
                path = data_directory / filename
                path.touch()
                os.utime(path, ns=(fixed_time_ns, fixed_time_ns))
            original_times = {
                filename: (data_directory / filename).stat().st_mtime_ns
                for filename in GENERATOR.EXPECTED_HASHES
            }

            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                GENERATOR.generate()
            self.assertEqual(output.getvalue().count("OK:"), 3)
            self.assertEqual(
                {name: (data_directory / name).stat().st_mtime_ns for name in original_times},
                original_times,
            )

            missing_filename = "small_mixed.csv"
            (data_directory / missing_filename).unlink()
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                GENERATOR.generate()
            self.assertIn(f"created: {missing_filename}", output.getvalue())
            self.assertEqual(
                GENERATOR.file_hash(data_directory / missing_filename),
                GENERATOR.EXPECTED_HASHES[missing_filename],
            )
            missing_file_repaired_time = (data_directory / missing_filename).stat().st_mtime_ns
            for filename in ("large_integer.csv", "large_mixed.csv"):
                self.assertEqual((data_directory / filename).stat().st_mtime_ns, original_times[filename])

            altered_filename = "large_mixed.csv"
            altered_path = data_directory / altered_filename
            altered_path.write_text("altered\n", encoding="utf-8")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                GENERATOR.generate()
            self.assertIn(f"repaired: {altered_filename}", output.getvalue())
            self.assertEqual(
                GENERATOR.file_hash(altered_path), GENERATOR.EXPECTED_HASHES[altered_filename]
            )
            self.assertEqual(
                (data_directory / "large_integer.csv").stat().st_mtime_ns,
                original_times["large_integer.csv"],
            )
            self.assertEqual(
                (data_directory / missing_filename).stat().st_mtime_ns,
                missing_file_repaired_time,
            )


if __name__ == "__main__":
    unittest.main()
