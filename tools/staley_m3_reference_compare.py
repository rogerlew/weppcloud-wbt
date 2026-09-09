"""Public-API external reference probe; no reference algorithm is reproduced."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import importlib.metadata

import numpy as np
import rasterio
from rasterio.transform import from_origin
from pfdf import watershed
from pfdf.raster import Raster


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=False)
    assert importlib.metadata.version('pysheds')=='0.4'
    assert importlib.metadata.version('pfdf')=='3.0.2'
    rows=[]
    for name,heights in [('descending',[130,120,100,95,90]),('internal_maximum',[110,130,100,95,90]),('pit',[130,90,100,95,90])]:
        directory=args.output/name;directory.mkdir()
        z=np.full((5,9),-9999.,dtype='float64');p=z.copy();z[2,2:7]=heights;p[2,2:7]=2
        profile=dict(driver='GTiff',height=5,width=9,count=1,dtype='float64',crs='EPSG:32611',transform=from_origin(500000,5200000,10,10),nodata=-9999)
        for filename,values in [('dem.tif',z),('pointer.tif',p)]:
            with rasterio.open(directory/filename,'w',**profile) as out:out.write(values,1)
        subprocess.run([str(args.binary),'-r=D8UpstreamRelief','--compress_rasters=false',f'--dem={directory}/dem.tif',f'--d8_pntr={directory}/pointer.tif',f'--output={directory}/h.tif',f'--area={directory}/a.tif','--elevation_units=m'],check=True,capture_output=True)
        f=np.where(p==2,1,p)
        kw=dict(nodata=-9999,crs=32611,transform=profile['transform'])
        reference_h=watershed.relief(Raster.from_array(z,**kw),Raster.from_array(f,**kw)).values
        reference_a=watershed.accumulation(Raster.from_array(f,**kw),times=100).values
        with rasterio.open(directory/'h.tif') as ds:h=ds.read(1)
        with rasterio.open(directory/'a.tif') as ds:a=ds.read(1)
        assert h[2,4]==30 and a[2,4]==300
        rows.append(dict(case=name,raw_elevations=heights,wbt_h=h[2,2:7].tolist(),reference_h=reference_h[2,2:7].tolist(),wbt_area=a[2,2:7].tolist(),reference_area=reference_a[2,2:7].tolist(),outlet_h_error=float(reference_h[2,4]-h[2,4]),outlet_t_error=float((reference_h[2,4]-h[2,4])/np.sqrt(300))))
    evidence=dict(binary=str(args.binary),binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),reference_module=watershed.__file__,versions={k:importlib.metadata.version(k) for k in ['pfdf','pysheds','numpy','numba','rasterio']},cases=rows)
    (args.output/'comparison.json').write_text(json.dumps(evidence,indent=2));print(json.dumps(evidence,indent=2))


if __name__=='__main__':main()
