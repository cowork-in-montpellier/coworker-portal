## Auth

### Login

```bash
# Login — success
curl -s -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "alicepass123"}' | jq .

# Expected response:
# { "token": "eyJ0eXAiOiJKV1QiLCJhbGci..." }

# Login — wrong credentials (returns 401)
curl -i -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "alice", "password": "wrong"}'

# Using the token on a protected route
curl -s http://localhost:3000/api/some-protected-route \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGci..."
```


## SumUp sandbox setup — what you need:

### Setup

1. Log into your SumUp account → Developer Settings → Sandboxes tab → create a sandbox merchant account. It generates a separate merchant account (with its own SUMUP_MERCHANT_CODE) that processes no real money.
2. API keys with prefix sk_test_ are used against SUMUP_BASE_URL=https://api.sumup.com (same URL, the sandbox/live distinction comes from the key prefix and the merchant account, not a different domain).

Test cards (all use any future expiry + any 3-digit CVV):

┌──────────────────────┬──────────────────────────────────────────┐
│       Scenario       │               Card number                │
├──────────────────────┼──────────────────────────────────────────┤
│ Visa — success       │ 4200000000000091                         │
├──────────────────────┼──────────────────────────────────────────┤
│ Mastercard — success │ 5200000000000007                         │
├──────────────────────┼──────────────────────────────────────────┤
│ Visa — 3DS challenge │ 4200000000000042                         │
├──────────────────────┼──────────────────────────────────────────┤
│ Failure (any card)   │ Use amount 11.00, 42.01, 42.76, or 42.91 │
└──────────────────────┴──────────────────────────────────────────┘


### Configuration

App configuration
```
SUMUP_ENABLED=true
SUMUP_API_KEY=<SUMUP_API_KEY>
SUMUP_MERCHANT_CODE=<SUMUP_MERCHANT_ID>
```