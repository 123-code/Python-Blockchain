#!/usr/bin/env bash
set -uo pipefail

BASE="http://localhost:8080"
PASS=0
FAIL=0
TOTAL=0

pass() { ((PASS++)); ((TOTAL++)); echo "  PASS: $1"; }
fail() { ((FAIL++)); ((TOTAL++)); echo "  FAIL: $1 -- got: $2"; }

assert_eq() {
  local desc="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then pass "$desc"; else fail "$desc" "$actual (expected $expected)"; fi
}

assert_contains() {
  local desc="$1" needle="$2" haystack="$3"
  if echo "$haystack" | grep -q "$needle"; then pass "$desc"; else fail "$desc" "missing '$needle'"; fi
}

assert_status() {
  local desc="$1" expected="$2" url="$3" method="${4:-GET}" body="${5:-}"
  local status
  if [ "$method" = "POST" ]; then
    status=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$url" -H "Content-Type: application/json" -d "$body")
  else
    status=$(curl -s -o /dev/null -w '%{http_code}' "$url")
  fi
  assert_eq "$desc" "$expected" "$status"
}

jq_val() { echo "$1" | python3 -c "import sys,json; print(json.load(sys.stdin)$2)"; }

post_json() { curl -s -X POST "$1" -H "Content-Type: application/json" -d "$2"; }
get_json()  { curl -s "$1"; }

# ════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo " TEST SUITE: L1 Blockchain"
echo "═══════════════════════════════════════════════"

echo "── 1. Genesis state ──"
R=$(get_json "$BASE/chain")
assert_eq "chain length is 1" "1" "$(jq_val "$R" '["length"]')"
assert_eq "genesis index is 1" "1" "$(jq_val "$R" '["chain"][0]["index"]')"
assert_eq "genesis prev_hash is 0" "0" "$(jq_val "$R" '["chain"][0]["previous_hash"]')"
assert_eq "genesis data" "Genesis Block" "$(jq_val "$R" '["chain"][0]["data"]')"
assert_eq "genesis proof is 1" "1" "$(jq_val "$R" '["chain"][0]["proof"]')"

echo "── 2. Mine blocks ──"
R=$(post_json "$BASE/mine" '{"data":"Block A"}')
assert_eq "mine block A message" "Block mined successfully" "$(jq_val "$R" '["message"]')"
assert_eq "block A index" "2" "$(jq_val "$R" '["block"]["index"]')"
assert_eq "block A data" "Block A" "$(jq_val "$R" '["block"]["data"]')"

R=$(post_json "$BASE/mine" '{"data":"Block B"}')
assert_eq "mine block B index" "3" "$(jq_val "$R" '["block"]["index"]')"

R=$(post_json "$BASE/mine" '{}')
assert_eq "mine with no data defaults" "No data" "$(jq_val "$R" '["block"]["data"]')"

echo "── 3. Chain integrity ──"
R=$(get_json "$BASE/chain")
assert_eq "chain length after mining" "4" "$(jq_val "$R" '["length"]')"

# verify hash linkage
HASH1=$(jq_val "$R" '["chain"][0]["hash"]')
PREV2=$(jq_val "$R" '["chain"][1]["previous_hash"]')
HASH2=$(jq_val "$R" '["chain"][1]["hash"]')
PREV3=$(jq_val "$R" '["chain"][2]["previous_hash"]')
HASH3=$(jq_val "$R" '["chain"][2]["hash"]')
PREV4=$(jq_val "$R" '["chain"][3]["previous_hash"]')
assert_eq "block 2 prev_hash links to block 1" "$HASH1" "$PREV2"
assert_eq "block 3 prev_hash links to block 2" "$HASH2" "$PREV3"
assert_eq "block 4 prev_hash links to block 3" "$HASH3" "$PREV4"

echo "── 4. Chain validation ──"
R=$(get_json "$BASE/validate")
assert_eq "chain is valid" "True" "$(jq_val "$R" '["valid"]')"
assert_eq "validated length" "4" "$(jq_val "$R" '["length"]')"

echo "── 5. HTTP status codes ──"
assert_status "GET /chain returns 200" "200" "$BASE/chain"
assert_status "GET /validate returns 200" "200" "$BASE/validate"
assert_status "POST /mine returns 201" "201" "$BASE/mine" "POST" '{"data":"x"}'

