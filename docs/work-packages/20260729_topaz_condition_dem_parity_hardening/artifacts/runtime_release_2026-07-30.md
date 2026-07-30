# TopazConditionDem Runtime Release Evidence

**Date**: 2026-07-30 UTC

**Starting source revision**:
`0f2960ffa45b69814ab3a0dc1c6cc7216574fb48`

## Provenance

| Artifact | SHA-256 |
| --- | --- |
| `Cargo.lock` | `c26bacf2dfff0bada3d4f67d162917ab237ebec7fb0dcf4f60a385d7ec21225e` |
| Prior `WBT/whitebox_tools` | `20209bd7216f22e7dadb35d26612db03f40d12ec20b27d8b51543e58375aabdc` |
| Locked release build | `e5b33364b788f0046db15760320c7b03c6412fda99987f2bbe3ac76ba53b4cd0` |
| Installed `WBT/whitebox_tools` | `e5b33364b788f0046db15760320c7b03c6412fda99987f2bbe3ac76ba53b4cd0` |

The installed artifact is byte-identical to the locked release build.

## Validation

- `cargo build --locked -p whitebox-tools-app --release` passed.
- `cargo test --locked -p whitebox-tools-app` passed 132 tests.
- Both Python wrappers compiled.
- Wrapper tests passed timeout forwarding, descendant process-group cleanup,
  explicit timeout failure, and explicit nonzero-exit failure.
- The installed binary passed all seven canonical TOPAZ FILDEP/RELIEF parity
  cases with manifest SHA-256
  `f0e397804978bae9c34568c745cb5e4f327b62ef1d300b95159b1e8b9703667b`.
- The local WEPPpy container discovered `TopazConditionDem` and executed it at
  width 2 against `test_fixtures/topaz_condition_dem/dem.tif`. The disposable
  output SHA-256 was
  `31ed622d6dc5f8c3b190935cdfc600ed7f9381aaad262fcc069e43d64246a299`.

Production fleet deployment remains a separate operation. Each worker host
must repeat discovery and disposable execution before a WEPPpy default that
depends on this release is deployed there.
