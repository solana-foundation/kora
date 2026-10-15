# Public relayer gateway

A fee relayer for Solana apps. An app sends the gateway its transaction, unsigned. It gets the
transaction back with our fee payer set and a USDC payment appended. The user signs once, the
gateway sends it, and the user never needs SOL.

Every app registers and is configured in a console. There is no anonymous use: each request names
an `app`. The user pays the network cost plus the app's markup (default: the network cost again,
so twice the cost). The app receives 80 percent of the markup and NoirWire 20 percent, both paid
inside the user's own transaction, so the gateway never holds anyone's money. On devnet charging
is off: no payment is appended and no USDC is needed. On mainnet the same code starts charging.

Plain Node ESM, Fastify, Postgres. It talks to a separate **public Kora**
(`../kora/kora.public.toml`) that only the gateway can reach. It is not the Kora behind NoirWire's
wallet app, which is untouched. One deployment per network, each with its own database; an app
belongs to the deployment it was created on. The TypeScript client is in `../sdk`.

## Request flow

1. `POST /v1/prepare` with `{ app, transaction, user }`.
   - `app` is required. Unknown: 404 `unknown_app`. Paused: 403 `app_paused`. On a charging
     deployment with no payout wallet: 403 `app_not_ready`.
   - `transaction` is base64, unsigned, legacy or v0. Its fee payer must be the address from
     `GET /v1/config`. `user` is the wallet that pays; it must be a required signer.
   - The gateway checks the transaction (see "Safety rules"), simulates it, prices it, appends the
     payment and answers `{ transaction, quote, expiresAt }`. `quote` has `networkCostMicroUsdc`,
     `markupMicroUsdc`, `appMicroUsdc`, `noirwireMicroUsdc` and `totalMicroUsdc` as strings.
2. The app has the user sign exactly that transaction.
3. `POST /v1/send` with `{ transaction }`. The gateway finds the quote by hashing the message it
   was sent, so any change to the message finds no quote. Every limit then applies to the user and
   app that quote was made for, whatever signer slot they occupy. It checks every signature except
   the fee payer's, reserves the worst case (below), asks Kora to sign, stores the signed bytes,
   broadcasts them itself and answers 200 `{ signature, status }`.
4. In the background the gateway watches the signature and records `landed`, `failed` (with the
   lamports the fee payer paid) or `expired`.

Transaction states: `sending` (admitted, reserved), `signed` (Kora signed; the exact bytes are
stored; no node has accepted them yet), `sent` (a node accepted the broadcast), then `landed`,
`failed` or `expired`; `rejected` (Kora refused; nothing was spent) and `unknown` (Kora's answer
was unclear).

If Kora or the broadcast gives an unclear answer, or a node refuses the broadcast, `send` answers
`202 { outcome: "unknown", messageHash, signature? }`. Nothing is re-signed and no other fee payer
is tried: the first may have signed, and two signed copies with different fee payers could both
land. A row that is `signed` but not `sent` is rebroadcast with the identical stored bytes, by a
repeat `send` of the same message or by the background sweep. Broadcast attempts are serialized
per message by a lease on the row (30 seconds), so concurrent repeats reach the network once; the
others answer unknown. One node refusal never releases a signed row, because an earlier attempt
may have landed: it stays reserved until its blockhash is gone and the signature is confirmed
absent, and the sweep then settles it as `expired`. A row with no signature (Kora's answer never
came) is released the same way once its blockhash is gone: Kora's `signTransaction` signs and
returns without broadcasting, so such a transaction can land only if someone holds the signed
bytes, and nobody can land it after its blockhash expires. A repeat `send` of anything else
returns the recorded outcome.

If the app's payout wallet, markup or program list changed after `prepare`, `send` answers 409
`app_settings_changed` and the caller prepares again.

## Routes

Amounts are strings of integers (`...MicroUsdc` has 6 decimals, `...Lamports` is lamports). Errors
are `{ error, code }`; `code` is stable, `error` is for people. CORS is open to any origin; owner
routes use `Authorization: Bearer <token>`, never cookies.