# ════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo " TEST SUITE: L2 Deposits, Transfers, Withdrawals"
echo "═══════════════════════════════════════════════"

echo "── 6. Empty initial L2 state ──"
R=$(get_json "$BASE/l2/balances")
assert_eq "no balances initially" "0" "$(echo "$R" | python3 -c "import sys,json; print(len(json.load(sys.stdin)['balances']))")"

R=$(get_json "$BASE/l2/pending")
assert_eq "no pending txs" "0" "$(jq_val "$R" '["count"]')"

echo "── 7. Deposits ──"
R=$(post_json "$BASE/l2/deposit" '{"account":"alice","amount":10000}')
assert_contains "deposit alice" "deposited 10000 to alice" "$R"

R=$(post_json "$BASE/l2/deposit" '{"account":"bob","amount":5000}')
assert_contains "deposit bob" "deposited 5000 to bob" "$R"

R=$(post_json "$BASE/l2/deposit" '{"account":"charlie","amount":2000}')
assert_contains "deposit charlie" "deposited 2000 to charlie" "$R"

R=$(get_json "$BASE/l2/balances")
assert_eq "alice balance after deposit" "10000" "$(jq_val "$R" '["balances"]["alice"]')"
assert_eq "bob balance after deposit" "5000" "$(jq_val "$R" '["balances"]["bob"]')"
assert_eq "charlie balance after deposit" "2000" "$(jq_val "$R" '["balances"]["charlie"]')"

echo "── 8. Transfers ──"
R=$(post_json "$BASE/l2/transfer" '{"from":"alice","to":"bob","amount":2500}')
assert_contains "transfer alice->bob" "transferred 2500 from alice to bob" "$R"

R=$(post_json "$BASE/l2/transfer" '{"from":"bob","to":"charlie","amount":1000}')
assert_contains "transfer bob->charlie" "transferred 1000 from bob to charlie" "$R"

R=$(get_json "$BASE/l2/balances")
assert_eq "alice after transfers" "7500" "$(jq_val "$R" '["balances"]["alice"]')"
assert_eq "bob after transfers" "6500" "$(jq_val "$R" '["balances"]["bob"]')"
assert_eq "charlie after transfers" "3000" "$(jq_val "$R" '["balances"]["charlie"]')"

echo "── 9. Withdrawals ──"
R=$(post_json "$BASE/l2/withdraw" '{"account":"charlie","amount":500}')
assert_contains "withdraw charlie" "withdrew 500 from charlie" "$R"

R=$(get_json "$BASE/l2/balance/charlie")
assert_eq "charlie after withdrawal" "2500" "$(jq_val "$R" '["balance"]')"

echo "── 10. Balance per account ──"
R=$(get_json "$BASE/l2/balance/alice")
assert_eq "GET alice balance" "7500" "$(jq_val "$R" '["balance"]')"
assert_eq "GET alice account field" "alice" "$(jq_val "$R" '["account"]')"

R=$(get_json "$BASE/l2/balance/nonexistent")
assert_eq "nonexistent account balance is 0" "0" "$(jq_val "$R" '["balance"]')"

echo "── 11. Pending transactions ──"
# 3 deposits + 2 transfers = 5 triggers auto-rollup, then 1 withdraw remains
R=$(get_json "$BASE/l2/pending")
assert_eq "1 pending tx (auto-rollup fired at 5)" "1" "$(jq_val "$R" '["count"]')"

# ════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo " TEST SUITE: ZK Proofs & Rollups"
echo "═══════════════════════════════════════════════"

echo "── 12. Manual rollup ──"
STATE_ROOT_BEFORE=$(jq_val "$(get_json "$BASE/l2/balances")" '["state_root"]')
R=$(post_json "$BASE/l2/rollup" '{}')
# rollup #2 because auto-rollup already created #1
assert_contains "rollup message" "submitted to L1 block" "$R"
assert_eq "rollup tx_count (just the withdraw)" "1" "$(jq_val "$R" '["rollup"]["tx_count"]')"

PROOF_STATE_AFTER=$(jq_val "$R" '["rollup"]["proof"]["state_root_after"]')
STATE_ROOT_AFTER=$(jq_val "$(get_json "$BASE/l2/balances")" '["state_root"]')
assert_eq "proof state_root_after matches current state" "$STATE_ROOT_AFTER" "$PROOF_STATE_AFTER"

