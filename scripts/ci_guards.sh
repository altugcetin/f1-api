#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

fail() {
  echo "GUARD FAIL: $*" >&2
  FAIL=1
}

BANNED_REF='signalr|livetiming|negotiate|ingest'
while IFS= read -r -d '' file; do
  if rg -ni --pcre2 "$BANNED_REF" "$file" >/dev/null 2>&1; then
    fail "banned reference in $file"
    rg -ni --pcre2 "$BANNED_REF" "$file" >&2 || true
  fi
done < <(find "$ROOT" -type f \
  \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.md' -o -name '*.yaml' -o -name '*.yml' -o -name '*.toml' -o -name '*.json' \) \
  -not -path '*/target/*' -not -path '*/.git/*' -not -path '*/scripts/ci_guards.sh' -print0)

if rg -n --pcre2 '\x{2014}' "$ROOT" -g '!target/**' -g '!.git/**' >/dev/null 2>&1; then
  fail "em-dash (U+2014) found"
  rg -n --pcre2 '\x{2014}' "$ROOT" -g '!target/**' -g '!.git/**' >&2 || true
fi

if [[ ! -f "$ROOT/LICENSE" ]]; then
  fail "LICENSE missing"
fi

if ! rg -q "unofficial project" "$ROOT/README.md"; then
  fail "disclaimer missing from README"
fi

if ! rg -q "non-commercial" "$ROOT/README.md"; then
  fail "non-commercial statement missing from README"
fi

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi

echo "ci guards passed"
