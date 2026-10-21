"""Optional harness regressions; uses the locked project-trial Python environment."""
import tempfile
from pathlib import Path
import unittest

import run


class TrialGuards(unittest.TestCase):
    def test_modified_cached_source_fails_before_any_formatter_runs(self):
        with tempfile.TemporaryDirectory() as directory:
            cache = Path(directory)
            source = cache / "sources/example/main.gd"
            source.parent.mkdir(parents=True)
            source.write_text("var value = 2\n")
            project = {"id": "example", "files": [{"path": "main.gd", "sha256": run.digest(b"var value = 1\n")}]}
            with self.assertRaisesRegex(RuntimeError, "cached source hash mismatch"):
                run.prepare(project, cache, False)

    def test_review_rejects_changed_code_and_comments(self):
        with self.assertRaisesRegex(RuntimeError, "unexplained token"):
            run.classify("var value = 1\n", "var value = 2\n")
        with self.assertRaisesRegex(RuntimeError, "unexplained comment"):
            run.classify("# first\nvar value = 1\n", "# second\nvar value = 1\n")

    def test_quote_punctuation_and_layout_policies_are_explained(self):
        categories = run.classify("var value = ['hello',]\n", 'var value = [\n\t"hello"\n]\n')
        self.assertEqual(categories, ["layout", "parentheses-commas-semicolons", "quotes"])


if __name__ == "__main__":
    unittest.main()