echo "── 13. Pending cleared after rollup ──"
R=$(get_json "$BASE/l2/pending")
assert_eq "pending cleared" "0" "$(jq_val "$R" '["count"]')"

echo "── 14. ZK proof structure ──"
R=$(get_json "$BASE/l2/rollups")
PROOF=$(echo "$R" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin)['rollups'][0]['proof']))")
for FIELD in state_root_before state_root_after tx_root commitment challenge response nullifier; do
  VAL=$(echo "$PROOF" | python3 -c "import sys,json; print(json.load(sys.stdin)['$FIELD'])")
  if [ ${#VAL} -ge 32 ]; then pass "proof has $FIELD (len=${#VAL})"; else fail "proof $FIELD too short" "$VAL"; fi
done
# First rollup was auto-triggered with 5 txs (3 deposits + 2 transfers)
assert_eq "proof tx_count" "5" "$(echo "$PROOF" | python3 -c "import sys,json; print(json.load(sys.stdin)['tx_count'])")"

echo "── 15. Verify valid proof ──"
R=$(post_json "$BASE/l2/verify" "{\"proof\":$PROOF}")
assert_eq "valid proof verifies" "True" "$(jq_val "$R" '["valid"]')"

echo "── 16. Verify tampered proofs ──"
# tamper challenge
TAMPERED=$(echo "$PROOF" | python3 -c "import sys,json; p=json.load(sys.stdin); p['challenge']='a'*64; print(json.dumps(p))")
R=$(post_json "$BASE/l2/verify" "{\"proof\":$TAMPERED}")
assert_eq "tampered challenge fails" "False" "$(jq_val "$R" '["valid"]')"

# tamper nullifier
TAMPERED=$(echo "$PROOF" | python3 -c "import sys,json; p=json.load(sys.stdin); p['nullifier']='b'*64; print(json.dumps(p))")
R=$(post_json "$BASE/l2/verify" "{\"proof\":$TAMPERED}")
assert_eq "tampered nullifier fails" "False" "$(jq_val "$R" '["valid"]')"

# tamper response
TAMPERED=$(echo "$PROOF" | python3 -c "import sys,json; p=json.load(sys.stdin); p['response']='c'*64; print(json.dumps(p))")
R=$(post_json "$BASE/l2/verify" "{\"proof\":$TAMPERED}")
assert_eq "tampered response fails" "False" "$(jq_val "$R" '["valid"]')"

# tamper state_root_before
TAMPERED=$(echo "$PROOF" | python3 -c "import sys,json; p=json.load(sys.stdin); p['state_root_before']='fake'; print(json.dumps(p))")
R=$(post_json "$BASE/l2/verify" "{\"proof\":$TAMPERED}")
assert_eq "tampered state_root_before fails" "False" "$(jq_val "$R" '["valid"]')"

# tamper state_root_after
TAMPERED=$(echo "$PROOF" | python3 -c "import sys,json; p=json.load(sys.stdin); p['state_root_after']='fake'; print(json.dumps(p))")
R=$(post_json "$BASE/l2/verify" "{\"proof\":$TAMPERED}")
assert_eq "tampered state_root_after fails" "False" "$(jq_val "$R" '["valid"]')"

# tamper tx_count
TAMPERED=$(echo "$PROOF" | python3 -c "import sys,json; p=json.load(sys.stdin); p['tx_count']=99; print(json.dumps(p))")
R=$(post_json "$BASE/l2/verify" "{\"proof\":$TAMPERED}")
assert_eq "tampered tx_count fails" "False" "$(jq_val "$R" '["valid"]')"

# zero tx_count
TAMPERED=$(echo "$PROOF" | python3 -c "import sys,json; p=json.load(sys.stdin); p['tx_count']=0; print(json.dumps(p))")
R=$(post_json "$BASE/l2/verify" "{\"proof\":$TAMPERED}")
assert_eq "zero tx_count fails" "False" "$(jq_val "$R" '["valid"]')"

echo "── 17. L1 chain includes rollup block ──"
R=$(get_json "$BASE/chain")
LAST_DATA=$(echo "$R" | python3 -c "import sys,json; c=json.load(sys.stdin)['chain']; print(c[-1]['data'][:10])")
assert_eq "last L1 block is rollup" "L2_ROLLUP:" "$LAST_DATA"

R=$(get_json "$BASE/validate")
assert_eq "L1 valid after rollup" "True" "$(jq_val "$R" '["valid"]')"

# ════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo " TEST SUITE: Auto-Rollup (batch_size=5)"
echo "═══════════════════════════════════════════════"

echo "── 18. Auto-rollup triggers at batch size ──"
ROLLUPS_BEFORE=$(jq_val "$(get_json "$BASE/l2/rollups")" '["count"]')
for i in 1 2 3 4; do
  post_json "$BASE/l2/transfer" "{\"from\":\"alice\",\"to\":\"bob\",\"amount\":$i}" > /dev/null
done
R=$(get_json "$BASE/l2/pending")
assert_eq "4 pending before threshold" "4" "$(jq_val "$R" '["count"]')"

R=$(post_json "$BASE/l2/transfer" '{"from":"alice","to":"bob","amount":5}')
assert_contains "5th tx triggers auto-rollup" "auto-rollup triggered" "$R"

R=$(get_json "$BASE/l2/pending")
assert_eq "pending cleared after auto-rollup" "0" "$(jq_val "$R" '["count"]')"

ROLLUPS_AFTER=$(jq_val "$(get_json "$BASE/l2/rollups")" '["count"]')
assert_eq "rollup count incremented" "$((ROLLUPS_BEFORE + 1))" "$ROLLUPS_AFTER"

echo "── 19. Sequential rollups have consistent state roots ──"
R=$(get_json "$BASE/l2/rollups")
R1_AFTER=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['rollups'][0]['proof']['state_root_after'])")
R2_BEFORE=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['rollups'][1]['proof']['state_root_before'])")
assert_eq "rollup 2 state_root_before == rollup 1 state_root_after" "$R1_AFTER" "$R2_BEFORE"

