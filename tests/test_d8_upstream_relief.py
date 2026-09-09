"""Generated-output contract tests; set WBT_TERRAIN_BINARY to a rebuilt binary."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import numpy as np
import rasterio
from rasterio.transform import from_origin

ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('WBT_TERRAIN_BINARY', ROOT / 'target/release/whitebox_tools')).resolve()


class TerrainCommandTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='wbt terrain ')
        self.directory = Path(self.temp.name)
        self.z = np.full((5, 7), -9999., dtype='float64')
        self.p = self.z.copy()
        self.z[2, 2:5] = [130, 120, 100]
        self.p[2, 2:5] = [2, 2, 0]
        self.write('dem.tif', self.z)
        self.write('pointer.tif', self.p)

    def tearDown(self):
        self.temp.cleanup()

    def write(self, name, values, **kwargs):
        options = dict(driver='GTiff', height=values.shape[0], width=values.shape[1],
                       count=1, dtype=values.dtype, crs='EPSG:32611',
                       transform=from_origin(500000, 5200000, 10, 10), nodata=-9999)
        options.update(kwargs)
        with rasterio.open(self.directory / name, 'w', **options) as out:
            out.write(values, 1)

    def arguments(self):
        return dict(dem=str(self.directory/'dem.tif'), d8_pntr=str(self.directory/'pointer.tif'),
                    output=str(self.directory/'h.tif'), area=str(self.directory/'a.tif'),
                    coverage=str(self.directory/'c.tif'), elevation_units='m')

    def run_cli(self, **changes):
        args = self.arguments(); args.update(changes)
        return subprocess.run([str(BINARY), '-r=D8UpstreamRelief',
                               *[f'--{k}={v}' for k, v in args.items()]],
                              capture_output=True, text=True, timeout=30)

    def assert_outputs(self, expected=(0, 10, 30)):
        for name, values in [('h.tif', expected), ('a.tif', [100, 200, 300]), ('c.tif', [1, 1, 1])]:
            with rasterio.open(self.directory/name) as out, rasterio.open(self.directory/'dem.tif') as dem:
                np.testing.assert_allclose(out.read(1)[2, 2:5], values, rtol=1e-9, atol=1e-9)
                np.testing.assert_array_equal(out.dataset_mask(), dem.dataset_mask())
                self.assertEqual(out.crs, dem.crs); self.assertEqual(out.transform, dem.transform)
                if name != 'c.tif': self.assertEqual(out.dtypes, ('float64',))

    def test_cli_chain_and_output_alias_rejection(self):
        result = self.run_cli(); self.assertEqual(result.returncode, 0, result.stdout+result.stderr)
        self.assert_outputs()
        before = (self.directory/'dem.tif').read_bytes()
        self.assertNotEqual(self.run_cli(output=str(self.directory/'dem.tif')).returncode, 0)
        self.assertEqual(before, (self.directory/'dem.tif').read_bytes())
        self.assertNotEqual(self.run_cli().returncode, 0)

    def test_both_bindings_execute_current_binary(self):
        for index, path in enumerate([ROOT/'whitebox_tools.py', ROOT/'WBT/whitebox_tools.py']):
            with self.subTest(binding=str(path)):
                spec = importlib.util.spec_from_file_location(f'terrain_binding_{index}', path)
                module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
                original = Path.cwd()
                tool = module.WhiteboxTools(verbose=False, raise_on_error=True); tool.set_whitebox_dir(str(BINARY.parent))
                try:
                    result = tool.d8_upstream_relief(**self.arguments(), callback=lambda _: None)
                finally:
                    os.chdir(original)
                self.assertEqual(result, 0); self.assert_outputs()
                for name in ['h.tif', 'a.tif', 'c.tif']: (self.directory/name).unlink()

    def test_conditioned_pit_and_internal_maximum(self):
        for heights, expected in [([130, 90, 100], [0, 40, 30]), ([110, 130, 100], [0, 0, 30])]:
            with self.subTest(heights=heights):
                self.z[2,2:5] = heights; self.write('dem.tif', self.z)
                result = self.run_cli(); self.assertEqual(result.returncode, 0, result.stderr)
                self.assert_outputs(expected)
                for name in ['h.tif', 'a.tif', 'c.tif']: (self.directory/name).unlink()

    def test_invalid_pointer_cycle_and_mask(self):
        for code in [3, 1.5, -1, np.inf, np.nan, 32, -9999]:
            with self.subTest(code=code):
                self.p[2,3] = code; self.write('pointer.tif', self.p)
                self.assertNotEqual(self.run_cli().returncode, 0)
                self.assertFalse((self.directory/'h.tif').exists())

    def test_grid_and_units_rejected(self):
        for options in [dict(crs='EPSG:4326'), dict(transform=from_origin(500001,5200000,10,10)),
                        dict(transform=rasterio.Affine(10,1,500000,0,-10,5200000))]:
            with self.subTest(options=options):
                self.write('pointer.tif', self.p, **options)
                self.assertNotEqual(self.run_cli().returncode, 0)
        self.write('pointer.tif', self.p)
        self.assertNotEqual(self.run_cli(elevation_units='ft').returncode, 0)

    def test_nodata_outgoing_pointer_does_not_contaminate_relief(self):
        self.p[2,4] = 2; self.write('pointer.tif', self.p)
        self.assertEqual(self.run_cli().returncode, 0); self.assert_outputs()

    def test_single_row_outputs_compressed_and_uncompressed(self):
        self.write('dem.tif', np.array([[130.,120.,100.]]))
        self.write('pointer.tif', np.array([[2.,2.,0.]]))
        for compress in ['false','true']:
            command=[str(BINARY), '-r=D8UpstreamRelief', f'--compress_rasters={compress}',
                     *[f'--{k}={v}' for k,v in self.arguments().items()]]
            result=subprocess.run(command,capture_output=True,text=True,timeout=30)
            self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            for name, expected in [('h.tif',[[0,10,30]]),('a.tif',[[100,200,300]]),('c.tif',[[1,1,1]])]:
                with rasterio.open(self.directory/name) as out:
                    np.testing.assert_array_equal(out.read(1),expected)
                    description=out.tags()['TIFFTAG_IMAGEDESCRIPTION']
                    self.assertIn('maximum upstream raw elevation',description)
                    self.assertIn('flagged cells=',description)
                (self.directory/name).unlink()

    def test_nan_nodata_and_bigtiff_roundtrip(self):
        for name, data in [('dem.tif', self.z.copy()), ('pointer.tif', self.p.copy())]:
            data[data == -9999] = np.nan
            self.write(name,data,nodata=np.nan,BIGTIFF='YES')
        result=self.run_cli();self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assert_outputs()

    def test_original_zero_pixel_scale_rejected(self):
        # Independent fixture mutation: standard little-endian TIFF double tag.
        import struct
        for name in ['dem.tif','pointer.tif']:
            path=self.directory/name
            data=bytearray(path.read_bytes())
            self.assertEqual(data[:2],b'II')
            ifd=struct.unpack_from('<I',data,4)[0]
            count=struct.unpack_from('<H',data,ifd)[0]
            for index in range(count):
                entry=ifd+2+12*index
                tag,kind,n,offset=struct.unpack_from('<HHII',data,entry)
                if tag==33550:
                    self.assertEqual((kind,n),(12,3))
                    struct.pack_into('<d',data,offset,0.)
                    break
            else:self.fail('Fixture has no pixel scale')
            path.write_bytes(data)
        result=self.run_cli();self.assertNotEqual(result.returncode,0)
        self.assertFalse((self.directory/'h.tif').exists())
        # Legacy tools retain their assumed-spacing behavior and output georeferencing.
        legacy = self.directory/'legacy.tif'
        result=subprocess.run([str(BINARY), '-r=D8Pointer', f'--dem={self.directory}/dem.tif',
                               f'--output={legacy}'],capture_output=True,text=True,timeout=30)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        with rasterio.open(legacy) as out:
            self.assertEqual(out.crs,rasterio.crs.CRS.from_epsg(32611))
            self.assertEqual(out.transform,from_origin(500000,5200000,1,1))

    def test_point_pixels_and_multisample_inputs_rejected(self):
        with rasterio.open(self.directory/'dem.tif','r+') as ds:
            ds.update_tags(AREA_OR_POINT='Point')
        self.assertNotEqual(self.run_cli().returncode,0)
        self.write('dem.tif',self.z,count=2)
        self.assertNotEqual(self.run_cli().returncode,0)

    def test_duplicate_output_symlink_and_missing_value(self):
        self.assertNotEqual(self.run_cli(area=str(self.directory/'h.tif')).returncode, 0)
        (self.directory/'link.tif').symlink_to(self.directory/'dem.tif')
        self.assertNotEqual(self.run_cli(output=str(self.directory/'link.tif')).returncode, 0)
        result = subprocess.run([str(BINARY), '-r=D8UpstreamRelief', '--dem'], capture_output=True, timeout=30)
        self.assertNotEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
