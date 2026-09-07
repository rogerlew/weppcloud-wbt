import json,os,subprocess
from pathlib import Path
root=Path('/workdir/wepppy')
results=[]
models=[['docker-compose.dev.yml'],['docker-compose.dev.hpc.yml'],['docker-compose.prod.yml'],['docker-compose.prod.yml','docker-compose.prod.wepp1.yml'],['docker-compose.prod.worker.yml']]
for files in models:
 for override in [None,'6']:
  env=os.environ.copy();env.pop('WBT_MAX_PROCS',None)
  env['RQ_REDIS_URL']='redis://127.0.0.1:6379/9'
  if override is not None:env['WBT_MAX_PROCS']=override
  env.pop('WCTL_COMPOSE_FILE_EXTRAS',None);env.pop('WCTL_COMPOSE_FILES',None)
  if len(files)>1:env['WCTL_COMPOSE_FILE_EXTRAS']=','.join('docker/'+f for f in files[1:])
  command=['wctl','--compose-file','docker/'+files[0],'docker','compose']
  command+=['config','--format','json']
  proc=subprocess.run(command,env=env,cwd=root,text=True,capture_output=True)
  if proc.returncode:
   print('Configuration failed for',files,'exit',proc.returncode)
   Path('/tmp/wbt-compose-error.log').write_text(proc.stderr)
   raise SystemExit(1)
  model=json.loads(proc.stdout)
  values={name:model['services'][name]['environment']['WBT_MAX_PROCS'] for name in ['rq-worker','rq-worker-batch']}
  assert set(values.values())=={override or '12'},(files,values)
  results.append({'files':files,'operator_override':override,'resolved':values})
print(json.dumps(results,indent=2))
Path('/workdir/weppcloud-wbt/docs/work-packages/20260907_wbt_runtime_concurrency/artifacts/compose.json').write_text(json.dumps(results,indent=2)+'\n')