# ════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo " TEST SUITE: Edge Cases & Error Handling"
echo "═══════════════════════════════════════════════"

echo "── 20. Insufficient balance ──"
assert_status "transfer > balance returns 400" "400" "$BASE/l2/transfer" "POST" '{"from":"charlie","to":"alice","amount":999999}'
R=$(post_json "$BASE/l2/transfer" '{"from":"charlie","to":"alice","amount":999999}')
assert_contains "insufficient balance error" "insufficient balance" "$R"

echo "── 21. Self-transfer ──"
assert_status "self-transfer returns 400" "400" "$BASE/l2/transfer" "POST" '{"from":"alice","to":"alice","amount":1}'
R=$(post_json "$BASE/l2/transfer" '{"from":"alice","to":"alice","amount":1}')
assert_contains "self-transfer error" "cannot transfer to self" "$R"

echo "── 22. Zero amounts ──"
assert_status "zero deposit returns 400" "400" "$BASE/l2/deposit" "POST" '{"account":"alice","amount":0}'
assert_status "zero transfer returns 400" "400" "$BASE/l2/transfer" "POST" '{"from":"alice","to":"bob","amount":0}'
assert_status "zero withdraw returns 400" "400" "$BASE/l2/withdraw" "POST" '{"account":"alice","amount":0}'

echo "── 23. Empty rollup ──"
# pending should be 0 right now
assert_status "empty rollup returns 400" "400" "$BASE/l2/rollup" "POST" '{}'

echo "── 24. Withdraw more than balance ──"
CHARLIE_BAL=$(jq_val "$(get_json "$BASE/l2/balance/charlie")" '["balance"]')
R=$(post_json "$BASE/l2/withdraw" "{\"account\":\"charlie\",\"amount\":$((CHARLIE_BAL + 1))}")
assert_contains "withdraw > balance fails" "insufficient balance" "$R"

echo "── 25. Transfer from empty account ──"
R=$(post_json "$BASE/l2/transfer" '{"from":"nobody","to":"alice","amount":1}')
assert_contains "transfer from empty account" "insufficient balance" "$R"

echo "── 26. Invalid JSON body ──"
STATUS=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/l2/deposit" -H "Content-Type: application/json" -d 'not json')
assert_eq "invalid JSON returns 4xx" "1" "$(echo "$STATUS" | grep -c '4[0-9][0-9]')"