| Route | What it does |
| --- | --- |
| `GET /v1/health` | `{ ok, network, feePayer: { state: ready/degraded/unavailable, balanceLamports, checkedAt } }` |
| `GET /v1/config` | `{ network, feePayer, usdcMint, charge, markup: { defaultBps, maxBps }, split: { appBps, noirwireBps } }` |
| `POST /v1/prepare`, `POST /v1/send` | the relay flow above |
| `POST /v1/auth/challenge` | `{ wallet }` to `{ nonce, message, expiresAt }` |
| `POST /v1/auth/session` | `{ wallet, nonce, signature }` (base58 over the UTF-8 message) to `{ token, expiresAt }` |
| `DELETE /v1/auth/session` | sign out, 204 |
| `GET /v1/me/apps`, `POST /v1/apps` | list the wallet's apps; create one from `{ name }` |
| `GET`, `PATCH /v1/apps/:id` | read; change `name`, `markupBps`, `allowedOrigins`, `allowedPrograms`, `dailyCapLamports`, `paused` |
| `POST /v1/apps/:id/payout/challenge`, `PUT /v1/apps/:id/payout` | change the payout wallet with a fresh signature |
| `GET /v1/apps/:id/stats?window=today\|30d` | totals and one row per UTC day |
| `GET /v1/apps/:id/activity?before=&limit=` | recent transactions, newest first, cursor paged |

Another owner's app answers 404. A wallet can own at most 20 apps.

### Sign-in and sessions

A wallet signs a plain message that names the network, a single-use nonce and the time, and says
it is not a transaction. The signature opens a bearer session for 12 hours; only the hash of the
token is kept. Sessions and challenges live in memory, so a restart signs everyone out.

### Payout changes

Changing the payout wallet needs the owner's fresh signature over a message that names the app and
the new wallet, on top of the session. The new wallet's USDC account must exist. Earnings always go
to the wallet on record at the time of `prepare`.

### App settings the gateway enforces

- `allowedOrigins`: when non-empty, a request carrying another `Origin` is refused 403
  `origin_not_allowed`. A request without `Origin` (a server) is allowed. This is hygiene, not
  protection: any script can send any `Origin`.
- `allowedPrograms`: when non-empty, every top-level instruction's program must be listed or be
  System, SPL Token, Token-2022, Associated Token Account or ComputeBudget (403
  `program_not_allowed`).
- `dailyCapLamports`: the most the app's transactions may cost the fee payer per UTC day, landed or
  not (429 `app_daily_cap`). Bounded by `APP_DAILY_CAP_MAX_LAMPORTS`. This is a shared public
  quota, not a private one: the app id is public, so anyone who has it can consume the cap. On
  mainnet that costs them real USDC for every landed transaction; failed ones are bounded per
  wallet and per IP by the loss budgets and the failure ratio. This is also why allowed origins are
  hygiene only: a script can send any `Origin`, so the list keeps honest browsers on the right
  pages but does not stop anyone who has the id.
- `paused`: refuses `prepare` and `send`.

## Money

For network cost `B` (micro-USDC) and the app's markup `m` in basis points: `M = ceil(B * m / 10000)`,
NoirWire receives `S = ceil(M / 5)` and the app receives `C = M - S`. The user pays `B + M` as one
`TransferChecked` of `B + S` to NoirWire's payment USDC account and, when `C` is above zero, one of
`C` to the app's payout USDC account.

`B` is measured: signature fees (5,000 lamports per signature), plus the priority fee, plus every
lamport the fee payer loses in simulation (rent included), converted at a fresh Pyth SOL/USD price
and rounded up. A stale price refuses the request.

## Safety rules

Kora only recognises system and token instructions, so its `allowed_programs = "All"` is not safe
on its own. The gateway enforces these, as pure functions with tests (`src/policy.mjs`):

1. **The fee payer is never handed to a program.** After resolving every address lookup table, the
   fee payer's address and every token account it owns may appear in an instruction's accounts only
   as the funder of a top-level System `CreateAccount` or an Associated Token Account `Create` /
   `CreateIdempotent`. It must be the message's fee payer, appear nowhere else, and no token
   account may be initialised with it as owner. Lookup tables must be active.
2. **Cost is measured, then charged.** The simulation (signatures not verified, blockhash not
   replaced) must succeed, and may cost the fee payer at most `MAX_FEE_PAYER_OUTFLOW_LAMPORTS`.
3. **Caps** on signatures, serialized size, compute unit price and limit. No durable nonces.
   The default price cap is 0: no priority fee, so a failed transaction costs only its signature
   fees. Separately, the total priority fee of one transaction (price times limit) may not exceed
   `MAX_PRIORITY_FEE_LAMPORTS` (default 0, never above 10,000), whatever the price and limit caps
   allow. A transaction that sets a price must also set a compute unit limit, so the fee measured
   before the payment instructions are appended is the fee of the final message.
