# weppcloud-wbt Release Build + Install Runbook

This runbook is the canonical release procedure for publishing a `weppcloud-wbt`
build that WEPPpy workers can execute safely.

## Why This Exists

`wepppy` executes `/workdir/weppcloud-wbt/WBT/whitebox_tools` at runtime.
If that binary is stale, wrappers may expose tools that the runtime binary does
not actually contain (for example `IterativeFirstOrderLinkPrune`), causing RQ
job failures.

## Release Checklist

Run from `/workdir/weppcloud-wbt` unless noted.

1. Build the release binary.

```bash
cargo build -p whitebox_tools --release
```

2. Install the binary into the tracked runtime artifact path.

```bash
cp target/release/whitebox_tools WBT/whitebox_tools.new
chmod 755 WBT/whitebox_tools.new
mv -f WBT/whitebox_tools.new WBT/whitebox_tools
```

3. Verify tool availability from the installed binary.

```bash
cd WBT
./whitebox_tools --listtools | grep -E "IterativeFirstOrderLinkPrune|RemoveShortStreams|RaiseRoads"
./whitebox_tools --toolhelp=IterativeFirstOrderLinkPrune | sed -n '1,30p'
```

4. Verify wrapper surfaces compile.

```bash
cd /workdir/weppcloud-wbt
python -m py_compile whitebox_tools.py WBT/whitebox_tools.py
```

5. Verify from WEPPpy container runtime (required for cutover confidence).

```bash
cd /workdir/wepppy
wctl exec weppcloud bash -lc 'cd /workdir/weppcloud-wbt/WBT && ./whitebox_tools --listtools | grep -E "IterativeFirstOrderLinkPrune|RemoveShortStreams"'
```

6. Commit and push release artifacts.

```bash
cd /workdir/weppcloud-wbt
git add -u WBT/whitebox_tools
git commit -m "Build WBT binary for release"
git push
```

## Required Release Artifacts

- `WBT/whitebox_tools` (binary)
- Any wrapper changes in:
  - `whitebox_tools.py`
  - `WBT/whitebox_tools.py`
- Any tool implementation or registration changes in `whitebox-tools-app/...`

## Failure Pattern + Immediate Triage

Symptom in WEPPpy worker logs:

- `WhiteboxAppError: Unrecognized tool name IterativeFirstOrderLinkPrune`

Immediate fix:

1. Re-run this runbook.
2. Confirm step 5 passes from container runtime.
3. Retry failed RQ job.
