# M4-T04 Pipeline Summary
Date: 2026-02-19
Result: fail
Pipeline: `scripts/run_m4_compat_pipeline.sh`
LOOP_MIN_AVAILABLE_MIB: `4096`
PIPELINE_STRICT: `0`

## Required Gates

- `VG-RCC-001`: fail (`M4-T04-VG-RCC-001.txt`)
- `VG-LA-001`: pass (`M4-T04-VG-LA-001.txt`)
- `VG-RFLX-001`: fail (`M4-T04-VG-RFLX-001.txt`)

## Optional Advisory Gates

- `VG-LA-002`: pass (enabled when `RUN_LA_FULL_SNAPSHOT=1`)
