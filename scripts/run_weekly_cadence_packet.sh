#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="/Users/rand/src/loop"
PIPELINE_SCRIPT="$ROOT_DIR/scripts/run_m4_compat_pipeline.sh"

EVIDENCE_DATE="${EVIDENCE_DATE:-$(date +%F)}"
EVIDENCE_DIR_DEFAULT="$ROOT_DIR/docs/execution-plan/evidence/$EVIDENCE_DATE/milestone-M6"
EVIDENCE_DIR="${EVIDENCE_DIR:-$EVIDENCE_DIR_DEFAULT}"
PACKET_PREFIX="${PACKET_PREFIX:-weekly-cadence}"

RUN_LOG="$EVIDENCE_DIR/${PACKET_PREFIX}-run.log"
TUPLE_TXT="$EVIDENCE_DIR/${PACKET_PREFIX}-tuples.txt"
PACKET_MD="$EVIDENCE_DIR/${PACKET_PREFIX}-packet.md"
PIPELINE_EVIDENCE_DIR="$EVIDENCE_DIR/${PACKET_PREFIX}-m4"

MIN_AVAILABLE_MIB="${LOOP_MIN_AVAILABLE_MIB:-4096}"
RUN_LA_FULL_SNAPSHOT="${RUN_LA_FULL_SNAPSHOT:-1}"
LOOP_AGENT_CANONICAL_DIR="${LOOP_AGENT_CANONICAL_DIR:-/Users/rand/src/loop-agent}"
LOOP_AGENT_CLEAN_CLONE_DIR="${LOOP_AGENT_CLEAN_CLONE_DIR:-/tmp/loop-agent-clean-cadence}"
LOOP_AGENT_TUPLE_POLICY="${LOOP_AGENT_TUPLE_POLICY:-committed_clean_clone_only}"

LOOP_AGENT_DIR="$LOOP_AGENT_CANONICAL_DIR"
LOOP_AGENT_TUPLE_MODE="canonical"

mkdir -p "$EVIDENCE_DIR"

: >"$RUN_LOG"
exec >>"$RUN_LOG" 2>&1

echo "weekly cadence packet starting"
echo "date: $(date -u '+UTC %Y-%m-%d %H:%M:%S')"
echo "evidence_dir: $EVIDENCE_DIR"
echo "pipeline_evidence_dir: $PIPELINE_EVIDENCE_DIR"
echo "loop_min_available_mib: $MIN_AVAILABLE_MIB"
echo "run_la_full_snapshot: $RUN_LA_FULL_SNAPSHOT"
echo "loop_agent_tuple_policy: $LOOP_AGENT_TUPLE_POLICY"
echo "loop_agent_canonical_dir: $LOOP_AGENT_CANONICAL_DIR"
echo "loop_agent_clean_clone_dir: $LOOP_AGENT_CLEAN_CLONE_DIR"
echo

capture_repo_ref() {
  local label="$1"
  local dir="$2"
  local branch
  local sha
  branch="$(git -C "$dir" rev-parse --abbrev-ref HEAD)"
  sha="$(git -C "$dir" rev-parse HEAD)"
  echo "${label}_branch=${branch}" >>"$TUPLE_TXT"
  echo "${label}_sha=${sha}" >>"$TUPLE_TXT"
}

capture_rcc_vendor_pin() {
  local line
  line="$(git -C /Users/rand/src/rlm-claude-code submodule status vendor/loop)"
  echo "rlm_claude_code_vendor_loop=${line}" >>"$TUPLE_TXT"
}

# Keep uv/python caches in writable temp locations when running from this sandbox.
export UV_CACHE_DIR="${UV_CACHE_DIR:-/tmp/loop-uv-cache}"
export PYTHONDONTWRITEBYTECODE=1
export PYTHONPYCACHEPREFIX="${PYTHONPYCACHEPREFIX:-/tmp/loop-pycache}"
export HYPOTHESIS_STORAGE_DIRECTORY="${HYPOTHESIS_STORAGE_DIRECTORY:-/tmp/loop-hypothesis}"
export PYTEST_ADDOPTS="${PYTEST_ADDOPTS:- -p no:cacheprovider}"

: >"$TUPLE_TXT"

if [[ "$LOOP_AGENT_TUPLE_POLICY" != "committed_clean_clone_only" ]]; then
  echo "unsupported LOOP_AGENT_TUPLE_POLICY: $LOOP_AGENT_TUPLE_POLICY" >&2
  echo "allowed value: committed_clean_clone_only" >&2
  exit 1
fi

