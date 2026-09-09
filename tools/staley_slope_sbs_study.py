#!/usr/bin/env python3
"""Six existing main watersheds: Rust terrain, GDAL Horn reference, tabular sensitivity.

Synthetic SBS checker classes isolate methods; never interpret dNBR as SBS.
Sources are immutable. Run from WBT with --output pointing to a fresh directory.
"""
import argparse
import csv
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time
import numpy as np
import rasterio


def read(path):
    with rasterio.open(path) as ds:
        return ds.read(1), ds.profile


def write(path,data,profile):
    with rasterio.open(path,'w',**dict(profile,dtype=data.dtype,nodata=255,compress='none',photometric='minisblack')) as ds:
        ds.write(data,1)


def csvwrite(path,records):
    with path.open('w') as out:
        writer=csv.DictWriter(out,fieldnames=list(records[0]));writer.writeheader();writer.writerows(records)



def plot_panel(output, fixtures):
    """Inspect generated slopes and classifications; no terrain differentiation."""
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    from matplotlib.colors import ListedColormap
    from matplotlib.patches import Patch
    fig,axes=plt.subplots(2,3,figsize=(12,7),layout='constrained')
    colors=['#eeeeee','#0072b2','#d55e00','#333333']
    for row,res in enumerate([10,30]):
        root=output/'topanga'/f'{res}m'/'raw'
        b,profile=read(fixtures/'topanga'/f'{res}m'/'dem/wbt/bound.tif')
        mask=(b>0)&(b!=profile['nodata']);values=[]
        for col,(name,path) in enumerate([('Horn',root/'horn/slope.tif'),('FVSlope',root/'fvslope.tif')]):
            z,profile=read(path);valid=(z!=profile['nodata'])&mask;values.append((z,valid))
            im=axes[row,col].imshow(np.where(valid,z,np.nan),vmin=0,vmax=60,cmap='terrain')
            axes[row,col].set_title(f'Topanga {res} m: {name}',fontsize=10)
        h,hv=values[0];v,vv=values[1];common=hv&vv
        classes=np.where(common,(h>=23).astype(int)+2*(v>=23).astype(int),np.nan)
        axes[row,2].imshow(classes,cmap=ListedColormap(colors),vmin=0,vmax=3)
        axes[row,2].set_title('23° classification',fontsize=10)
        for ax in axes[row]:ax.set_xticks([]);ax.set_yticks([])
    fig.colorbar(im,ax=axes[:,:2],shrink=.65,label='Slope (degrees)')
    fig.legend(handles=[Patch(color=c,label=label) for c,label in zip(colors,['Both below','Horn only steep','FVSlope only steep','Both steep'])],
               loc='lower center',bbox_to_anchor=(.5,-.04),ncol=4)
    fig.suptitle('Existing Topanga main watersheds: generated terrain products\nNative extents differ; panels are not cell-aligned. No observed outcome validation.',fontsize=11)
    fig.savefig(output/'topanga_method_panel.png',dpi=150,bbox_inches='tight');plt.close(fig)


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--binary',type=Path,default=Path('target/release/whitebox_tools'))
    parser.add_argument('--fixtures',type=Path,default=Path('test_fixtures/staley_m3_resolution'))
    parser.add_argument('--wepppy',type=Path,default=Path('/workdir/wepppy'))
    args=parser.parse_args();args.fixtures=args.fixtures.resolve();out=args.output.resolve();out.mkdir();binary=args.binary.resolve()
    repo=Path(__file__).resolve().parents[1]
    module_path=args.wepppy/'wepppy/nodb/mods/postfire_debris_flow/staley2017.py'
    spec=importlib.util.spec_from_file_location('staley_scalar_study',module_path)
    engine=importlib.util.module_from_spec(spec);sys.modules[spec.name]=engine;spec.loader.exec_module(engine)
    calls=[];timings=[];records=[];comparisons=[];conditioned=[];source_hashes={};grids={}
    def run(command,label):
        calls.append(command); start=time.perf_counter();stamp=len(calls)
        metric=out/f'process-{stamp}.time'
        result=subprocess.run(['/usr/bin/time','-f','%e,%M','-o',str(metric),*command],capture_output=True,text=True,
                              timeout=180,env=dict(os.environ,WBT_MAX_PROCS='8'))
        (out/f'process-{stamp}.log').write_text(result.stdout+result.stderr)
        if result.returncode: raise RuntimeError(f'{label}: {result.stdout}{result.stderr}')
        wall,rss=metric.read_text().strip().split(',')
        timings.append(dict(label=label,wall_seconds=time.perf_counter()-start,peak_rss_kib=int(rss),command_index=stamp))
    def wbt(tool,label,**params):
        run([str(binary),f'-r={tool}','--compress_rasters=false',*[f'--{k}={v}' for k,v in params.items()]],label)
    def summarize(site,res,kind,method,slope,valid,mask,sbs,area):
        steep=valid & (slope>=23);burned=sbs>=2
        true=mask & steep & burned;unknown=mask & ~valid & burned
        n=int(mask.sum());y=int(true.sum());u=int(unknown.sum());T=y/n if u==0 else None
        probability=None if T is None else engine.probability('M1',15,T=T,F=.5,S=.25,rainfall_mm=20)
        row=dict(site=site,resolution_m=res,terrain=kind,method=method,basin_cells=n,basin_area_m2=n*area,
                 slope_valid=int((valid & mask).sum()),steep=int((steep & mask).sum()),
                 intersection_true=y,intersection_unknown=u,T=T,T_lower=y/n,T_upper=(y+u)/n,
                 probability_F05_S025_R20mm_D15min=probability)
        records.append(row);return row
    for site in ['moscow_mountain','topanga','az_ponderosa']:
        for res in [10,30]:
            root=args.fixtures.resolve()/site/f'{res}m';directory=out/site/f'{res}m';directory.mkdir(parents=True)
            raw=root/'dem/dem.tif';relief=root/'dem/wbt/relief.tif';mask_path=root/'dem/wbt/bound.tif';pointer=root/'dem/wbt/flovec.tif';outlet=root/'dem/wbt/outlet.geojson'
            for p in [raw,relief,mask_path,pointer,outlet]:source_hashes[str(p)]=hashlib.sha256(p.read_bytes()).hexdigest()
            z,profile=read(raw);b,bp=read(mask_path);mask=(b>0)&(b!=bp['nodata'])
            rr,cc=np.indices(z.shape);tr=profile['transform']
            x=tr.c+(cc+.5)*tr.a;y=tr.f+(rr+.5)*tr.e
            sbs=((np.floor(x/210)+np.floor(y/330))%4).astype('int16')
            sbs_path=directory/'synthetic-sbs.tif';write(sbs_path,sbs,profile)
            source_hashes[str(sbs_path)]=hashlib.sha256(sbs_path.read_bytes()).hexdigest()
            methods={};baselines={}
            for kind,dem in [('raw',raw),('conditioned',relief)]:
                d=directory/kind;d.mkdir();horn=d/'horn'
                wbt('StaleySlopeSbs',f'{site}-{res}-{kind}-Horn',dem=dem,sbs=sbs_path,mask=mask_path,output_dir=horn,elevation_units='m',sbs_classes='0,1,2,3')
                # Existing routed pointer held fixed to isolate estimator/conditioning.
                wbt('Slope',f'{site}-{res}-{kind}-Florinsky',dem=dem,output=d/'florinsky.tif',units='degrees',zfactor=1)
                wbt('FVSlope',f'{site}-{res}-{kind}-FVSlope',dem=dem,d8_pntr=pointer,output=d/'fvslope.tif',units='degrees',zfactor=1)
                run(['gdaldem','slope',str(dem),str(d/'gdal-horn.tif'),'-alg','Horn','-s','1'],f'{site}-{res}-{kind}-GDAL')
                for method,path in [('Horn',horn/'slope.tif'),('Florinsky',d/'florinsky.tif'),('FVSlope',d/'fvslope.tif'),('GDAL_Horn',d/'gdal-horn.tif')]:
                    slope,pr=read(path);valid=np.isfinite(slope)&(slope!=pr['nodata'])
                    record=summarize(site,res,kind,method,slope,valid,mask,sbs,res*res)
                    methods[kind,method]=(slope,valid,record)
                # Actual intersection is authoritative at f64 boundary, not rounded slope.
                summary=json.loads((horn/'summary.json').read_text());baseline=methods[kind,'Horn'][2]
                assert baseline['T']==summary['T'] and baseline['intersection_true']==summary['counts']['intersection_true']
                baselines[kind]=summary
                hs,hv,hr=methods[kind,'Horn']
                for other in ['Florinsky','FVSlope','GDAL_Horn']:
                    ss,sv,sr=methods[kind,other];common=mask&hv&sv
                    crossing=int((((hs>=23)!=(ss>=23))&common).sum())
                    record=dict(site=site,resolution_m=res,terrain=kind,comparison=f'{other}-minus-Horn',common_basin_cells=int(common.sum()),
                                threshold_crossings=crossing,other_only_steep=int(((ss>=23)&(hs<23)&common).sum()),
                                horn_only_steep=int(((hs>=23)&(ss<23)&common).sum()),
                                max_abs_degrees=float(np.max(np.abs(hs[common]-ss[common]))),
                                T_delta=None if sr['T'] is None or hr['T'] is None else sr['T']-hr['T'],
                                probability_delta=None if sr['T'] is None or hr['T'] is None else sr['probability_F05_S025_R20mm_D15min']-hr['probability_F05_S025_R20mm_D15min'])
                    comparisons.append(record)
                    if other=='GDAL_Horn':
                        # GDAL 3.10 DEMProcessing uses float32 working values even
                        # for Float64 sources; bound cancellation and output rounding.
                        finite=z[np.isfinite(z)&(z!=profile['nodata'])]
                        bound=16*np.finfo('float32').eps*max(1,float(np.abs(finite).max()))/res*180/math.pi+1e-5
                        assert record['max_abs_degrees']<bound, (record,bound)
            hraw,vraw,rraw=methods['raw','Horn'];hc,vc,rc=methods['conditioned','Horn'];common=mask&vraw&vc
            conditioned.append(dict(site=site,resolution_m=res,common_cells=int(common.sum()),threshold_crossings=int((((hraw>=23)!=(hc>=23))&common).sum()),
                                    T_raw=rraw['T'],T_conditioned=rc['T'],T_delta=None if rraw['T'] is None or rc['T'] is None else rc['T']-rraw['T']))
            grids[site,res]=rraw
            # Both bindings also run one real main-watershed fixture.
            if site=='topanga' and res==10:
                for i,wrapper in enumerate([repo/'whitebox_tools.py',repo/'WBT/whitebox_tools.py']):
                    ws=importlib.util.spec_from_file_location(f'wbt_real_{i}',wrapper.resolve());wm=importlib.util.module_from_spec(ws);ws.loader.exec_module(wm)
                    dest=directory/f'binding-{i}'
                    previous=Path.cwd()
                    try:
                        w=wm.WhiteboxTools(verbose=False, raise_on_error=True);w.set_whitebox_dir(str(binary.parent));w.set_verbose_mode(False)
                        assert w.staley_slope_sbs(str(raw),str(sbs_path),str(mask_path),str(dest),'m','0,1,2,3')==0
                    finally:
                        os.chdir(previous)
                    got=json.loads((dest/'summary.json').read_text());assert got==baselines['raw']
            print(f'{site} {res}m complete',flush=True)
    native=[]
    for site in ['moscow_mountain','topanga','az_ponderosa']:
        fine=grids[site,10];coarse=grids[site,30]
        native.append(dict(site=site,area10_m2=fine['basin_area_m2'],area30_m2=coarse['basin_area_m2'],T10=fine['T'],T30=coarse['T'],
                           basin_T_delta=None if fine['T'] is None or coarse['T'] is None else coarse['T']-fine['T']))
    assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in source_hashes.items())
    for name,data in [('terrain_results',records),('method_comparisons',comparisons),('conditioning',conditioned),('native_resolution',native),('timings',timings)]:csvwrite(out/f'{name}.csv',data)
    scenarios=[]
    for row in records:
        base=next(r for r in records if r['site']==row['site'] and r['resolution_m']==row['resolution_m'] and r['terrain']==row['terrain'] and r['method']=='Horn')
        for duration in [15,30,60]:
            for rainfall in [5,10,20]:
                p=None if row['T'] is None else engine.probability('M1',duration,T=row['T'],F=.5,S=.25,rainfall_mm=rainfall)
                h=None if base['T'] is None else engine.probability('M1',duration,T=base['T'],F=.5,S=.25,rainfall_mm=rainfall)
                scenarios.append(dict(site=row['site'],resolution_m=row['resolution_m'],terrain=row['terrain'],method=row['method'],duration_minutes=duration,rainfall_mm=rainfall,F=.5,S=.25,T=row['T'],probability=p,delta_from_Horn=None if p is None or h is None else p-h))
    csvwrite(out/'probability_scenarios.csv',scenarios)
    evidence=dict(binary=str(binary),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),sources_sha256=source_hashes,commands=calls,
                  gdal_version=subprocess.check_output(['gdalinfo','--version'],text=True).strip(),scalar_engine_sha256=hashlib.sha256(module_path.read_bytes()).hexdigest(),
                  sbs='synthetic: (floor(cell_center_x/210m)+floor(cell_center_y/330m))%4; same projected field at both resolutions; no real burn inference',F=.5,S=.25,rainfall_mm=20,duration_minutes=15)
    (out/'provenance.json').write_text(json.dumps(evidence,indent=2)+'\n')
    plot_panel(out,args.fixtures)

if __name__=='__main__':main()
