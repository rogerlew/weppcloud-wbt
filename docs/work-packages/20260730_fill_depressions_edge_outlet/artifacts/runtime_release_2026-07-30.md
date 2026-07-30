# FillDepressions WEPPpy runtime release

Built and validated on 2026-07-30 UTC from `/workdir/weppcloud-wbt`.

## Provenance

| Artifact | SHA-256 |
| --- | --- |
| `Cargo.lock` | `c26bacf2dfff0bada3d4f67d162917ab237ebec7fb0dcf4f60a385d7ec21225e` |
| Prior `WBT/whitebox_tools` | `491f892aabf83a6ecde7639473f94c63004935b275f3d846f9eddaee1c5cb14f` |
| `target/release/whitebox_tools` | `0dbb64c96a05d7e53eaa9930ad777ee207210a569c40db8d3ac11f69ec91b7d3` |
| Installed `WBT/whitebox_tools` | `0dbb64c96a05d7e53eaa9930ad777ee207210a569c40db8d3ac11f69ec91b7d3` |

The installed binary was compared byte-for-byte with the locked release build.

## Build and validation

- `cargo build --locked -p whitebox-tools-app --release`: pass.
- Installed-binary discovery exposed `FillDepressions`,
  `RemoveShortStreams`, and `TopazConditionDem`.
- WEPPpy `weppcloud` container discovery exposed the same tools.
- The container executed installed `FillDepressions` against the exact
  447-by-430 production reproducer and wrote a non-empty disposable output.
- A host-side installed-binary run preserved the issue reference elevation at
  `533.868286132812 m`.
- `python3 -m py_compile whitebox_tools.py WBT/whitebox_tools.py`: pass.
- `python3 -m unittest discover -s tests -p 'test_*.py'`: pass, 2 tests.
- The source work package previously passed all 140 Rust tests and the
  differential depression-inventory gate.

The disposable container and host outputs were created under `/tmp`; they are
not release artifacts.
