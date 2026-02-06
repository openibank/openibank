#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${OPENIBANK_URL:-http://localhost:8080}"
DEMO_ENDPOINT="${BASE_URL%/}/api/demo/run"

if ! command -v curl >/dev/null 2>&1; then
  echo "error: curl is required" >&2
  exit 1
fi

echo "Running OpeniBank deterministic demo against ${BASE_URL}..."

response="$(curl -sS -X POST "${DEMO_ENDPOINT}" \
  -H 'Content-Type: application/json' \
  -d '{"commit":true}')"

if command -v jq >/dev/null 2>&1; then
  success="$(printf '%s' "${response}" | jq -r '.success // false')"
  if [[ "${success}" != "true" ]]; then
    echo "Demo failed:"
    printf '%s' "${response}" | jq .
    exit 1
  fi

  demo_id="$(printf '%s' "${response}" | jq -r '.demo_id')"
  scenario="$(printf '%s' "${response}" | jq -r '.scenario')"
  bundle_id="$(printf '%s' "${response}" | jq -r '.receipt_bundle_id')"
  commitments="$(printf '%s' "${response}" | jq -r '.commitment_ids | join(", ")')"
  buyer_balance="$(printf '%s' "${response}" | jq -r '.balances.buyer')"
  seller_balance="$(printf '%s' "${response}" | jq -r '.balances.seller')"
  export_url="$(printf '%s' "${response}" | jq -r '.share_export_url')"

  printf '\nDemo Summary\n'
  printf '  Demo ID: %s\n' "${demo_id}"
  printf '  Scenario: %s\n' "${scenario}"
  printf '  Commitment IDs: %s\n' "${commitments}"
  printf '  Receipt Bundle: %s\n' "${bundle_id}"
  printf '  Buyer Balance: $%.2f\n' "$(awk "BEGIN {print ${buyer_balance}/100}")"
  printf '  Seller Balance: $%.2f\n' "$(awk "BEGIN {print ${seller_balance}/100}")"
  printf '  Export URL: %s%s\n' "${BASE_URL%/}" "${export_url}"
  printf '\n'
else
  echo "jq not found; raw response:"
  echo "${response}"
fi
