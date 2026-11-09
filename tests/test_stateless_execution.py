"""Real concurrent CLI/binding/plugin output checks for stateless execution."""
from concurrent.futures import ThreadPoolExecutor
import importlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

import numpy as np
import rasterio
from rasterio.transform import from_origin

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('WBT_STATELESS_BINARY', ROOT / 'target/release/whitebox_tools'))


class StatelessExecutionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='wbt-stateless-')
        self.root = Path(self.temp.name)
        self.install = self.root / 'install'
        self.install.mkdir()
        self.binary = self.install / 'whitebox_tools'
        shutil.copy2(BINARY, self.binary)
        self.settings = self.install / 'settings.json'
        self.defaults = dict(verbose_mode=False, working_directory='', compress_rasters=False, max_procs=1)
        self.settings.write_text(json.dumps(self.defaults))
        self.settings.chmod(0o640)
        self.original = self.settings.read_bytes()

    def tearDown(self):
        self.install.chmod(0o700)
        self.temp.cleanup()

    def fixture(self, number):
        work = self.root / f'run-{number}'
        work.mkdir()
        values = np.full((5, 5), number + 1, dtype=np.float32)
        with rasterio.open(work / 'input.tif', 'w', driver='GTiff', width=5, height=5,
                           count=1, dtype='float32', crs='EPSG:32611',
                           transform=from_origin(500000, 4100000, 10, 10), nodata=-9999) as out:
            out.write(values, 1)
        return work, values

    def cli(self, work, *args):
        return subprocess.run([str(self.binary), f'--wd={work}', '-v=false', *args],
                              cwd=self.root, capture_output=True, text=True, timeout=30)

    def check_output(self, work, expected, compressed):
        with rasterio.open(work / 'output.tif') as result:
            np.testing.assert_array_equal(result.read(1), expected)
            self.assertEqual(result.crs.to_epsg(), 32611)
            self.assertEqual(result.transform, from_origin(500000, 4100000, 10, 10))
            self.assertEqual(result.compression is not None, compressed)

    def test_parallel_cli_does_not_write_readonly_installation(self):
        cases = [self.fixture(i) for i in range(8)]
        self.settings.chmod(0o440)
        self.install.chmod(0o550)
        def run(item):
            i, (work, values) = item
            result = self.cli(work, '--run=ConvertNodataToZero', '--input=input.tif',
                              '--output=output.tif', f'--compress_rasters={str(bool(i % 2)).lower()}')
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.check_output(work, values, bool(i % 2))
        with ThreadPoolExecutor(max_workers=4) as pool:
            list(pool.map(run, enumerate(cases)))
        self.assertEqual(self.settings.read_bytes(), self.original)
        self.assertEqual(set(p.name for p in self.install.iterdir()), {'whitebox_tools', 'settings.json'})

    def test_absent_settings_stays_absent_and_corrupt_settings_fails(self):
        self.settings.unlink()
        self.install.chmod(0o550)
        work, values = self.fixture(1)
        result = self.cli(work, '--run=ConvertNodataToZero', '--input=input.tif',
                          '--output=output.tif', '--compress_rasters=false')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.check_output(work, values, False)
        self.assertFalse(self.settings.exists())
        self.install.chmod(0o700)
        for i, malformed in enumerate(('', '{', '{}', '[]')):
            with self.subTest(settings=malformed):
                self.settings.write_text(malformed)
                bad, _ = self.fixture(i + 2)
                result = self.cli(bad, '--run=ConvertNodataToZero', '--input=input.tif', '--output=output.tif')
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((bad / 'output.tif').exists())
                self.assertEqual(self.settings.read_text(), malformed)

    def test_all_wrappers_keep_options_and_cwd_local(self):
        modules = []
        for index, source in enumerate([ROOT / 'whitebox_tools.py', ROOT / 'WBT/whitebox_tools.py']):
            dest = self.install / f'wrapper{index}.py'
            shutil.copy2(source, dest)
            spec = importlib.util.spec_from_file_location(f'private_wbt{index}', dest)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            modules.append(module)
        sys.path.insert(0, str(ROOT / 'python'))
        try:
            modules.append(importlib.import_module('weppcloud_wbt.whitebox_tools'))
        finally:
            sys.path.pop(0)
        before_cwd = os.getcwd()
        with patch.dict(os.environ, {'WHITEBOX_TOOLS_EXE': str(self.binary)}):
            cases = []
            for i in range(12):
                tool = modules[i % 3].WhiteboxTools()
                self.assertIs(tool.verbose, False)
                work, values = self.fixture(i)
                tool.set_whitebox_dir(str(self.install))
                tool.set_working_dir(str(work))
                tool.set_verbose_mode(False)
                tool.set_compress_rasters(bool(i % 2))
                tool.set_max_procs(1 + i % 2)
                cases.append((tool, work, values, bool(i % 2)))
            self.assertEqual(self.settings.read_bytes(), self.original)
            def run(case):
                tool, work, values, compressed = case
                self.assertEqual(tool.run_tool('ConvertNodataToZero',
                    ['--input=input.tif', '--output=output.tif']), 0)
                self.assertEqual(os.getcwd(), before_cwd)
                self.check_output(work, values, compressed)
            with ThreadPoolExecutor(max_workers=4) as pool:
                list(pool.map(run, cases))
        self.assertEqual(os.getcwd(), before_cwd)
        self.assertEqual(self.settings.read_bytes(), self.original)

    def test_explicit_configuration_creation_umask_and_write_failure(self):
        self.settings.unlink()
        result = subprocess.run([str(self.binary), '--max_procs=2'],
                                capture_output=True, timeout=30, umask=0o027)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.settings.stat().st_mode & 0o777, 0o640)
        if os.geteuid() != 0:
            before = self.settings.read_bytes()
            self.install.chmod(0o550)
            result = subprocess.run([str(self.binary), '--max_procs=1'],
                                    capture_output=True, timeout=30)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(self.settings.read_bytes(), before)

    def test_explicit_configuration_is_atomic_and_preserves_mode(self):
        done = threading.Event()
        errors = []
        def reader():
            while not done.is_set():
                try:
                    json.loads(self.settings.read_text())
                except (ValueError, OSError) as error:
                    errors.append(error)
                    return
        thread = threading.Thread(target=reader)
        thread.start()
        try:
            for i in range(30):
                result = subprocess.run([str(self.binary), f'--max_procs={1+i%2}'],
                                        capture_output=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stderr)
        finally:
            done.set()
            thread.join(5)
        self.assertFalse(errors)
        self.assertEqual(self.settings.stat().st_mode & 0o777, 0o640)
        self.assertEqual(json.loads(self.settings.read_text())['max_procs'], 2)
        self.assertFalse(list(self.install.glob('.settings.*.tmp')))

    @unittest.skipUnless(sys.platform == 'linux', 'Linux ACL/xattr publication contract')
    def test_explicit_configuration_preserves_link_owner_and_acl(self):
        import struct
        target = self.install / 'defaults.json'
        self.settings.rename(target)
        self.settings.symlink_to(target.name)
        os.setxattr(target, 'user.wbt-test', b'preserve-this')
        acl = struct.pack('<I', 2) + b''.join(struct.pack('<HHI', *entry) for entry in [
            (1, 6, 0xffffffff), (2, 4, 12345), (4, 4, 0xffffffff),
            (16, 4, 0xffffffff), (32, 0, 0xffffffff)])
        os.setxattr(target, 'system.posix_acl_access', acl)
        if os.geteuid() == 0:
            os.chown(target, 1000, 4)  # Exercise non-creator ownership/group too.
        before = target.stat()
        attributes = {name: os.getxattr(target, name) for name in os.listxattr(target)}
        result = subprocess.run([str(self.binary), '--max_procs=2'], capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.settings.is_symlink())
        self.assertEqual(json.loads(target.read_text())['max_procs'], 2)
        after = target.stat()
        self.assertEqual((after.st_uid, after.st_gid, after.st_mode),
                         (before.st_uid, before.st_gid, before.st_mode))
        self.assertEqual({name: os.getxattr(target, name) for name in os.listxattr(target)}, attributes)

    def test_bundled_plugin_receives_invocation_configuration(self):
        plugins = self.install / 'plugins'
        plugins.mkdir()
        shutil.copy2(ROOT / 'target/release/max_upslope_value', plugins / 'max_upslope_value')
        shutil.copy2(ROOT / 'WBT/plugins/max_upslope_value.json', plugins / 'max_upslope_value.json')
        cases = [self.fixture(i) for i in range(4)]
        def run(item):
            i, (work, values) = item
            result = self.cli(work, '--run=MaxUpslopeValue', '--dem=input.tif', '--values=input.tif',
                              '--output=output.tif', f'--compress_rasters={str(bool(i % 2)).lower()}')
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.check_output(work, values, bool(i % 2))
        with ThreadPoolExecutor(max_workers=4) as pool:
            list(pool.map(run, enumerate(cases)))
        self.assertEqual(self.settings.read_bytes(), self.original)
        bad, _ = self.fixture(99)
        result = self.cli(bad, '--run=MaxUpslopeValue', '--dem=missing.tif',
                          '--values=input.tif', '--output=output.tif')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((bad / 'output.tif').exists())


if __name__ == '__main__':
    unittest.main()
