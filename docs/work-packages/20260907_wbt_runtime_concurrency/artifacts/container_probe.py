import hashlib,json,os,shutil,tempfile
from pathlib import Path
from wepppy.topo.wbt.wbt_topaz_emulator import WhiteboxToolsTopazEmulator
root=Path('/workdir/weppcloud-wbt')
with tempfile.TemporaryDirectory(prefix='wbt-runtime-') as tmp:
    out=Path(tmp)
    shutil.copy2(root/'target/release/whitebox_tools',out/'whitebox_tools')
    (out/'settings.json').write_text(json.dumps({'verbose_mode':True,'working_directory':'','compress_rasters':False,'max_procs':4}))
    before=(out/'settings.json').read_bytes()
    w=WhiteboxToolsTopazEmulator(str(out/'output'),str(root/'test_fixtures/breach_least_cost/flat_edge.tif'),verbose=True)
    w.wbt.set_whitebox_dir(str(out))
    w._create_relief('breach_least_cost',blc_dist=320,blc_fill=False,blc_fail_on_unresolved=True)
    assert Path(w.relief).is_file()
    assert (out/'settings.json').read_bytes()==before
    diagnostics=json.loads(Path(w.conditioning_diagnostics).read_text())
    reference=json.loads((root/'target/breach-benchmark/reference-flat-edge-1/result.json').read_text())['diagnostics']
    reference['parameters']['fail_on_unresolved']=True
    for key in ['conditioning','terrain_change','parameters']:
        assert diagnostics[key]==reference[key],key
    report={'status':'passed','uid':os.getuid(),'gid':os.getgid(),'environment_limit':os.environ['WBT_MAX_PROCS'],'persisted_limit':4,'resolved_workers':12,'diagnostic_parity':True,'binary_sha256':hashlib.sha256((out/'whitebox_tools').read_bytes()).hexdigest()}
    print('RUNTIME_CONTAINER_RESULT '+json.dumps(report),flush=True)