LOOP_AGENT_CANONICAL_SHA="$(git -C "$LOOP_AGENT_CANONICAL_DIR" rev-parse HEAD)"
LOOP_AGENT_CANONICAL_DIRTY="0"
if [[ -n "$(git -C "$LOOP_AGENT_CANONICAL_DIR" status --porcelain)" ]]; then
  LOOP_AGENT_CANONICAL_DIRTY="1"
fi

rm -rf "$LOOP_AGENT_CLEAN_CLONE_DIR"
git clone "$LOOP_AGENT_CANONICAL_DIR" "$LOOP_AGENT_CLEAN_CLONE_DIR" >/dev/null 2>&1
git -C "$LOOP_AGENT_CLEAN_CLONE_DIR" checkout --detach "$LOOP_AGENT_CANONICAL_SHA" >/dev/null 2>&1
LOOP_AGENT_DIR="$LOOP_AGENT_CLEAN_CLONE_DIR"
LOOP_AGENT_TUPLE_MODE="clean_clone_committed"

if [[ -n "$(git -C "$LOOP_AGENT_DIR" status --porcelain)" ]]; then
  echo "loop_agent tuple directory is not clean: $LOOP_AGENT_DIR" >&2
  exit 1
fi

capture_repo_ref "loop" "/Users/rand/src/loop"
capture_repo_ref "rlm_claude_code" "/Users/rand/src/rlm-claude-code"
capture_rcc_vendor_pin
capture_repo_ref "loop_agent_canonical" "$LOOP_AGENT_CANONICAL_DIR"
capture_repo_ref "loop_agent" "$LOOP_AGENT_DIR"
capture_repo_ref "io_rflx" "/Users/rand/src/io-rflx"
echo "io_rflx_interop_schema=io_rflx_interop.v0" >>"$TUPLE_TXT"
echo "loop_agent_tuple_mode=$LOOP_AGENT_TUPLE_MODE" >>"$TUPLE_TXT"
echo "loop_agent_tuple_dir=$LOOP_AGENT_DIR" >>"$TUPLE_TXT"
echo "loop_agent_canonical_dirty=$LOOP_AGENT_CANONICAL_DIRTY" >>"$TUPLE_TXT"

echo "tuple snapshot captured: $TUPLE_TXT"

LA_PYTEST_CMD_RUN="${LA_PYTEST_CMD:-}"
if [[ -z "$LA_PYTEST_CMD_RUN" && -x "$LOOP_AGENT_DIR/.venv/bin/pytest" ]]; then
  LA_PYTEST_CMD_RUN="$LOOP_AGENT_DIR/.venv/bin/pytest"
fi
if [[ -z "$LA_PYTEST_CMD_RUN" && -x "$LOOP_AGENT_CANONICAL_DIR/.venv/bin/pytest" ]]; then
  LA_PYTEST_CMD_RUN="$LOOP_AGENT_CANONICAL_DIR/.venv/bin/pytest"
fi

LA_PYTHONPATH_RUN="${LA_PYTHONPATH:-}"
if [[ "$LOOP_AGENT_TUPLE_MODE" == "clean_clone_committed" && -z "$LA_PYTHONPATH_RUN" ]]; then
  LA_PYTHONPATH_RUN="$LOOP_AGENT_DIR/src"
fi

EVIDENCE_DIR="$PIPELINE_EVIDENCE_DIR" \
EVIDENCE_DATE="$EVIDENCE_DATE" \
LOOP_MIN_AVAILABLE_MIB="$MIN_AVAILABLE_MIB" \
RUN_LA_FULL_SNAPSHOT="$RUN_LA_FULL_SNAPSHOT" \
LOOP_AGENT_DIR="$LOOP_AGENT_DIR" \
LA_PYTEST_CMD="$LA_PYTEST_CMD_RUN" \
LA_PYTHONPATH="$LA_PYTHONPATH_RUN" \
PIPELINE_STRICT=0 \
  "$PIPELINE_SCRIPT"

PIPELINE_RESULT="unknown"
if [[ -f "$PIPELINE_EVIDENCE_DIR/M4-T04-pipeline-summary.md" ]]; then
  PIPELINE_RESULT="$(rg '^Result:' "$PIPELINE_EVIDENCE_DIR/M4-T04-pipeline-summary.md" | head -n 1 | sed 's/^Result:[[:space:]]*//')"
fi

