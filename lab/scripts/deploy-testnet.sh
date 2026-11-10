#!/usr/bin/env bash
# Deploy testnet copies of EURAU (AllUnity SEP-41 wasm), the Zebec streaming
# contract, a fresh privacy pool, and the atomic stream adapter.
set -euo pipefail
export PATH="${HOME}/.local/bin:${PATH}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LAB="${LAB:-/tmp/stellar-lab}"
WORK="${WORK:-/tmp/eurau-zebec-run}"
NETWORK="${NETWORK:-testnet}"
XLM_SAC="${XLM_SAC:-CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC}"
mkdir -p "$LAB/wasm" "$WORK"
cd "$WORK"

log() { printf '\n== %s\n' "$*" >&2; }

ensure_key() {
  local name="$1"
  if ! stellar keys ls 2>/dev/null | grep -qx "$name"; then
    log "creating $name"
    stellar keys generate "$name" --fund --network "$NETWORK"
  else
    stellar keys fund "$name" --network "$NETWORK" >/dev/null 2>&1 || true
  fi
  stellar keys address "$name"
}

log "accounts"
ADMIN="$(ensure_key eurau_admin)"
RECEIVER="$(ensure_key eurau_receiver)"
echo "admin=$ADMIN"
echo "receiver=$RECEIVER"

if [[ ! -f "$LAB/wasm/eurau.wasm" ]]; then
  log "fetch EURAU wasm from mainnet"
  stellar contract fetch \
    --id CB44W727WSLHPXJ47A6DHF5D34RKWSOZAMEDXO3CF5TEEEQ2ZX4V3VRI \
    --rpc-url https://mainnet.sorobanrpc.com \
    --network-passphrase "Public Global Stellar Network ; September 2015" \
    -o "$LAB/wasm/eurau.wasm"
fi
if [[ ! -f "$LAB/wasm/zebec.wasm" ]]; then
  stellar contract fetch --id CDWK5GRLUB24FMWLWEAS3NLU2JCDW7T3BMU7HWKGEZEDJRXXESGLQ4YU \
    --network "$NETWORK" -o "$LAB/wasm/zebec.wasm"
fi
if [[ ! -f "$LAB/wasm/privacy_pools.wasm" ]]; then
  stellar contract fetch --id CA5VMH2O7ZVBNBMAMRO3INM5NSTRQYLJEKOC352OKGFNBU45Y3O7CNQ4 \
    --network "$NETWORK" -o "$LAB/wasm/privacy_pools.wasm"
fi
if [[ ! -f "$LAB/wasm/registry.wasm" ]]; then
  stellar contract fetch --id CCYTAXXIHB2PY2AY423LIE3YOGY5BJSDP2EGWF6SXV3KF52R4VJ5X7AV \
    --network "$NETWORK" -o "$LAB/wasm/registry.wasm"
fi

if [[ ! -f "$LAB/wasm/zebec_pool_adapter.wasm" ]]; then
  echo "missing adapter wasm; build lab/zebec-pool-adapter first" >&2
  exit 1
fi

if [[ -f "$WORK/deploy.json" ]] && jq -e '.eurau' "$WORK/deploy.json" >/dev/null; then
  EURAU="$(jq -r '.eurau' "$WORK/deploy.json")"
  log "reuse EURAU $EURAU"
else
  log "deploy EURAU copy"
  EURAU="$(stellar contract deploy --wasm "$LAB/wasm/eurau.wasm" --source eurau_admin --network "$NETWORK" -- \
    --admin eurau_admin \
    --pauser eurau_admin \
    --upgrader eurau_admin \
    --minter eurau_admin \
    --blacklister eurau_admin \
    --burner eurau_admin \
    --blacklisted_burner eurau_admin \
    --decimals 6 \
    --name "AllUnity EUR" \
    --symbol EURAU | tail -1)"
  log "eurau=$EURAU"
fi

log "deploy Zebec copy"
CONFIG="$(jq -nc \
  --arg fee "$ADMIN" \
  --arg withdraw "$ADMIN" \
  --arg xlm "$XLM_SAC" \
  --arg token "$EURAU" \
  '{
    base_fee: 0,
    fee_recipient: $fee,
    fee_tiers: [{fee_bps: 0, max_amount: 1000000000000, min_amount: 0}],
    frequencies: [1],
    platform_fee: 0,
    stream_token_fee: 0,
    whitelisted_tokens: [$token],
    withdraw_account: $withdraw,
    xlm_token: $xlm
  }')"
ZEBEC="$(stellar contract deploy --wasm "$LAB/wasm/zebec.wasm" --source eurau_admin --network "$NETWORK" -- \
  --admin eurau_admin \
  --config "$CONFIG")"
echo "zebec=$ZEBEC"

log "deploy privacy pool"
POOL="$(stellar contract deploy --wasm "$LAB/wasm/privacy_pools.wasm" --source eurau_admin --network "$NETWORK" -- \
  --tree_depth 20 \
  --admin eurau_admin)"
echo "pool=$POOL"

log "deploy registry"
REGISTRY="$(stellar contract deploy --wasm "$LAB/wasm/registry.wasm" --source eurau_admin --network "$NETWORK" -- \
  --admin eurau_admin)"
echo "registry=$REGISTRY"

log "link registry"
stellar contract invoke --id "$POOL" --source eurau_admin --network "$NETWORK" --send yes -- \
  set_registry --registry "$REGISTRY"

if [[ ! -f "$LAB/main_verification_key.json" ]]; then
  curl -fsSL "https://static.pl.arcane.finance/stellar/0.15.0/main_verification_key.json" \
    -o "$LAB/main_verification_key.json"
fi
VK_HEX="$(node "$ROOT/lab/scripts/encode-vk.mjs" "$LAB/main_verification_key.json")"
log "register 2x2 zk config nonce 3"
stellar contract invoke --id "$POOL" --source eurau_admin --network "$NETWORK" --send yes -- \
  add_zk_config \
  --nonce 3 \
  --vk_bytes "$VK_HEX" \
  --n_ins 2 \
  --n_outs 2 \
  --public_n_inputs 1 \
  --public_n_outputs 1 \
  --n_audit_slots 4 \
  --note_audit_len 12 \
  --audit_offset 8 \
  --note_output_len 7 \
  --output_note_offset 14

log "deploy adapter"
ADAPTER="$(stellar contract deploy --wasm "$LAB/wasm/zebec_pool_adapter.wasm" --source eurau_admin --network "$NETWORK")"
echo "adapter=$ADAPTER"

jq -n \
  --arg admin "$ADMIN" \
  --arg receiver "$RECEIVER" \
  --arg eurau "$EURAU" \
  --arg zebec "$ZEBEC" \
  --arg pool "$POOL" \
  --arg registry "$REGISTRY" \
  --arg adapter "$ADAPTER" \
  --arg xlm "$XLM_SAC" \
  '{admin:$admin, receiver:$receiver, eurau:$eurau, zebec:$zebec, pool:$pool, registry:$registry, adapter:$adapter, xlm:$xlm}' \
  | tee "$WORK/deploy.json"
