#!/usr/bin/env python3
"""Small independently derived analytical fixtures and real CLI/binding checks.

Usage: python tools/validate_staley_slope_sbs.py --output /tmp/fresh-staley-tests
Requires existing offline rasterio/numpy; no Python terrain implementation.
"""
import argparse
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import subprocess
import struct
import numpy as np
import rasterio
from rasterio.transform import from_origin


def read(path):
    with rasterio.open(path) as ds:
        return ds.read(1), ds.profile


def write(path, data, *, nodata=-9999, transform=None, crs='EPSG:32611', palette=False):
    with rasterio.open(path, 'w', driver='GTiff', height=data.shape[0], width=data.shape[1],
                       count=1, dtype=data.dtype, crs=crs,
                       transform=transform or from_origin(500000, 4000000, 10, 10),
                       nodata=nodata, photometric="palette" if palette else "minisblack") as ds:
        ds.write(data, 1)
        if palette:
            ds.write_colormap(1, {0:(0,0,0,255),1:(0,255,0,255),2:(255,255,0,255),3:(255,0,0,255)})


class Checks:
    def __init__(self, root, binary):
        self.root, self.binary = root, binary
        self.results = []
        self.commands = []

    def invoke(self, name, dem, sbs, mask, *, success=True, **options):
        output = self.root/name
        params=dict(dem=dem,sbs=sbs,mask=mask,output_dir=output,elevation_units='m',sbs_classes='0,1,2,3')
        params.update(options)
        cmd=[str(self.binary),'-r=StaleySlopeSbs','--compress_rasters=false', *[f'--{k}={v}' for k,v in params.items()]]
        source_hashes={str(p):hashlib.sha256(Path(p).read_bytes()).hexdigest() for p in [dem,sbs,mask]}
        self.commands.append(cmd)
        result=subprocess.run(cmd,capture_output=True,text=True,timeout=30)
        (self.root/f'{name}.log').write_text(result.stdout+result.stderr)
        assert (result.returncode==0)==success, (name,result.returncode,result.stdout,result.stderr)
        assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in source_hashes.items())
        if success:
            summary=json.loads((Path(params['output_dir'])/'summary.json').read_text())
            counts=summary['counts']; n=counts['basin']
            assert n==sum(counts[f'intersection_{state}'] for state in ['true','false','unknown'])
            assert counts['sbs_valid']==sum(counts[f'sbs_{state}'] for state in ['unburned','low','moderate','high'])
            assert (summary['T'] is None)==(counts['intersection_unknown']>0)
            support,sp=read(Path(params['output_dir'])/'support.tif')
            _,dp=read(dem)
            assert sp['crs']==dp['crs'] and sp['transform']==dp['transform']
            assert sp['dtype']=='uint8' and sp['nodata']==255
            inside=support!=255
            assert int(inside.sum())==n
            assert int(((support & 1 != 0) & inside).sum())==counts['slope_valid']
            assert int(((support & 2 != 0) & inside).sum())==counts['sbs_valid']
            assert int((support==3).sum())==counts['jointly_valid']
            for product,dtype,nodata in [('slope.tif','float64',-32768),('intersection.tif','uint8',255)]:
                _,profile=read(Path(params['output_dir'])/product)
                assert profile['crs']==dp['crs'] and profile['transform']==dp['transform']
                assert profile['dtype']==dtype and profile['nodata']==nodata
            for key,count in counts.items():
                assert summary['areas_m2'][key]==count*abs(dp['transform'].a*dp['transform'].e)
            for key in ['dem','sbs','mask']:
                data=Path(params[key]).read_bytes();h=0xcbf29ce484222325
                for byte in data: h=((h^byte)*0x100000001b3) & ((1<<64)-1)
                assert summary['sources'][key]['fnv1a64']==f'{h:016x}'
                assert summary['sources'][key]['bytes']==len(data)
            state,_=read(Path(params['output_dir'])/'intersection.tif')
            for value,key in enumerate(['false','true','unknown']):
                assert int((state==value).sum())==counts[f'intersection_{key}']
            assert summary['T_lower']==counts['intersection_true']/n
            assert summary['T_upper']==(counts['intersection_true']+counts['intersection_unknown'])/n
            self.results.append(dict(test=name,passed=True,counts=counts))
            return summary
        assert not output.exists(), name
        self.results.append(dict(test=name,passed=True,rejected=True))


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--binary',type=Path,default=Path('target/release/whitebox_tools'))
    args=parser.parse_args(); root=args.output.resolve();root.mkdir()
    binary=args.binary.resolve();checks=Checks(root,binary)
    mask=np.zeros((7,7),dtype='int16');mask[1:-1,1:-1]=1
    sbs=np.full((7,7),2,dtype='int16')
    mask_path=root/'mask.tif';sbs_path=root/'sbs.tif'
    write(mask_path,mask);write(sbs_path,sbs,nodata=255)
    rr,cc=np.indices((7,7)); records=[]
    for degrees in [0,22.999999,23,23.000001,45]:
        for azimuth in [0,30,45,90,135,230]:
            g=math.tan(math.radians(degrees));az=math.radians(azimuth)
            dem=((cc-3)*math.cos(az)+(rr-3)*math.sin(az))*g*10
            path=root/f'plane-{degrees}-{azimuth}.tif';write(path,dem)
            name=f'plane-{degrees}-{azimuth}'
            summary=checks.invoke(name,path,sbs_path,mask_path)
            slope,_=read(root/name/'slope.tif')
            assert np.max(np.abs(slope[1:-1,1:-1]-degrees))<1e-12
            if degrees!=23:
                assert summary['T']==int(degrees>23)
            records.append(dict(degrees=degrees,azimuth=azimuth,T=summary['T']))
    # Representable threshold construction with unit spacing: exact inclusive boundary.
    threshold=math.tan(math.radians(23));z=np.tile(np.array([-threshold,0,threshold]),(3,1))
    one=np.zeros((3,3),dtype='int16');one[1,1]=1
    for name,data in [('exact-dem',z),('exact-mask',one),('exact-sbs',np.full((3,3),2,dtype='int16'))]:
        write(root/f'{name}.tif',data,transform=from_origin(500000,4000000,1,1))
    assert checks.invoke('exact-threshold',root/'exact-dem.tif',root/'exact-sbs.tif',root/'exact-mask.tif')['T']==1
    steep=root/'steep.tif';flat=root/'flat.tif';write(steep,cc.astype('float64')*10);write(flat,np.zeros((7,7)))
    gap=np.full((7,7),255,dtype='int16');write(root/'missing-sbs.tif',gap,nodata=255)
    assert checks.invoke('unknown-burn',steep,root/'missing-sbs.tif',mask_path)['T'] is None
    assert checks.invoke('known-flat-missing-burn',flat,root/'missing-sbs.tif',mask_path)['T']==0
    holes=cc.astype('float64')*10;holes[3,3]=-9999;write(root/'hole.tif',holes)
    assert checks.invoke('missing-neighbor',root/'hole.tif',sbs_path,mask_path)['counts']['intersection_unknown']==9
    write(root/'unburned.tif',np.zeros((7,7),dtype='int16'),nodata=255)
    assert checks.invoke('known-unburned-missing-slope',root/'hole.tif',root/'unburned.tif',mask_path)['T']==0
    write(root/'full-mask.tif',np.ones((7,7),dtype='int16'))
    assert checks.invoke('raster-edges',steep,sbs_path,root/'full-mask.tif')['counts']['intersection_unknown']==24
    for name,demdata in [('ridge',np.tile(np.array([0,0,10,20,10,0,0.]),(7,1))),('pit',np.zeros((7,7)))]:
        if name=='pit': demdata[3,3]=-50
        write(root/f'{name}.tif',demdata)
        checks.invoke(name,root/f'{name}.tif',sbs_path,mask_path)
        slope,_=read(root/name/'slope.tif');assert slope[3,3]==0
    # Same marginal steep and burned fractions, distinct spatial overlap.
    dem=np.tile(np.array([0,0,0,0,10,20,30.]),(7,1));write(root/'overlap-dem.tif',dem)
    two=np.zeros((7,7),dtype='int16');two[3,1]=two[3,5]=1;write(root/'two-mask.tif',two)
    for name,col,want in [('overlap',5,.5),('disjoint',1,0)]:
        b=np.zeros((7,7),dtype='int16');b[3,col]=2;write(root/f'{name}-sbs.tif',b,nodata=255)
        summary=checks.invoke(name,root/'overlap-dem.tif',root/f'{name}-sbs.tif',root/'two-mask.tif')
        assert summary['counts']['steep']==summary['counts']['sbs_moderate']==1 and summary['T']==want
    invalids=[('invalid-class',np.full((7,7),4,dtype='int16'),{}),
              ('nonfinite-sbs',np.full((7,7),np.inf),{}),
              ('misaligned',sbs,{'transform':from_origin(500001,4000000,10,10)}),
              ('wrong-crs',sbs,{'crs':'EPSG:4326'}),
              ('no-nodata',sbs,{'nodata':None}),
              ('no-sampleformat',sbs.astype('uint8'),{'nodata':255}),
              ('palette',sbs.astype('uint8'),{'nodata':255,'palette':True})]
    for name,data,kwargs in invalids:
        write(root/f'{name}.tif',data,**kwargs)
        if name=='no-sampleformat':
            raw=bytearray((root/f'{name}.tif').read_bytes()); endian='<' if raw[:2]==b'II' else '>'
            offset=struct.unpack_from(endian+'I',raw,4)[0];count=struct.unpack_from(endian+'H',raw,offset)[0]
            for i in range(count):
                pos=offset+2+i*12
                if struct.unpack_from(endian+'H',raw,pos)[0]==339:
                    struct.pack_into(endian+'H',raw,pos,65000)
            (root/f'{name}.tif').write_bytes(raw)
        checks.invoke(name,steep,root/f'{name}.tif',mask_path,success=False)
    write(root/'empty.tif',np.zeros((7,7),dtype='int16'))
    checks.invoke('empty-mask',steep,sbs_path,root/'empty.tif',success=False)
    checks.invoke('wrong-units',steep,sbs_path,mask_path,success=False,elevation_units='ft')
    checks.invoke('duplicate-codes',steep,sbs_path,mask_path,success=False,sbs_classes='0,1,2,2')
    checks.invoke('nodata-collision',steep,sbs_path,mask_path,success=False,sbs_classes='0,1,2,255')
    checks.invoke('no-parent',steep,sbs_path,mask_path,success=False,output_dir=root/'missing'/'output')
    sentinel=root/'preserved';sentinel.mkdir();(sentinel/'keep').write_bytes(b'keep')
    checks.invoke('existing-output',steep,sbs_path,mask_path,success=False,output_dir=sentinel)
    checks.invoke('input-as-output',steep,sbs_path,mask_path,success=False,output_dir=steep)
    assert (sentinel/'keep').read_bytes()==b'keep'
    link=root/'symlink';link.symlink_to(sentinel,target_is_directory=True)
    checks.invoke('symlink-output',steep,sbs_path,mask_path,success=False,output_dir=link)
    # Independently derived Esri published complete/gap stencil examples.
    # One missing corner leaves seven neighbors: Esri reweights, strict Horn is unknown.
    z=np.arange(1,10,dtype='float64').reshape(3,3)
    for name,data in [('esri-full',z.copy()),('esri-gap',z.copy())]:
        if name=='esri-gap': data[0,2]=-9999
        write(root/f'{name}.tif',data,transform=from_origin(500000,4000000,1,1))
        result=checks.invoke(name,root/f'{name}.tif',root/'exact-sbs.tif',root/'exact-mask.tif')
        slope,_=read(root/name/'slope.tif')
        if name=='esri-full': assert abs(slope[1,1]-math.degrees(math.atan(math.sqrt(10))))<1e-12
        else: assert slope[1,1]==-32768 and result['T'] is None
    esri_gap_degrees=math.degrees(math.atan(math.hypot((28-16)/8,(32-20/3)/8)))
    assert 74 < esri_gap_degrees < 75
    # Routed westward drop captures only one component of a 45-degree surface.
    write(root/'west-pointer.tif',np.full((7,7),32,dtype='int16'))
    command=[str(binary),'-r=FVSlope',f'--dem={root/"plane-45-45.tif"}',
             f'--d8_pntr={root/"west-pointer.tif"}',f'--output={root/"directional.tif"}',
             '--units=degrees','--zfactor=1']
    result=subprocess.run(command,capture_output=True,text=True,check=True,timeout=30)
    directional,_=read(root/'directional.tif')
    assert abs(directional[3,3]-math.degrees(math.atan(1/math.sqrt(2))))<1e-5
    checks.commands.append(command)
    checks.results.append(dict(test='surface-versus-directional',passed=True,
                               surface_degrees=45,directional_degrees=float(directional[3,3]),
                               esri_gap_formula_degrees=esri_gap_degrees))
    # Both actual bindings must execute the exact rebuilt binary, no installed copy.
    repo=Path(__file__).resolve().parents[1]
    for i,path in enumerate([repo/'whitebox_tools.py',repo/'WBT/whitebox_tools.py']):
        spec=importlib.util.spec_from_file_location(f'wbt_binding_{i}',path)
        mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
        output=root/f'binding-{i}'
        previous=Path.cwd()
        try:
            wbt=mod.WhiteboxTools(verbose=False, raise_on_error=True);wbt.set_whitebox_dir(str(binary.parent));wbt.set_verbose_mode(False)
            rc=wbt.staley_slope_sbs(str(steep),str(sbs_path),str(mask_path),str(output),'m','0,1,2,3')
        finally:
            os.chdir(previous)
        assert rc==0 and json.loads((output/'summary.json').read_text())['T']==1
        checks.results.append(dict(test=f'binding-{i}',passed=True,source=str(path)))
    result=dict(binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                tests=checks.results,threshold_boundary_cases=records,commands=checks.commands)
    (root/'validation.json').write_text(json.dumps(result,indent=2)+'\n')
    print(f'{len(checks.results)} CLI/binding checks passed; evidence {root}/validation.json')

if __name__=='__main__':
    main()
