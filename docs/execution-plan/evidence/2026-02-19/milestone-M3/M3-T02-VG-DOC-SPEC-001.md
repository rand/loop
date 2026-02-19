# M3-T02 VG-DOC-SPEC-001
Date: 2026-02-19
Task: M3-T02 Fix SPEC-20 file locations and runtime references
Gate: VG-DOC-SPEC-001
Result: pass

## Checklist

- [x] SPEC-20 file-location table updated to repository-accurate paths.
- [x] Derive macro location corrected to `rlm-core-derive/src/lib.rs`.
- [x] Python REPL path references corrected to `rlm-core/python/rlm_repl/*`.
- [x] Rust REPL bridge path corrected to `rlm-core/src/repl.rs`.
- [x] SPEC-20 test-plan entries updated to real test identifiers/modules.
- [x] File-location existence check run (`M3-T02-file-location-check.txt`).
- [x] Test-reference existence check run (`M3-T02-test-reference-check.txt`).

## Conclusion

No invalid file paths remain in targeted SPEC-20 sections, and listed test references map to real test modules/functions.