VG_RCC_STATUS="unknown"
VG_LA_STATUS="unknown"
VG_RFLX_STATUS="unknown"
if [[ -f "$PIPELINE_EVIDENCE_DIR/M4-T04-pipeline-summary.md" ]]; then
  VG_RCC_STATUS="$(rg 'VG-RCC-001' "$PIPELINE_EVIDENCE_DIR/M4-T04-pipeline-summary.md" | sed -E 's/.*VG-RCC-001`: ([a-z_]+).*/\1/' | head -n 1 || true)"
  VG_LA_STATUS="$(rg 'VG-LA-001' "$PIPELINE_EVIDENCE_DIR/M4-T04-pipeline-summary.md" | sed -E 's/.*VG-LA-001`: ([a-z_]+).*/\1/' | head -n 1 || true)"
  VG_RFLX_STATUS="$(rg 'VG-RFLX-001' "$PIPELINE_EVIDENCE_DIR/M4-T04-pipeline-summary.md" | sed -E 's/.*VG-RFLX-001`: ([a-z_]+).*/\1/' | head -n 1 || true)"
fi

RCC_NOTE="none"
if [[ -f "$PIPELINE_EVIDENCE_DIR/M4-T04-VG-RCC-001.txt" ]]; then
  RCC_NOTE="$(rg -n 'ImportError|error:|failed|Operation not permitted' "$PIPELINE_EVIDENCE_DIR/M4-T04-VG-RCC-001.txt" | head -n 1 | sed 's/^[0-9]*://' || true)"
  RCC_NOTE="${RCC_NOTE:-none}"
fi

RFLX_NOTE="none"
if [[ -f "$PIPELINE_EVIDENCE_DIR/M4-T04-VG-RFLX-001.txt" ]]; then
  RFLX_NOTE="$(rg -n 'error:|failed|Operation not permitted' "$PIPELINE_EVIDENCE_DIR/M4-T04-VG-RFLX-001.txt" | head -n 1 | sed 's/^[0-9]*://' || true)"
  RFLX_NOTE="${RFLX_NOTE:-none}"
fi

LA_SNAPSHOT_NOTE="not_run"
if [[ -f "$PIPELINE_EVIDENCE_DIR/M4-T04-VG-LA-002.txt" ]]; then
  LA_LINE="$(rg -n '([0-9]+ failed|[0-9]+ passed|no tests ran)' "$PIPELINE_EVIDENCE_DIR/M4-T04-VG-LA-002.txt" | tail -n 1 || true)"
  if [[ -n "$LA_LINE" ]]; then
    LA_SNAPSHOT_NOTE="$(echo "$LA_LINE" | sed 's/^[0-9]*://')"
  else
    LA_SNAPSHOT_NOTE="captured (see artifact)"
  fi
fi

cat >"$PACKET_MD" <<EOF
# Weekly Cadence Packet
Date: $(date -u '+%Y-%m-%d')
Result: $PIPELINE_RESULT
Runner: \`scripts/run_weekly_cadence_packet.sh\`
LOOP_MIN_AVAILABLE_MIB: \`$MIN_AVAILABLE_MIB\`
RUN_LA_FULL_SNAPSHOT: \`$RUN_LA_FULL_SNAPSHOT\`

## Governance Sources

- \`docs/execution-plan/COMPATIBILITY-MATRIX.md\`
- \`docs/execution-plan/RELEASE-ROLLBACK-PLAYBOOK.md\`
- \`docs/execution-plan/MAINTENANCE-CADENCE.md\`

## Tuple Snapshot

\`\`\`
$(cat "$TUPLE_TXT")
\`\`\`

## Compatibility Gate Artifacts

- \`VG-RCC-001\`: \`$VG_RCC_STATUS\` (\`$PIPELINE_EVIDENCE_DIR/M4-T04-VG-RCC-001.txt\`)
- \`VG-LA-001\`: \`$VG_LA_STATUS\` (\`$PIPELINE_EVIDENCE_DIR/M4-T04-VG-LA-001.txt\`)
- \`VG-RFLX-001\`: \`$VG_RFLX_STATUS\` (\`$PIPELINE_EVIDENCE_DIR/M4-T04-VG-RFLX-001.txt\`)
- \`VG-LA-002\` advisory snapshot: \`$LA_SNAPSHOT_NOTE\`

## Gate Notes

- \`VG-RCC-001\`: \`$RCC_NOTE\`
- \`VG-RFLX-001\`: \`$RFLX_NOTE\`

## Policy Notes

- Full-suite \`VG-LA-002\` promotion criteria are governed by D-014.
- \`loop-agent\` claim-grade tuple evidence is restricted to clean-clone committed mode while D-017 is active.
- This packet is intended for weekly cadence review and release-readiness context updates.
EOF

echo
echo "weekly cadence packet finished"
echo "packet: $PACKET_MD"
