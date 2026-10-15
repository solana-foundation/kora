-- The gateway's tables for a local Postgres, in the default schema. Production uses the same
-- tables from the Supabase migration (see the README's Database section). Every statement is
-- idempotent.

create table if not exists apps (
  id                  text primary key,
  owner               text not null,
  name                text not null,
  payout_wallet       text,
  markup_bps          integer not null check (markup_bps between 0 and 30000),
  allowed_origins     text[] not null default '{}',
  allowed_programs    text[] not null default '{}',
  daily_cap_lamports  bigint not null check (daily_cap_lamports > 0),
  paused              boolean not null default false,
  first_transaction_at timestamptz,
  created_at          timestamptz not null default now(),
  updated_at          timestamptz not null default now()
);

create index if not exists apps_owner on apps (owner);

-- One row per transaction the gateway was asked to send, keyed by the hash of the message
-- the user signed. status:
--   sending   admitted and reserved; Kora has not answered yet
--   signed    Kora signed it and the signed bytes are stored; not yet accepted by the network
--   sent      broadcast and accepted by a node; the chain decides
--   landed    confirmed without error
--   failed    confirmed with an error; loss_lamports is what the fee payer paid
--   expired   the blockhash lapsed without the transaction landing
--   rejected  Kora or the node refused it: nothing was spent
--   unknown   Kora's answer was unclear; the signature is not known. Settled as expired once
--             the blockhash is gone (see the sweep)
--
-- loss_lamports is what the fee payer could lose to this transaction failing: the worst case
-- (signature and priority fees) while it is open, the actual fee once it failed, zero once it
-- landed, expired or was rejected. sponsored_lamports is what it costs the fee payer if it lands
-- (fees plus the simulated outflow), and is what an app's daily cap counts.
create table if not exists transactions (
  message_hash          text primary key,
  app_id                text not null references apps (id),
  user_wallet           text not null,
  client_ip             text,
  blockhash             text not null,
  signature             text,
  signed_transaction    text,
  -- Unix milliseconds until which one broadcast attempt owns this row; null when none does.
  broadcast_lease_until bigint,
  status                text not null check (status in ('sending', 'signed', 'sent', 'landed', 'failed', 'expired', 'rejected', 'unknown')),
  failure_reason        text,
  network_cost_lamports bigint not null,
  network_cost          bigint not null,
  markup                bigint not null,
  noirwire_share        bigint not null,
  app_share             bigint not null,
  loss_lamports         bigint not null default 0,
  sponsored_lamports    bigint not null default 0,
  -- Whole milliseconds, so a paging cursor that travels through a JavaScript Date is exact.
  created_at            timestamptz not null default date_trunc('milliseconds', now()),
  updated_at            timestamptz not null default now()
);

create index if not exists transactions_created on transactions (created_at) include (loss_lamports);
create index if not exists transactions_wallet on transactions (user_wallet, created_at) include (loss_lamports, status);
create index if not exists transactions_ip on transactions (client_ip, created_at) include (loss_lamports) where client_ip is not null;
create index if not exists transactions_app on transactions (app_id, created_at desc, message_hash desc);
create index if not exists transactions_open on transactions (created_at, message_hash) where status in ('sending', 'signed', 'sent', 'unknown');
