import importlib.util
from pathlib import Path
import unittest


REPO_ROOT = Path(__file__).resolve().parents[1]
WRAPPER_SPECS = (
    ("root_whitebox_tools", REPO_ROOT / "whitebox_tools.py"),
    ("wbt_whitebox_tools", REPO_ROOT / "WBT" / "whitebox_tools.py"),
)


def load_wrapper_module(module_name: str, module_path: Path):
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load wrapper module from {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class IFOLPWrapperSmokeTests(unittest.TestCase):
    def assert_ifolp_wrapper_contract(self, module) -> None:
        whitebox_tools_cls = module.WhiteboxTools
        captured = {}

        class ProbeWhiteboxTools(whitebox_tools_cls):
            def run_tool(self, tool_name, args, callback=None):
                captured["tool_name"] = tool_name
                captured["args"] = list(args)
                captured["callback"] = callback
                return 0

        probe = ProbeWhiteboxTools(verbose=False, raise_on_error=False)

        status = probe.iterative_first_order_link_prune(
            d8_pntr="d8.tif",
            upstream_area="area.tif",
            output="streams.tif",
            csa=10.0,
            mscl=100.0,
            threshold_code_raster="codes.tif",
            threshold_table="thresholds.csv",
            esri_pntr=True,
            epsilon=1e-5,
            fail_if_only_channel_pruned=False,
        )

        self.assertEqual(status, 0)
        self.assertEqual(captured["tool_name"], "iterative_first_order_link_prune")
        self.assertEqual(
            captured["args"],
            [
                "--d8_pntr='d8.tif'",
                "--upstream_area='area.tif'",
                "--output='streams.tif'",
                "--csa='10.0'",
                "--mscl='100.0'",
                "--threshold_code_raster='codes.tif'",
                "--threshold_table='thresholds.csv'",
                "--esri_pntr",
                "--epsilon='1e-05'",
                "--fail_if_only_channel_pruned='false'",
            ],
        )

        captured.clear()
        probe.iterative_first_order_link_prune(
            d8_pntr="d8.tif",
            upstream_area="area.tif",
            output="streams.tif",
            csa=10.0,
            mscl=100.0,
            fail_if_only_channel_pruned=True,
        )
        self.assertIn("--fail_if_only_channel_pruned='true'", captured["args"])

        with self.assertRaises(ValueError):
            probe.iterative_first_order_link_prune(
                d8_pntr="d8.tif",
                upstream_area="area.tif",
                output="streams.tif",
                csa=10.0,
                mscl=100.0,
                threshold_code_raster="codes.tif",
            )

        with self.assertRaises(ValueError):
            probe.iterative_first_order_link_prune(
                d8_pntr="d8.tif",
                upstream_area="area.tif",
                output="streams.tif",
                csa=10.0,
                mscl=100.0,
                threshold_table="thresholds.csv",
            )

    def test_ifolp_wrapper_contract_for_both_python_surfaces(self) -> None:
        for module_name, module_path in WRAPPER_SPECS:
            with self.subTest(module=module_name):
                module = load_wrapper_module(module_name, module_path)
                self.assert_ifolp_wrapper_contract(module)


if __name__ == "__main__":
    unittest.main()
