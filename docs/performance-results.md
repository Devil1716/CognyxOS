# Performance Results

Test profile (cargo test debug), Windows host, 2026-08-14. Not a release benchmark.

| Operation | Time |
|---|---|
| Kernel submit_task to handle | 19 ms |
| e2e_phase5 including real Notepad | 1.98 s |
| system_validation 16 tests | 2.41 s |
| native_host 4 tests | 0.88 s |
| browser_integration 6 tests | 0.17 s |
| e2e_phase6 | 0.06 s |
| cargo test --workspace offline target-val | ~212 s then fail-fast VAL-001 |

Cannot claim production performance.
