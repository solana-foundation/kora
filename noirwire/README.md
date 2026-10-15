# Fee relayer and SOL refill job

Two small services that let a wallet app's users transact without ever holding SOL.

Run by [NoirWire](https://noirwire.com) for its wallet at [app.noirwire.com](https://app.noirwire.com).
Updates on X: [@_Noirwire_](https://x.com/_Noirwire_). Contact: ph1l1ph@proton.me.

This repository is a fork of [solana-foundation/kora](https://github.com/solana-foundation/kora).
Kora's own code is untouched; everything NoirWire adds lives in this `noirwire/` folder, so
upstream releases merge cleanly (see `docs/updating-from-upstream.md`). You are welcome to run
your own relayer from it. NoirWire's hosted relayer is not a public endpoint: it only answers
NoirWire's own server.

- **`kora/`**: [Kora](https://github.com/solana-foundation/kora), the Solana Foundation fee
  relayer, as a pinned Docker image plus its configuration. It co-signs a user's transaction
  as fee payer after checking that the transaction pays it in USDC.
- **`refill/`**: a job that runs once an hour, turns part of the collected USDC back into
  SOL and tops the fee payer up. Without it the fee payer runs dry and someone has to send it
  SOL by hand.

Everything here lives in this one folder; all paths below are relative to it.

```
kora/        Dockerfile, kora.toml, signers.toml (v2.2.0-beta.8, the default)
             Dockerfile.stable, kora.stable.toml (v2.0.5, the audited alternative)
refill/      the refill job (plain Node, two dependencies) and its tests
gateway/     the public relayer gateway (Fastify, Postgres) for registered apps; see gateway/README.md
sdk/         @noirwire/relayer, the TypeScript client for the gateway; see sdk/README.md
             kora/kora.public.toml and Dockerfile.public are the separate Kora only it may call
scripts/     check-deploy.sh (a deployed Kora), check-upstream.sh (pinned image against upstream releases)
.railway/    railway.ts: both Railway services as code
deploy.md    the deployment sequence, command by command
docs/        updating-from-upstream.md, and the record of the fork test this is built on
```

## The operating rule

> **WARNING. This Kora is ONLY safe behind a caller that verifies the whole transaction and
> enforces its own minimum payment, rent included, for every transaction in which the fee
> payer funds a token account.**
>
> Here that caller is the wallet app's server route, and it is the only one: it holds the API
> key and the HMAC secret, checks each transaction against the template the app built and the
> user's signature on it, refuses to forward a relayer-funded account creation that does not
> pay at least the rent plus the fee, and sets the price the user sees.
>
> - Never expose this Kora service directly to browsers or to any other client.
> - Never give the API key and HMAC secret to a caller that does not enforce the same rule.
> - Do not send real funds through it until the caller's own check has passed against the
>   deployed route (`deploy.md`, "Before real funds").
> - A caller must **never fail over to another relayer after an ambiguous signing response**
>   (a timeout, a dropped connection): the first relayer may have signed, and two signed
>   copies of one transaction with different fee payers can both land.
> - If you reuse this repository without such a caller, use the stable alternative with
>   account opening off (`kora/Dockerfile.stable`), and accept what it cannot do.

## Stable or pre-release

Production runs Kora **`v2.2.0-beta.8`, a pre-release**, pinned by digest in `kora/Dockerfile`.
The audited stable release, `v2.0.5`, is kept as a named alternative. This is the reasoning.

**Why not stable.** Two things, both observed on a mainnet fork:

1. `v2.0.5` cannot relay a Jupiter Lend withdrawal at all. It rebuilds the withdrawal's inner
   SPL `Burn` with two accounts and then requires three, and refuses the transaction with
   "Instruction doesn't have the required number of accounts". Earn withdrawals must always
   work without the user holding SOL, so this alone rules it out here.
2. `v2.0.5` charges no rent for a relayer-funded account creation when the account already
   exists at the moment Kora signs. Someone who closes the account between signing and
   landing gets it re-created at the relayer's expense and keeps the rent (2,039,280 lamports
   per transaction, measured; `docs/fork-test-runbook.md` section 0). `v2.2.0-beta.8` charges
   the rent whether or not the account exists.

**What is given up.** The pre-release is newer than Kora's audit. The audit (Runtime
Verification, `audits/20251119_runtime-verification.pdf` upstream) is commit based:
upstream's `audits/AUDIT_STATUS.md` names the audited-through commit
`8c592591debd08424a65cc471ce0403578fd5d5d` and says commits after it "are considered unaudited
until a new audit or mitigation review updates this file". The code that fixes both problems
above is after that commit. Upstream also notes that the `solana-keychain` package Kora uses
has not been audited, in either release.

**What makes that acceptable.** Kora is the second line here, not the first, and what it can
lose is small and watched:

- Kora is never exposed except to one caller, which verifies the whole transaction against
  its own template and the user's signature, and enforces its own minimum payment including
  rent. An unaudited bug in Kora's validation has to get past that first.
- Every request needs both an API key and an HMAC signature.
- ComputeBudget is not an allowed program, so no caller can set a priority fee for the
  relayer to pay: a transaction that fails after signing costs 10,000 lamports, not more.
- Exactly two signers (`max_signatures = 2`), and at most one account creation per
  transaction (`max_allowed_lamports = 2200000`).
- Each fee payer is a key that owns nothing but a small SOL float (0.1 SOL). The collected
  USDC is in a different wallet whose key Kora never sees.
- The refill job cannot turn a drained float into drained revenue: at most 0.5 SOL per fee
  payer per day, and it stops refilling and raises an alarm when the fee payer loses SOL on
  transactions that did not pay for themselves.
- The caller never fails over to another relayer after an ambiguous signing response.

**The upgrade rule.** Move to the next **audited stable** release as soon as one contains both
fixes (a Lend withdrawal relays; rent is charged for a create whatever the account's state at
signing). `docs/updating-from-upstream.md` has the checks that decide it, and
`scripts/check-upstream.sh` shows when upstream has released something.

| | `kora/Dockerfile` + `kora.toml` (default) | `kora/Dockerfile.stable` + `kora.stable.toml` (alternative) |
| --- | --- | --- |
| Kora | `v2.2.0-beta.8`, pre-release | `v2.0.5`, stable |
| Audit | **newer than the audit** | inside the audited code |
| Earn (Jupiter Lend) withdrawal | relays | **refused** |
| Account opening | on; rent always charged by Kora | off (on is unsafe without the caller's rule: rent not charged for an existing account) |
| `max_allowed_lamports` | 2,200,000 | 50,000 |

To run the alternative, build the service from `Dockerfile.stable` (in Railway: the
`RAILWAY_DOCKERFILE_PATH` variable, or `dockerfilePath` in `.railway/railway.ts`) and put the
payment wallet's address in `kora.stable.toml`. On the default, the app must add a create
instruction only when the account is really missing: the pre-release charges rent for it
either way.

Both configs were checked with their own image's `kora config validate`.

## The two keys, and why they are separate

| Wallet | Signs | Holds | Where its secret lives |
| --- | --- | --- | --- |
| **Fee payer** | every relayed transaction, inside Kora | SOL only, about 0.1 | the Kora service (`KORA_PRIVATE_KEY`) |
| **Payment wallet** | only the refill job's swap and transfer | the USDC users pay, plus 0.01 SOL | the refill service (`PAYMENT_WALLET_PRIVATE_KEY`) |

Kora signs any transaction that passes its checks, so its key must own nothing but a small
SOL float: that float is all a signing bug can lose. The USDC goes to a different wallet
whose key Kora never sees. The refill job, in turn, never sees the fee payer's key. It only
knows the fee payer's public address, pinned in `FEE_PAYER`, and that is the only address it
can send SOL to.

Both must be new keys used for nothing else.

## How the money flows

```
user --USDC--> payment wallet --part of the USDC--> Jupiter --SOL--> payment wallet --SOL--> fee payer
                    |                                                                          |
                    +-- the rest stays: revenue                              pays network fees and rent
```

1. Every relayed transaction carries a USDC payment from the user to the payment wallet.
2. The fee payer spends SOL: 10,000 lamports per transaction, plus rent when it opens an account.
3. When the fee payer is at or below 0.03 SOL, the refill job swaps just enough USDC to bring
   it back to 0.1 SOL and sends the SOL over.
4. Whatever USDC is not needed for that accumulates in the payment wallet. Withdraw it by
   hand, leaving enough for the next refills (a refill costs about 10 USDC at 150 USD per SOL).
   A withdrawal counts as a swap attempt for 24 hours and blocks swaps for three minutes.

## The numbers

| Setting | Value | Meaning |
| --- | --- | --- |
| Fee payer target | 0.1 SOL | what a refill brings it back to; also the most it is ever refilled to |
| Refill threshold | 0.03 SOL | at or below this, the job swaps (about 70 percent of the float spent) |
| Payment wallet reserve | 0.01 SOL | kept back for the job's own fees |
| Kora margin | 0.1 | Kora requires network cost x 1.1; the app may charge more |
| `max_allowed_lamports` | 2,200,000 | admits exactly one account creation per transaction (50,000 on the stable alternative, where none fits) |

What one transaction costs, from the measured relayer costs, at an illustrative 150 USD per SOL
(check the live price; these scale with it):

| Transaction | Relayer spends | Kora requires (x 1.1) | User pays (set by the app) |
| --- | --- | --- | --- |
| Plain send, Earn deposit or withdrawal | 10,000 lamports = 0.0015 USD | 11,055 lamports = 0.0017 USD | 2 x network cost = 0.0030 USD |
| USDC send that opens the recipient's account | 2,049,280 lamports = 0.307 USD | 2,254,263 lamports = 0.338 USD | rent x 1.1 = 0.338 USD |
| Tracker send that opens a Token-2022 account | 2,146,720 lamports = 0.322 USD | 2,361,447 lamports = 0.354 USD | rent x 1.1 = 0.354 USD |

The 0.07 SOL
between refills pays for about 7,000 plain transactions or about 34 account openings. The user-facing price is the app server's decision, not a Kora setting: Kora accepts
any payment at or above what it requires.

## The refill job

Runs once and exits. Each run, in this order:

1. **Reads** the fee payer's SOL, the payment wallet's SOL and USDC, and the reference SOL
   price (Pyth's SOL/USD account, through the same RPC).
2. **Reconciles** the fee payer's last 15 minutes, transaction by transaction. If that looks
   like a drain, or cannot be proven not to be one, the run **stops here, moves nothing** and
   exits 4 (see "Monitoring").
3. **Step 1, always:** if the payment wallet holds SOL above its reserve and the fee payer is
   below target, it transfers the excess, never taking the fee payer above the target. This is
   what finishes a previous run that swapped and then died. Transfers under 0.001 SOL are
   skipped: not worth the fee.
4. **Step 2:** if the fee payer is still at or below the threshold, it asks Jupiter's Swap API
   v2 for a USDC to SOL order with the payment wallet as taker, **from market makers only**,
   sized to reach the target plus the reserve, bounded by `MAX_USDC_PER_RUN` and by the
   balance minus `USDC_FLOOR`.
5. **Guards the order before signing.** Any failure is a refusal; nothing is signed.
6. **Submits** through Jupiter's `/swap/v2/execute` (the maker's signature is collected there)
   and confirms on chain by polling `getSignatureStatuses` for at most 90 seconds. The swap
   counts only when a confirmed transaction carries this wallet's signature. Anything else is
   **unknown**: the run exits 3 and never retries.
7. **Step 3:** the same transfer as step 1, on fresh balances.

### Why market makers only

Jupiter has four routers. Three are aggregators, whose transaction carries its own slippage
limit inside a router-specific instruction; a quote in JSON plus one simulation does not prove
that limit matches the quote, and a pool can move after the simulation. The fourth, JupiterZ,
is a request-for-quote: a market maker commits to an exact input and output, both written in
the one `fill` instruction the taker signs, and the program fills at exactly those amounts or
fails. The job asks for that router only (`excludeRouters=metis,dflow,okx`), refuses anything
else that comes back, decodes the fill instruction and requires its input to equal the quote
and its output to be at least the quote's guaranteed minimum. A transaction it cannot decode
is not signed. The maker also pays the network fee, so the payment wallet's reserve is only
spent on its own transfers.

The cost: if no market maker quotes, there is no swap that run (`no_quote` or `not_built`,
exit 2) and the next run tries again. In read-only tests against live orders, makers quoted
USDC to SOL from 0.5 USDC up.

### The guard

- **The order** is USDC to SOL, exact-in, a market-maker order, for exactly the amount asked,
  for this wallet, not expired.
- **The operator's bound, `MAX_USDC_PER_SOL`.** Required, no default. The order may not pay
  more USDC per SOL than this, whatever any price source says; it is checked on the quote and
  again on what the simulation delivers. If the reference price itself is above the bound the
  job refuses (`price_above_bound`) until you raise it. Set it to about twice the current
  price and revisit it.
- **The reference price** is Pyth's SOL/USD price account, decoded by the job: owned by Pyth's
  receiver program, carrying the SOL/USD feed id, fully verified, at most 120 seconds old,
  confidence within 1 percent. It does not depend on Jupiter. The order's guaranteed minimum
  must be within `MAX_SLIPPAGE_BPS` of it. With no usable price there is no swap. Two honest
  limits: it is a USD price and the swap pays USDC (treated as equal), and it is read through
  your RPC, so an RPC that lies can lie about it too. That is what the operator's bound is for.
- **The transaction** contains compute-budget instructions and exactly one fill, no address
  lookup tables, and is paid for by the maker. No other program may be called at the top
  level, which refuses every stray transfer, approval, account closure and durable nonce.
- **The simulation** must: debit at most the quoted USDC; deliver at least the guaranteed
  minimum as native SOL, and at least what the operator's bound requires; debit, close or
  re-own no other token account of the wallet; leave the USDC account open and under the
  wallet's control; leave the wallet an ordinary System account.

### The fee payer must be proven

`FEE_PAYER` is the only address the job sends SOL to, and a mistyped but valid address would
receive it. So before any transfer the chain must show that it is the relayer:

- it exists and is an ordinary System account, and
- among the newest 100 transactions touching the payment wallet's USDC account, at least one
  was paid for by `FEE_PAYER` and paid USDC into this wallet.

Before the fee payer's first relayed transaction no such history exists. For that first
refill only, set `FEE_PAYER_UNSEEN_OK` to the same address as `FEE_PAYER`. Remove it after the
first relayed transaction. `scripts/check-deploy.sh` asks the running Kora which key it signs
with and compares it with the `FEE_PAYER` you intend to set; run it before the first refill.

### Limits, all enforced from chain state

There is no database and no local file. A restart, a redeploy or a second copy of the job
cannot reset anything, because every limit is counted from the ledger, read at `confirmed`.

| Limit | Where | How it is counted |
| --- | --- | --- |
| Swaps per 24 hours | `MAX_RUNS_PER_DAY` (default 6) | transactions **the payment wallet signed** in the last 24 hours that touch its USDC account, landed or failed |
| USDC per swap | `MAX_USDC_PER_RUN` (default 15) | the order amount |
| USDC per SOL | `MAX_USDC_PER_SOL` (**required**) | the order's and the simulation's price |
| USDC never spent | `USDC_FLOOR` (default 1) | balance minus floor |
| SOL into the fee payer per 24 hours | **constant, 0.5 SOL** (`MAX_LAMPORTS_TO_FEE_PAYER_PER_DAY` in `src/plan.mjs`) | the fee payer's balance gain in every transaction the payment wallet signed in the last 24 hours |
| Fee payer balance after a refill | `TARGET_SOL` (default 0.1, at most 0.5) | every transfer is cut to target minus current balance |
| Quiet time before a swap | constant, 450 slots (about three minutes) | slots since the payment wallet's newest swap attempt that this run did not send (plain transfers do not count) |

`MAX_USDC_PER_RUN` defaults to 15: a refill buys at most 0.09 SOL (0.1 target plus the 0.01
reserve, from empty), which costs 13.5 USDC at 150 USD per SOL. 15 covers a full refill up to
about 165 USD per SOL; above that a refill simply takes two runs. Raise it if SOL is dearer.

The 0.5 SOL daily ceiling is deliberately not a setting. If someone finds a way to make the
fee payer burn SOL, refills must not turn that into a way to burn the collected USDC as well.

History is paged back to the 24-hour boundary, at most 5,000 transactions naming the payment
wallet, and only the ones it signed are counted, so dust sent to it by strangers changes no
counter. Past 5,000 in a day the job refuses (`cannot_count`): stalling is the safe side. A
transaction the RPC cannot return is unknown history: it is never counted as the wallet's and
never skipped. The run refuses (`history_unreadable`) and the next scheduled run reads again.

### Two runs at once, and unknown outcomes

- Railway does not start a cron run while the previous one is still running: its docs say
  the new run is skipped. The job also gives itself a hard deadline of 8 minutes, shorter
  than the 10-minute schedule: every RPC call is cut off after 20 seconds, every HTTP call
  after 10 or 30, every wait is bounded, no leg starts that could not finish in time, and a
  watchdog ends the process at 9 minutes with exit 3.
- Balances and the day's counters are read again immediately before each signature. If they
  changed while an order was being checked, the swap is abandoned (`state_changed`, or the cap
  that now applies). The reference price is read and validated again at the same moment, and
  the order must still pass against the fresh value and against `MAX_USDC_PER_SOL`.
- **Never run a second copy of the job by hand while the scheduled service is enabled.**
  Nothing on chain can stop two copies that read the same confirmed state at the same instant
  from each signing a refill, and the job has no storage for a lock. Railway's cron runs one
  execution at a time and skips the next while one is still running (its docs; not tested
  here), so the scheduled service alone cannot do this; a manual `node src/main.mjs` or a
  second service for the same fee payer can. (Refill services for different fee payers that
  share the wallet are meant to exist; their schedules are shifted for the same reason.) The
  worst case is bounded: the fee payer ends one refill above its
  target (at most 0.2 SOL instead of 0.1, and one extra swap of at most `MAX_USDC_PER_RUN`).
  That stays inside the daily SOL ceiling and the daily swap cap, both of which count it
  afterwards, and the money only moves between the operator's own two wallets. To run it by
  hand, disable the cron service first, or use `DRY_RUN=1`.
- An unknown outcome leaves no trace the job could read later: a swap that has not landed is
  not on chain. The quiet-time rule covers what has landed. For the rest: **after an exit
  code 3, do not run the job by hand for three minutes.** By then the order has expired (a
  market-maker fill lives about a minute) and the next scheduled run sees the truth. No
  persistent volume is used; the job keeps no state at all.

### Output and exit codes

One JSON line per run on stdout: balances before and after, the reference price, the
reconciliation totals, what was transferred and swapped, signatures. It names the operator's
own two public addresses (fee payer and payment wallet) and nothing else: no user address, no
key, no raw environment value.

| Exit | `outcome` | Meaning | What to do |
| --- | --- | --- | --- |
| 0 | `nothing_to_do`, `done`, `dry_run` | fine | nothing |
| 1 | `failed` | something was sent and the chain says it did not happen, or a read failed | read `reason`; the next run retries by itself |
| 2 | `refused` | a cap, a guard or a missing proof stopped the run; nothing was sent | read `refusal` and `reason` |
| 3 | `unknown` | something was sent and is not confirmed; it may still land | check `pendingSignature`; do not re-run by hand for three minutes |
| 4 | `halted`, with `alert: "drain_suspected"` | the fee payer is losing SOL on transactions that did not pay for themselves, or that could not be ruled out. **Nothing was refilled** | see "Monitoring" |

`DRY_RUN=1` performs every read, quote and check, and prints what it would do. It signs and
sends nothing.

## Environment variables

### Kora service

| Variable | Secret | Value |
| --- | --- | --- |
| `RPC_URL` | yes (contains a key) | mainnet RPC URL |
| `KORA_PRIVATE_KEY` | **yes** | the fee payer's key: the JSON array `solana-keygen` writes, or base58 |
| `KORA_API_KEY` | **yes** | `openssl rand -hex 32` |
| `KORA_HMAC_SECRET` | **yes** | another `openssl rand -hex 32` |
| `JUPITER_API_KEY` | yes | Kora refuses to start without it |
| `RUST_LOG` | no | `warn`. At `info` Kora logs every transaction it is sent |
| `PORT` | no | `8080` |

The payment wallet's address is not a variable: it is `payment_address` in `kora/kora.toml`.

### Refill job

| Variable | Secret | Default | Value |
| --- | --- | --- | --- |
| `RPC_URL` | yes | | mainnet RPC URL |
| `PAYMENT_WALLET_PRIVATE_KEY` | **yes** | | the payment wallet's key, JSON array or base58 |
| `FEE_PAYER` | no | | the fee payer's public key |
| `MAX_USDC_PER_SOL` | no | **none, required** | the most USDC ever paid for one SOL; about twice the current price, at most 1000 |
| `FEE_PAYER_UNSEEN_OK` | no | unset | first refill only: the same address as `FEE_PAYER`; remove afterwards |
| `JUPITER_API_KEY` | yes | none | optional; without it Jupiter answers at its keyless rate limit |
| `TARGET_SOL` | no | `0.1` | refill target, at most 0.5 |
| `REFILL_BELOW_SOL` | no | `0.03` | swap at or below this |
| `MAX_USDC_PER_RUN` | no | `15` | most USDC one swap may spend |
| `MAX_RUNS_PER_DAY` | no | `6` | most swaps in 24 hours |
| `USDC_FLOOR` | no | `1` | USDC never spent |
| `PAYMENT_WALLET_SOL_RESERVE` | no | `0.01` | SOL the payment wallet keeps, at least 0.002 |
| `MAX_SLIPPAGE_BPS` | no | `100` | how far under the reference price a quote may be, at most 500 |
| `DRY_RUN` | no | `0` | `1` to sign and send nothing |

The job refuses to start (exit 2, `bad_config`) without `MAX_USDC_PER_SOL`, on a target at or
below the threshold, a cap of zero, a `FEE_PAYER` that is the payment wallet or is not a
wallet address, a `FEE_PAYER_UNSEEN_OK` that differs from `FEE_PAYER`, or any value it cannot
parse. Its error names the variable and the rule, never the value.

### The app's server

`KORA_URL`, `KORA_API_KEY`, `KORA_HMAC_SECRET` (same values as on Kora), `KORA_FEE_PAYERS`,
`KORA_PAYMENT_WALLET`. See `docs/app-integration-notes.md` section 1.

## Deploying

`deploy.md` has the full sequence. In short: Kora is a Railway web service built from `kora/`
with a public domain and the healthcheck `/liveness`; the refill job is a Railway cron service
built from `refill/`, schedule once an hour, restart policy "never". Both are described
in `.railway/railway.ts`.

## More than one relayer

The app's server accepts several relayer endpoints, each with its own fee payer. To run more
than one:

- **Each replica is its own Kora service with its own fee payer key.** Never give two Kora
  services the same key.
- **All replicas use the same payment wallet** (`payment_address` in `kora/kora.toml` is one
  address for the whole deployment).
- **One refill service per fee payer.** A refill job handles exactly one `FEE_PAYER`. Run one
  per replica, with the same `PAYMENT_WALLET_PRIVATE_KEY` and a different `FEE_PAYER`.
- **Shift their schedules** so two jobs never start in the same minute: replica 1 at :00,
  :10, ...; replica 2 at :05, :15, .... `.railway/railway.ts` does this when `REPLICAS` is 2.
  Two jobs running at once on one wallet are the "second copy" case below.

How the limits behave when jobs share a wallet:

| Limit | Scope | Meaning with two replicas |
| --- | --- | --- |
| `MAX_RUNS_PER_DAY` (swaps) | **shared**: counted on the payment wallet | both jobs draw on one count, on purpose: it caps how often the wallet's USDC is spent. With two replicas, 6 per day is three each; raise it on both services if that is too few |
| Daily SOL ceiling, 0.5 SOL | **per job**: counted on that job's fee payer | two replicas can receive up to 1 SOL a day between them |
| Drain alarm | per job, on its own fee payer | one replica halting does not halt the other |
| Fee payer proof | per job | each fee payer needs its own first relayed payment, or `FEE_PAYER_UNSEEN_OK` once |
| Quiet time before a swap | shared: any swap by the wallet | one job's swap holds the other's back for three minutes; with shifted schedules it never does. One job's plain refill transfer never holds the other back, so they cannot deadlock |

`deploy.md` has the steps for adding a second replica.

## Monitoring and alerts

- **The refill job's exit code is the alert.** Any non-zero exit needs a look. How Railway
  notifies you of a failed cron run is NOT VERIFIED; until it is, read the service's logs or
  point an external monitor at them.
- **Reconciliation, every run.** The job pages through the fee payer's transactions of the
  last 15 minutes (up to 3,000) and, for each one the fee payer signed, compares the SOL it
  lost with the USDC the payment wallet received in the same transaction, valued at the
  reference price. Kora requires cost plus its margin, so a payment worth less than the SOL
  spent means the transaction did not pay for itself: it failed after Kora signed it, or it
  took rent it did not pay for. The report's `reconcile` carries `relayed`, `failed`,
  `underpaid`, `feePayerSolSpent`, `feePayerSolUncovered`, `usdcReceived`. With the `before`
  and `after` balances this is enough to reconcile SOL spent against USDC received over time.
- **Exit code 4, `drain_suspected`: the job stops refilling.** Raised when
  - the SOL lost on underpaid transactions in the window reaches 50,000 lamports (five failed
    plain transactions; one unpaid account creation is forty times that), or
  - the fee payer spent SOL and no USDC arrived at all, or
  - the window cannot be proven clean: more history than the page budget, a transaction that
    could not be read, or relayed transactions and no usable reference price.

  Inbound transfers to the fee payer cannot hide anything: the whole window is read, not the
  newest page. While the alarm stands, nothing is transferred and nothing is swapped. If the
  float is being drained, losing what is left of it (at most 0.1 SOL) is the bounded loss;
  feeding it from the collected USDC would not be. The fee payer runs dry, Kora refuses, and
  the app falls back to its own path. The alarm clears by itself 15 minutes after the last
  bad transaction. Look at the app's relay route and its rate limits meanwhile.
- **Fee payer balance.** Alert below 0.02 SOL: by then the refill has failed to keep up.
- **Kora itself:** `GET /liveness`. Railway checks it only at deploy time, not continuously,
  so use an external uptime monitor. `/metrics` is on port 9090, which is not exposed.
- **A rise in 401s** on Kora means a wrong secret or someone who found the URL.

## Rotating the fee payer

A transaction Kora has already signed can still land for as long as its blockhash is valid,
about 60 to 90 seconds. If the old key is emptied before that, those transactions fail. So
the old key is swept last, not first.

1. Generate the new key and fund it with 0.1 SOL.
2. Add its public key to the app's `KORA_FEE_PAYERS`, keeping the old one. Deploy the app.
3. Replace `KORA_PRIVATE_KEY` on the Kora service. Run `scripts/check-deploy.sh` with
   `FEE_PAYER=<new public key>` to confirm Kora now signs with it. Set the refill job's
   `FEE_PAYER` to the new public key and, because the new key has not relayed anything yet,
   `FEE_PAYER_UNSEEN_OK` to the same address (remove it after the new key's first relayed
   transaction). Redeploy both. From now on nothing new is signed with the old key.
4. **Wait at least 10 minutes, leaving the old key funded.** (Blockhash lifetime of about 90
   seconds, plus margin for the redeploy to finish everywhere.)
5. Confirm nothing is pending: `solana transaction-history <OLD_FEE_PAYER> --limit 5` shows
   no transaction newer than step 3, and `solana balance <OLD_FEE_PAYER>` has stopped moving.
6. Remove the old public key from `KORA_FEE_PAYERS`. Deploy the app.
7. Only now sweep the old key: `solana transfer --from old-fee-payer.json <NEW_FEE_PAYER> ALL`.

The payment wallet does not change, so nothing users pay to moves.

**The auth secrets:** set the new `KORA_API_KEY` and `KORA_HMAC_SECRET` on both Kora and the
app. Between the two deploys the app gets 401s and falls back to its non-relayed path.

**The payment wallet:** generate a new key, create its USDC account (`deploy.md` step 3), put
the new address in `kora/kora.toml` and in the app's `KORA_PAYMENT_WALLET`, redeploy Kora and
the app, give the refill job the new `PAYMENT_WALLET_PRIVATE_KEY`, then move the old wallet's
USDC and SOL by hand. The job's 24-hour counters start from zero for a new wallet.

## Keeping up with upstream

`scripts/check-upstream.sh` prints the latest upstream stable and pre-release tags and image
digests next to the pinned ones and exits non-zero when they differ.
`docs/updating-from-upstream.md` is the procedure, including the rule for when account opening
can move to a newer audited release.

## Tests

```bash
cd refill && npm ci && npm test
```

91 tests, no network: configuration, the decision logic, the guard against hostile orders
and fills, the price reference, unknown outcomes, the daily limits, the fee payer proof, the
reconciliation and the drain halt, timeouts and the deadline, and the pinned transfer
destination.

## What was verified, and what was not

Verified on 2026-10-02 and 2026-10-03:

- Both Kora configs pass their own image's `kora config validate`; both Dockerfiles build.
  The default image built from `kora/Dockerfile` ran against a local mainnet fork, answered
  `/liveness`, refused unauthenticated and half-authenticated calls with 401, and reported its
  settings and its fee payer through `getConfig` and `getPayerSigner`.
- The relayed flows on the default release with this configuration at a 10 percent margin
  (sends, account-opening sends, Earn deposit and withdrawal) and the refusals:
  `docs/fork-test-runbook.md` section 8.
- The refill job on a local mainnet fork, each run a fresh process, with real signatures: the
  transfer legs (steps 1 and 3 are the same code), the fee payer proof and its one-time
  acknowledgement, a wrong `FEE_PAYER` receiving nothing, the swap cap with inbound dust
  ignored, the quiet-time rule, the daily SOL ceiling, and the drain halt with unpaid
  transactions hidden behind newer inbound transfers.
- The job's guard, read-only, against genuine market-maker orders Jupiter built for real
  mainnet wallets: the fill instruction decoded, its amounts equal to the quote, the
  simulation matching, nothing signed. And the Pyth price read from mainnet.
- A `DRY_RUN` against mainnet, read-only, with an empty throwaway wallet.

**NOT VERIFIED:**

- **A real swap.** Jupiter builds no transaction for a wallet without real funds and its
  execute endpoint submits to mainnet, so signing, `/execute` and the confirmation of a swap
  have only run against fakes in the unit tests. Run the first refill with `DRY_RUN=1`, then
  watch the first live one.
- **The fill instruction's layout comes from observation, not from a published
  specification.** It was read off live orders (discriminator, input, output, expiry, and the
  taker, input-account and mint positions) and matched every order tried. If Jupiter changes
  it, the job refuses every order (`bad_fill`) until `src/guard.mjs` is updated: it fails
  closed, but it does fail.
- **How long market makers keep quoting small USDC to SOL orders.** If they stop, refills
  stop (exit 2 every run) and the fee payer needs SOL by hand.
- **Anything on Railway.** No command in `deploy.md` was run against Railway. The local CLI
  available during this work predates `railway config`. `.railway/railway.ts` was evaluated
  with the SDK and produces the intended service graph; it was never applied. The statement
  that an overlapping cron run is skipped is from Railway's docs, not from a test.
- How Railway reports a cron run that exits non-zero, and whether its builder picks Node 22
  or newer from `refill/package.json`.
- The caller-side checks. They live in the app; its test for the minimum payment is named in
  `deploy.md` and was not run here. The stable alternative's failure on a Lend withdrawal was
  established by the app's team on a fork, not re-run here.
- Two refill jobs sharing one payment wallet on a fork. The shared counter and the quiet-time
  behaviour are covered by unit tests only.
- Kora pricing against the live Jupiter price (`price_source = "Jupiter"` with a real key).
- Landing a plain transfer without a priority fee under congestion. The refill transfer
  carries none; if it expires the run exits 1 and the next one retries.
- The key rotation sequence as a whole.
- Everything `docs/fork-test-runbook.md` section 11 lists.

## Security

Found a way to make the relayer pay for something it should not, or to move the payment
wallet's funds? Write to ph1l1ph@proton.me before publishing it. Please include the
transaction or request that shows it. Issues in Kora itself belong upstream: see the
repository's `SECURITY.md`.
