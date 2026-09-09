#!/usr/bin/env python3
"""Reproducible terrain study. Bulk routing/traversal uses the rebuilt owned WBT.

Requires installed numpy, rasterio, matplotlib. All outputs go to a new directory.
"""
import argparse
import csv
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import time

import numpy as np
import rasterio
from rasterio.warp import reproject, Resampling
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

SITES = ['moscow_mountain', 'topanga', 'az_ponderosa']
COEFFICIENTS = [(15, -3.71, .32, .33, .47), (30, -3.79, .21, .19, .36), (60, -3.46, .14, .10, .18)]


def csv_write(path, rows):
    with path.open('w') as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)


def read(path):
    with rasterio.open(path) as ds:
        return ds.read(1), ds.profile


def write(path, data, profile, nodata=-32768):
    profile = dict(profile, dtype=data.dtype, count=1, nodata=nodata)
    with rasterio.open(path, 'w', **profile) as out:
        out.write(data, 1)


class Study:
    def __init__(self, root, binary, output):
        self.root, self.binary, self.output = root, binary, output
        self.commands = []; self.timings = []; self.records = []; self.scenarios = []
        self.grids = {}; self.unavailable = []

    def run(self, tool, **params):
        command = [str(self.binary), f'-r={tool}', '--compress_rasters=false', *[f'--{key}={value}' for key,value in params.items()]]
        self.commands.append(command)
        stamp = len(self.commands)
        memory_path = self.output/f'process-{stamp}.time'
        start = time.perf_counter()
        result = subprocess.run(['/usr/bin/time', '-f', '%e,%M', '-o', str(memory_path), *command],
                                capture_output=True, text=True, env=dict(os.environ, WBT_MAX_PROCS='12'))
        (self.output/f'process-{stamp}.log').write_text(result.stdout+result.stderr)
        if result.returncode:
            raise RuntimeError(f'{tool} failed: {result.stdout}{result.stderr}')
        wall, rss = memory_path.read_text().strip().split(',')
        self.timings.append(dict(tool=tool,wall_seconds=time.perf_counter()-start,
                                 time_wall_seconds=float(wall),peak_rss_kib=int(rss),command_index=stamp))

    def terrain(self, site, kind, resolution, dem, pointer):
        directory = self.output/site/kind/resolution
        directory.mkdir(parents=True, exist_ok=True)
        paths = dict(output=directory/'h.tif', area=directory/'a.tif', coverage=directory/'c.tif')
        self.run('D8UpstreamRelief', dem=dem,d8_pntr=pointer,elevation_units='m',**paths)
        z,profile=read(dem); area,_=read(paths['area']);h,_=read(paths['output']);coverage,_=read(paths['coverage'])
        grid=dict(dem=dem,pointer=pointer,z=z,area=area,h=h,coverage=coverage,profile=profile,directory=directory)
        self.grids[(site,kind,resolution)]=grid
        return grid

    def catchment(self, grid, row, col, label):
        pour=np.zeros(grid['z'].shape,dtype='float32'); pour[row,col]=1
        pourpath=grid['directory']/f'{label}-pour.tif'; maskpath=grid['directory']/f'{label}-basin.tif'
        write(pourpath,pour,grid['profile'],nodata=0)
        self.run('Watershed',d8_pntr=grid['pointer'],pour_pts=pourpath,output=maskpath)
        mask,_=read(maskpath)
        mask=mask==1
        expected=grid['area'][row,col]/abs(grid['profile']['transform'].a*grid['profile']['transform'].e)
        if abs(mask.sum()-expected)>1e-6:
            raise ValueError(f'Catchment cell count disagrees: {mask.sum()} versus {expected}')
        return mask

    def snap(self, grid, x,y):
        transform=grid['profile']['transform']; rr,cc=np.nonzero(grid['area']>=5000)
        xx=transform.c+(cc+.5)*transform.a; yy=transform.f+(rr+.5)*transform.e
        distances=(xx-x)**2+(yy-y)**2
        if distances.size == 0: return None
        i=int(np.argmin(distances))
        if distances[i]>90**2: return None
        return int(rr[i]),int(cc[i]),math.sqrt(float(distances[i]))

    def coordinates(self, grid,row,col):
        return rasterio.transform.xy(grid['profile']['transform'],row,col)

    def compare(self, site, kind, label, ref, other, rc10, rc30, snap_distance):
        r,c=rc10; rr,cc=rc30
        mask10=self.catchment(ref,r,c,f'{label}-10');mask30=self.catchment(other,rr,cc,f'{label}-30')
        # Nearest raster overlay on the fine reference grid for diagnostic overlap.
        projected=np.zeros(mask10.shape,dtype='uint8')
        reproject(mask30.astype('uint8'),projected,src_transform=other['profile']['transform'],
                  src_crs=other['profile']['crs'],dst_transform=ref['profile']['transform'],
                  dst_crs=ref['profile']['crs'],resampling=Resampling.nearest)
        intersection=np.count_nonzero(mask10 & (projected==1))*100
        a10=float(ref['area'][r,c]);a30=float(other['area'][rr,cc])
        overlap=intersection/(a10+a30-intersection)
        h10=float(ref['h'][r,c]);h30=float(other['h'][rr,cc]);t10=h10/math.sqrt(a10);t30=h30/math.sqrt(a30)
        x10,y10=self.coordinates(ref,r,c);x30,y30=self.coordinates(other,rr,cc)
        record=dict(site=site,comparison=kind,outlet=label,row10=r,col10=c,row30=rr,col30=cc,
                    x10=x10,y10=y10,x30=x30,y30=y30,snap_distance_m=snap_distance,
                    actual_center_offset_m=math.hypot(x10-x30,y10-y30),boundary_iou=overlap,
                    area10_m2=a10,area30_m2=a30,area_change_pct=100*(a30/a10-1),
                    cells10=int(mask10.sum()),cells30=int(mask30.sum()),
                    outlet_z10=float(ref['z'][r,c]),outlet_z30=float(other['z'][rr,cc]),
                    maximum_z10=float(ref['z'][mask10].max()),maximum_z30=float(other['z'][mask30].max()),
                    h10_m=h10,h30_m=h30,t10=t10,t30=t30,t_change_pct=100*(t30/t10-1) if t10 else None,
                    coverage10=int(ref['coverage'][r,c]),coverage30=int(other['coverage'][rr,cc]))
        # Independently sampled raster extrema validate the bulk output at every study outlet.
        assert math.isclose(h10,record['maximum_z10']-record['outlet_z10'],rel_tol=1e-9,abs_tol=1e-9)
        assert math.isclose(h30,record['maximum_z30']-record['outlet_z30'],rel_tol=1e-9,abs_tol=1e-9)
        changes=[];threshold_changes=[]
        for duration,b,ct,cf,cs in COEFFICIENTS:
            for f in [.25,.75]:
                for s in [.25,.75]:
                    k10=ct*t10+cf*f+cs*s;k30=ct*t30+cf*f+cs*s
                    for probability in [.25,.5,.75]:
                        rain=(math.log(probability/(1-probability))-b)/k10
                        x=b+rain*k30
                        p30=1/(1+math.exp(-x))
                        i10=rain/(duration/60)
                        i30=(math.log(probability/(1-probability))-b)/k30/(duration/60)
                        change=100*(p30-probability);threshold_pct=100*(i30/i10-1)
                        changes.append(abs(change))
                        if probability in [.5,.75]:threshold_changes.append(abs(threshold_pct))
                        self.scenarios.append(dict(site=site,comparison=kind,outlet=label,duration_min=duration,
                           F=f,S=s,reference_probability=probability,rain_mm=rain,probability30=p30,
                           probability_change_pp=change,threshold10_mm_h=i10,threshold30_mm_h=i30,
                           threshold_change_mm_h=i30-i10,threshold_change_pct=threshold_pct))
        record['max_probability_change_pp']=max(changes);record['max_threshold_change_pct']=max(threshold_changes)
        record['passes_primary_screen']=max(changes)<=5 and max(threshold_changes)<=10
        record['coverage_eligible']=record['coverage10']==0 and record['coverage30']==0
        self.records.append(record)
        print(site,kind,label,'A',round(a10),round(a30),'H',round(h10,3),round(h30,3),
              'p change',round(max(changes),3),'coverage',record['coverage10'],record['coverage30'],flush=True)

    def execute(self):
        for site in SITES:
            native={}
            for resolution in ['10m','30m']:
                source=self.root/site/resolution/'dem'
                native[resolution]=self.terrain(site,'native',resolution,source/'dem.tif',source/'wbt/flovec.tif')
            props={res:json.loads((self.root/site/res/'dem/wbt/outlet.geojson').read_text())['features'][0]['properties'] for res in native}
            main=(props['10m']['row'],props['10m']['column'])
            main30=(props['30m']['row'],props['30m']['column'])
            mask=self.catchment(native['10m'],*main,'selection')
            points=[('terminal',main)]
            mainarea=native['10m']['area'][main]
            for target in [20000,100000,1000000]:
                if target<mainarea/2:
                    delta=np.where(mask,np.abs(native['10m']['area']-target),np.inf)
                    index=np.unravel_index(np.argmin(delta),delta.shape)
                    points.append((f'nested_{target}',tuple(map(int,index))))
            # Prepare controlled raw average and condition at both resolutions.
            controlled={}
            for resolution in ['10m','30m']:
                directory=self.output/site/'controlled'/resolution;directory.mkdir(parents=True)
                profile=native['10m']['profile'];z=native['10m']['z']
                if resolution=='30m':
                    shape=(math.ceil(z.shape[0]/3),math.ceil(z.shape[1]/3))
                    transform=profile['transform']*rasterio.Affine.scale(3)
                    coarse=np.full(shape,profile['nodata'],dtype='float64')
                    reproject(z,coarse,src_transform=profile['transform'],src_crs=profile['crs'],
                              src_nodata=profile['nodata'],dst_transform=transform,dst_crs=profile['crs'],
                              dst_nodata=profile['nodata'],resampling=Resampling.average)
                    raw=directory/'raw.tif';write(raw,coarse,dict(profile,height=shape[0],width=shape[1],transform=transform),nodata=profile['nodata'])
                else:raw=native['10m']['dem']
                conditioned=directory/'conditioned.tif';pointer=directory/'pointer.tif'
                self.run('FillDepressions',dem=raw,output=conditioned,fix_flats='true',flat_increment=.00001)
                self.run('D8Pointer',dem=conditioned,output=pointer)
                controlled[resolution]=self.terrain(site,'controlled',resolution,raw,pointer)
            for label,rc in points:
                x,y=self.coordinates(native['10m'],*rc)
                # Controlled comparison shares the same physical target, with explicit snapping at each resolution.
                match10=self.snap(controlled['10m'],x,y);match30=self.snap(controlled['30m'],x,y)
                if match10 is None or match30 is None:
                    self.unavailable.append(dict(site=site,comparison='controlled',outlet=label,reason='No eligible channel within 90 m'))
                    continue
                cr,cc,dist10=match10
                dr,dc,dist30=match30
                self.compare(site,'controlled',label,controlled['10m'],controlled['30m'],(cr,cc),(dr,dc),dist30)
                self.records[-1]['snap_reference_m']=dist10
            for label,rc in points:
                x,y=self.coordinates(native['10m'],*rc)
                if label=='terminal':
                    other=main30; xx,yy=self.coordinates(native['30m'],*other);distance=math.hypot(x-xx,y-yy)
                else:
                    match=self.snap(native['30m'],x,y)
                    if match is None:
                        self.unavailable.append(dict(site=site,comparison='native',outlet=label,reason='No eligible channel within 90 m'))
                        continue
                    r,c,distance=match;other=(r,c)
                self.compare(site,'native',label,native['10m'],native['30m'],rc,other,distance)
                self.records[-1]['snap_reference_m']=0.
        # Three measured new-output invocations for largest source grid.
        grid=self.grids[('az_ponderosa','native','10m')]
        for iteration in range(3):
            directory=self.output/f'benchmark-{iteration}';directory.mkdir()
            self.run('D8UpstreamRelief',dem=grid['dem'],d8_pntr=grid['pointer'],elevation_units='m',
                     output=directory/'h.tif',area=directory/'a.tif',coverage=directory/'c.tif')
        (self.output/'unavailable.json').write_text(json.dumps(self.unavailable,indent=2))
        csv_write(self.output/'resolution_comparison.csv',self.records)
        csv_write(self.output/'m3_sensitivity.csv',self.scenarios)
        csv_write(self.output/'runtime.csv',self.timings)
        (self.output/'commands.json').write_text(json.dumps(self.commands,indent=2))
        env=dict(binary=str(self.binary),binary_sha256=hashlib.sha256(self.binary.read_bytes()).hexdigest(),
                 platform=platform.platform(),cpu_count=os.cpu_count(),python=platform.python_version(),
                 compress_rasters=False,wbt_max_procs=12,
                 numpy=np.__version__,rasterio=rasterio.__version__,source_fixture_root=str(self.root))
        (self.output/'environment.json').write_text(json.dumps(env,indent=2))
        fig,axes=plt.subplots(1,2,figsize=(13,5),layout='constrained')
        for kind,marker in [('controlled','o'),('native','x')]:
            records=[r for r in self.records if r['comparison']==kind]
            axes[0].scatter([r['area10_m2']/1e6 for r in records],[r['max_probability_change_pp'] for r in records],label=kind,marker=marker)
            axes[1].scatter([r['area10_m2']/1e6 for r in records],[r['max_threshold_change_pct'] for r in records],label=kind,marker=marker)
        for ax,limit,title in zip(axes,[5,10],['Maximum probability change (percentage points)','Maximum inverse-threshold change (%)']):
            ax.axhline(limit,color='red',linestyle='--',label='predeclared screen');ax.set_xscale('log');ax.set_xlabel('10 m upstream area (km²)');ax.set_ylabel(title);ax.legend()
        fig.savefig(self.output/'resolution_sensitivity.png',dpi=160);plt.close(fig)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--fixtures',type=Path,required=True);parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=False)
    Study(args.fixtures.resolve(),args.binary.resolve(),args.output.resolve()).execute()


if __name__=='__main__':main()