echo "── 27. Missing fields ──"
STATUS=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/l2/transfer" -H "Content-Type: application/json" -d '{"from":"alice"}')
assert_eq "missing fields returns 4xx" "1" "$(echo "$STATUS" | grep -c '4[0-9][0-9]')"

# ════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo " TEST SUITE: Stress Test"
echo "═══════════════════════════════════════════════"

echo "── 28. Rapid L2 transactions (50 transfers) ──"
ALICE_BEFORE=$(jq_val "$(get_json "$BASE/l2/balance/alice")" '["balance"]')
BOB_BEFORE=$(jq_val "$(get_json "$BASE/l2/balance/bob")" '["balance"]')
for i in $(seq 1 50); do
  post_json "$BASE/l2/transfer" '{"from":"alice","to":"bob","amount":1}' > /dev/null
done
ALICE_AFTER=$(jq_val "$(get_json "$BASE/l2/balance/alice")" '["balance"]')
BOB_AFTER=$(jq_val "$(get_json "$BASE/l2/balance/bob")" '["balance"]')
assert_eq "alice lost 50" "$((ALICE_BEFORE - 50))" "$ALICE_AFTER"
assert_eq "bob gained 50" "$((BOB_BEFORE + 50))" "$BOB_AFTER"

echo "── 29. Auto-rollups fired during stress ──"
R=$(get_json "$BASE/l2/rollups")
ROLLUP_COUNT=$(jq_val "$R" '["count"]')
# 3 before stress (1 auto + 1 manual + 1 auto-rollup-test), 50 txs / 5 = 10 more = 13
assert_eq "rollup count after stress" "13" "$ROLLUP_COUNT"

echo "── 30. All rollup proofs verify ──"
ALL_VALID=true
for i in $(seq 0 $((ROLLUP_COUNT - 1))); do
  P=$(echo "$R" | python3 -c "import sys,json; print(json.dumps(json.load(sys.stdin)['rollups'][$i]['proof']))")
  V=$(jq_val "$(post_json "$BASE/l2/verify" "{\"proof\":$P}")" '["valid"]')
  if [ "$V" != "True" ]; then ALL_VALID=false; break; fi
done
assert_eq "all $ROLLUP_COUNT rollup proofs verify" "true" "$ALL_VALID"

echo "── 31. Chain valid after all operations ──"
R=$(get_json "$BASE/validate")
assert_eq "L1 chain still valid" "True" "$(jq_val "$R" '["valid"]')"

CHAIN_LEN=$(jq_val "$(get_json "$BASE/chain")" '["length"]')
echo "  INFO: L1 chain length: $CHAIN_LEN blocks"

echo "── 32. Hash linkage intact across full chain ──"
R=$(get_json "$BASE/chain")
CHAIN_LEN=$(jq_val "$R" '["length"]')
LINK_OK=true
for i in $(seq 1 $((CHAIN_LEN - 1))); do
  PREV_HASH=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['chain'][$((i-1))]['hash'])")
  CURR_PREV=$(echo "$R" | python3 -c "import sys,json; print(json.load(sys.stdin)['chain'][$i]['previous_hash'])")
  if [ "$PREV_HASH" != "$CURR_PREV" ]; then LINK_OK=false; break; fi
done
assert_eq "all blocks properly linked" "true" "$LINK_OK"

echo "── 33. State root determinism ──"
ROOT1=$(jq_val "$(get_json "$BASE/l2/balances")" '["state_root"]')
ROOT2=$(jq_val "$(get_json "$BASE/l2/balances")" '["state_root"]')
assert_eq "state root is deterministic" "$ROOT1" "$ROOT2"

echo "── 34. Conservation of funds ──"
R=$(get_json "$BASE/l2/balances")
TOTAL_FUNDS=$(echo "$R" | python3 -c "import sys,json; print(sum(json.load(sys.stdin)['balances'].values()))")
# deposited: 10000+5000+2000=17000, withdrew: 500, net=16500
assert_eq "total funds conserved (17000-500=16500)" "16500" "$TOTAL_FUNDS"

# ════════════════════════════════════════════════════════════════
echo ""
echo "═══════════════════════════════════════════════"
echo " RESULTS: $PASS/$TOTAL passed, $FAIL failed"
echo "═══════════════════════════════════════════════"
echo ""

if [ "$FAIL" -gt 0 ]; then exit 1; fi