4. **Exact payments.** The gateway builds the transfers itself and the message hash pins them.
5. **One quote, one use, one hash.** A quote is keyed by the sha256 of the final message and
   expires at the earlier of `QUOTE_TTL_SECONDS` and the blockhash lifetime. What Kora returns
   must be the same message, signed by the fee payer, before it is stored and broadcast.
6. **The worst case is reserved at admission, atomically.** A transaction that passes simulation
   and then fails on chain costs the fee payer its fee and costs the sender nothing, so a limit
   that only counts settled losses lets concurrent sends all slip under it. At `send` the gateway
   reserves the worst case in one step (a single Postgres transaction under an advisory lock) against: the global daily ceiling,
   the user wallet, the client IP, the app's loss budget (each counting the fees a failed
   transaction would cost, `loss_lamports`), and the app's `dailyCapLamports` (counting everything
   it sponsors, fees plus simulated outflow, landed or not). An open send counts at its worst case,
   a failed one at its actual fee, a landed one at nothing against the loss budgets, an expired or
   rejected one at nothing anywhere. A failed transaction reverts its account creations, so only
   fees can be lost to it. Over a limit: 429 (`wallet_loss_budget`, `ip_loss_budget`,
   `app_loss_budget`, `app_daily_cap`) or 503 (`loss_ceiling`).
7. **Failure griefing.** A wallet with at least `FAILURE_BLOCK_MIN_SENDS` landed-or-failed sends in
   the last 24 hours whose failed share is above `FAILURE_BLOCK_RATIO` is refused (429
   `wallet_failure_ratio`). Expired sends do not count. These checks read the ledger and run
   before any RPC call; the ledger records the client IP of each send.
8. **Reconciliation must be healthy.** The sweep pages through every `signed`, `sent` and
   `unknown` row (a few stuck ones never hide the rest), rebroadcasts signed ones, and settles the
   others by the chain. Sends are refused with 503 `reconciliation_unhealthy` while more than
   `RECONCILIATION_STALE_LIMIT` rows are still open ten minutes after they were created.
9. **Abuse limits.** A global ingress rate limit comes first, then per IP and per user wallet limits
   (the quote's user for `send`), all before any RPC call. Every in-memory map (quotes, rate
   limits, challenges, sessions) has a hard cap on live entries; when full, new work is refused
   with 503 `overloaded`.

The client IP is only as good as the proxy in front. `TRUSTED_PROXY_HOPS=1` trusts exactly one
hop: the client is the right-most `X-Forwarded-For` entry, and anything to its left is ignored.
This is correct only if that proxy appends the connecting address rather than passing the header
through; verify it on the deployment (send a request with a forged `X-Forwarded-For` and check the
address the ledger records). With `0` the header is ignored.

Kora's own checks (payment to the public payment address at margin 0) are the second gate.
`force_sig_verify` stays false in `../kora/kora.public.toml`: Kora would simulate with signature
verification before signing, while the fee payer's signature slot is still empty, and every request
would fail. The gateway verifies every other signature itself.

## Configuration

All settings come from the environment and are checked at start. The service refuses to start on
a bad value, and never prints one.

