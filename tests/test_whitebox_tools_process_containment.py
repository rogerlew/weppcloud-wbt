import importlib.util
import os
from pathlib import Path
import tempfile
import textwrap
import time
import unittest
from unittest.mock import patch


REPO_ROOT = Path(__file__).resolve().parents[1]
WRAPPER_SPECS = (
    ("root_whitebox_tools_containment", REPO_ROOT / "whitebox_tools.py"),
    ("wbt_whitebox_tools_containment", REPO_ROOT / "WBT" / "whitebox_tools.py"),
)


def load_wrapper_module(module_name: str, module_path: Path):
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load wrapper module from {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_fake_whitebox_tools(directory: Path) -> None:
    executable = directory / "whitebox_tools"
    executable.write_text(
        textwrap.dedent(
            """\
            #!/usr/bin/env python3
            import os
            from pathlib import Path
            import subprocess
            import sys
            import time

            mode = os.environ["FAKE_WBT_MODE"]
            if mode == "nonzero":
                sys.exit(7)

            child = subprocess.Popen(
                [sys.executable, "-c", "import time; time.sleep(60)"]
            )
            Path(os.environ["FAKE_WBT_CHILD_PID"]).write_text(str(child.pid))
            if mode == "closed_output_timeout":
                os.close(sys.stdout.fileno())
                os.close(sys.stderr.fileno())
                time.sleep(60)
            print("child started", flush=True)
            time.sleep(60)
            """
        ),
        encoding="utf-8",
    )
    executable.chmod(0o755)


def process_is_live(pid: int) -> bool:
    stat_path = Path(f"/proc/{pid}/stat")
    if not stat_path.exists():
        return False
    fields = stat_path.read_text(encoding="utf-8").split()
    return len(fields) > 2 and fields[2] != "Z"


class WhiteboxToolsProcessContainmentTests(unittest.TestCase):
    def assert_wrapper_contract(self, module) -> None:
        captured = {}

        class ProbeWhiteboxTools(module.WhiteboxTools):
            def run_tool(self, tool_name, args, callback=None, timeout=None):
                captured["tool_name"] = tool_name
                captured["args"] = list(args)
                captured["callback"] = callback
                captured["timeout"] = timeout
                return 0

        probe = ProbeWhiteboxTools(verbose=False, raise_on_error=False)
        status = probe.topaz_condition_dem(
            dem="dem.tif",
            output="relief.tif",
            max_obstruction_width=2,
            timeout=12.5,
        )

        self.assertEqual(status, 0)
        self.assertEqual(captured["tool_name"], "topaz_condition_dem")
        self.assertIn("--max_obstruction_width='2'", captured["args"])
        self.assertEqual(captured["timeout"], 12.5)

    def assert_nonzero_exit_contract(self, module, executable_dir: Path) -> None:
        messages = []
        with patch.dict(os.environ, {"FAKE_WBT_MODE": "nonzero"}):
            wbt = module.WhiteboxTools(verbose=False, raise_on_error=False)
            wbt.set_whitebox_dir(str(executable_dir))
            status = wbt.run_tool("probe", [], callback=messages.append, timeout=2)

        self.assertEqual(status, 1)
        self.assertTrue(any("exit status 7" in message for message in messages))

    def assert_timeout_kills_process_group(
        self,
        module,
        executable_dir: Path,
        *,
        mode: str,
    ) -> None:
        child_pid_path = executable_dir / "child.pid"
        messages = []
        env = {
            "FAKE_WBT_MODE": mode,
            "FAKE_WBT_CHILD_PID": str(child_pid_path),
        }
        with patch.dict(os.environ, env):
            wbt = module.WhiteboxTools(verbose=False, raise_on_error=False)
            wbt.set_whitebox_dir(str(executable_dir))
            status = wbt.run_tool(
                "probe",
                [],
                callback=messages.append,
                timeout=0.25,
            )

        self.assertEqual(status, 1)
        self.assertTrue(any("timed out" in message for message in messages))
        child_pid = int(child_pid_path.read_text(encoding="utf-8"))
        deadline = time.monotonic() + 2
        while process_is_live(child_pid) and time.monotonic() < deadline:
            time.sleep(0.05)
        self.assertFalse(process_is_live(child_pid))

    def test_both_wrappers_forward_timeout_and_contain_processes(self) -> None:
        original_cwd = Path.cwd()
        try:
            for module_name, module_path in WRAPPER_SPECS:
                with self.subTest(module=module_name):
                    module = load_wrapper_module(module_name, module_path)
                    self.assert_wrapper_contract(module)
                    with tempfile.TemporaryDirectory() as temp_dir:
                        executable_dir = Path(temp_dir)
                        write_fake_whitebox_tools(executable_dir)
                        self.assert_nonzero_exit_contract(module, executable_dir)
                        self.assert_timeout_kills_process_group(
                            module,
                            executable_dir,
                            mode="timeout",
                        )
                        self.assert_timeout_kills_process_group(
                            module,
                            executable_dir,
                            mode="closed_output_timeout",
                        )
        finally:
            os.chdir(original_cwd)


if __name__ == "__main__":
    unittest.main()
