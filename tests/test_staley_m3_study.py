"""Regression coverage for unavailable study matches and nearest-cell selection."""
import importlib.util
from pathlib import Path
import unittest

import numpy as np
from rasterio.transform import from_origin

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('staley_study', ROOT/'tools/staley_m3_resolution_study.py')
study = importlib.util.module_from_spec(spec)
spec.loader.exec_module(study)


class StudyMatchingTests(unittest.TestCase):
    def setUp(self):
        self.study = study.Study(ROOT, ROOT/'target/release/whitebox_tools', Path('/unused'))
        self.grid = dict(area=np.array([[100., 5000., 9000.]]),
                         profile=dict(transform=from_origin(500000, 5200000, 30, 30)))

    def test_empty_or_distant_match_is_explicitly_unavailable(self):
        self.assertIsNone(self.study.snap(self.grid, 0, 0))
        self.grid['area'][:] = 100
        self.assertIsNone(self.study.snap(self.grid, 500045, 5199985))

    def test_nearest_eligible_cell_does_not_optimize_area_agreement(self):
        self.assertEqual(self.study.snap(self.grid, 500045, 5199985), (0, 1, 0.))
        self.assertEqual(self.study.snap(self.grid, 500075, 5199985), (0, 2, 0.))


if __name__ == '__main__':
    unittest.main()