| Variable | Default | Meaning |
| --- | --- | --- |
| `NETWORK` | none, required | `devnet` or `mainnet` |
| `RPC_URL` | none, required | Solana RPC |
| `KORA_URL` | none, required | the public Kora. Keep it private |
| `KORA_API_KEY` | none, required | sent as `x-api-key` |
| `KORA_HMAC_SECRET` | none, required | signs each request (see below) |
| `FEE_PAYER` | none, required | the public Kora's fee payer address |
| `PAYMENT_USDC_ACCOUNT` | none, required when charging | NoirWire's USDC account that receives `B + S` |
| `USDC_MINT` | per network | USDC mint |
| `CHARGE` | `off` on devnet, `on` on mainnet | mainnet refuses `off` |
| `DATABASE_URL` | none, required | Postgres, with the tables already made (see below). The gateway never changes the schema |
| `PORT` | `8080` | |
| `TRUSTED_PROXY_HOPS` | `0` | `1` behind exactly one proxy that appends the client address |
| `MAX_SIGNATURES` | `3` | required signatures, fee payer included (at most 8) |
| `MAX_TX_BYTES` | `1232` | serialized size, before and after the payment (at most 1232) |
| `MAX_COMPUTE_UNIT_PRICE` | `0` | micro-lamports per compute unit. 0 refuses any priority fee |
| `MAX_PRIORITY_FEE_LAMPORTS` | `0` | ceiling on the total priority fee of one transaction (at most 10000) |
| `MAX_COMPUTE_UNIT_LIMIT` | `1400000` | compute units |
| `MAX_FEE_PAYER_OUTFLOW_LAMPORTS` | `2200000` | most the fee payer may lose to one simulated transaction (one account creation) |
| `RATE_LIMIT_GLOBAL_PER_MINUTE` | `3000` | every request, ahead of the per-key limits |
| `RATE_LIMIT_IP_PER_MINUTE` | `60` | relay and read routes per IP |
| `RATE_LIMIT_USER_PER_MINUTE` | `20` | `prepare` and `send` per user wallet |
| `RATE_LIMIT_REGISTER_PER_MINUTE` | `10` | sign-in and app changes per IP |
| `QUOTE_TTL_SECONDS` | `60` | at most 90; also capped by the blockhash lifetime |
| `DAILY_LOSS_CEILING_LAMPORTS` | `100000000` | global worst-case fee loss per UTC day (0.1 SOL) |
| `WALLET_DAILY_LOSS_LAMPORTS` | `50000` | per user wallet |
| `IP_DAILY_LOSS_LAMPORTS` | `200000` | per client IP |
| `APP_DAILY_LOSS_LAMPORTS` | `100000` | per app |
| `APP_DAILY_CAP_DEFAULT_LAMPORTS` | `10000000` | a new app's `dailyCapLamports` |
| `APP_DAILY_CAP_MAX_LAMPORTS` | `500000000` | the most an app may set |
| `FAILURE_BLOCK_MIN_SENDS` | `5` | settled sends in 24 hours before the failure ratio applies |
| `FAILURE_BLOCK_RATIO` | `0.2` | failed share (0 to 1) above which a wallet is refused |
| `RECONCILIATION_STALE_LIMIT` | `20` | open rows older than ten minutes before sends are refused |
| `FEE_PAYER_LOW_LAMPORTS` | `30000000` | health state `degraded` below this balance |
| `FEE_PAYER_EMPTY_LAMPORTS` | `5000000` | health state `unavailable` at or below this balance |

Kora authentication, as Kora checks it: header `x-api-key`; header `x-timestamp` (unix seconds,
within Kora's `max_timestamp_age`); header `x-hmac-signature`, the hex HMAC-SHA256, keyed with the
HMAC secret, of the timestamp followed directly by the exact request body.

Rate limits, quotes, challenges and sessions are kept in memory, so run one instance. The ledger
and the apps are in Postgres.

## Database

In production the tables live in the NoirWire Supabase project, schema `relayer`, made by the
migration `supabase/migrations/20261015120000_relayer.sql` in the `app-noirwire` repo. The gateway
connects as the role `relayer_gateway`, which may read, add and change rows there and nothing
else. Once, after the migration is pushed, give the role a login in the SQL editor:

```sql
alter role relayer_gateway with login password '<new password>';
```

Then point `DATABASE_URL` at the transaction pooler (port 6543) as that role:

```
postgresql://relayer_gateway.<project ref>:<password>@<pooler host>:6543/postgres?uselibpqcompat=true&sslmode=require
```

The role's own `search_path` is `relayer`, so the gateway's queries name no schema. Its queries
are unnamed statements, which the transaction pooler needs. For a local Postgres, `src/schema.sql`
is the same tables in the default schema: apply it once with `psql "$DATABASE_URL" -f src/schema.sql`.

## Run locally

Needs Node 22 and a Postgres.

```sh
npm ci
export NETWORK=devnet RPC_URL=https://api.devnet.solana.com
export KORA_URL=http://localhost:8081 KORA_API_KEY=... KORA_HMAC_SECRET=...
export FEE_PAYER=<public key of the Kora signer>
export DATABASE_URL=postgres://postgres:postgres@localhost:5432/gateway
psql "$DATABASE_URL" -f src/schema.sql   # once
npm start
```

Run the public Kora from `../kora/kora.public.toml` with the devnet changes described at the top
of that file.

## Tests

```sh
npm test                       # node:test, no network and no database
node --check src/*.mjs test/*.mjs
```

The tests cover the pure functions (money, fee payer exposure, signer rules, caps, payments for
legacy and v0 messages, message hashing, signature checks, sign-in and payout messages, config) and
every route through Fastify's `inject` with an in-memory store and fake RPC, Kora and price. The
Postgres store follows the same contract as the in-memory one but is not exercised by the suite.

## Before mainnet

This service stays a devnet relayer until the mainnet checks are done: a legal read, the
measured failure loss under adversarial load, an independent review of this service and an
end-to-end fork test against a CPI drain.
