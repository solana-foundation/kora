# Databricks notebook source
import re
import time
from datetime import datetime, timezone

import requests
from pyspark.sql.types import (
    BooleanType,
    DoubleType,
    IntegerType,
    LongType,
    StringType,
    StructField,
    StructType,
    TimestampType,
)

dbutils.widgets.text("catalog", "")
dbutils.widgets.text("schema", "")
dbutils.widgets.text("address", "")
dbutils.widgets.text("rpc_url", "https://api.devnet.solana.com")
dbutils.widgets.text("max_signatures", "5000")

CATALOG = dbutils.widgets.get("catalog")
SCHEMA = dbutils.widgets.get("schema")
ADDR = dbutils.widgets.get("address")
RPC = dbutils.widgets.get("rpc_url")
MAX_SIGNATURES = int(dbutils.widgets.get("max_signatures"))

if not (CATALOG and SCHEMA and ADDR):
    raise ValueError("catalog, schema, and address parameters are required")
if not all(re.fullmatch(r"[A-Za-z0-9_]+", x) for x in (CATALOG, SCHEMA)):
    raise ValueError("catalog and schema must be plain identifiers")

NS = f"`{CATALOG}`.`{SCHEMA}`"
TX = f"{CATALOG}.{SCHEMA}.transactions"
IX = f"{CATALOG}.{SCHEMA}.instructions"
BAL = f"{CATALOG}.{SCHEMA}.balance_snapshots"

spark.sql(f"CREATE SCHEMA IF NOT EXISTS {NS}")
spark.sql(
    f"CREATE TABLE IF NOT EXISTS {NS}.`balance_snapshots` "
    "(snapshot_at TIMESTAMP, address STRING, lamports BIGINT, sol DOUBLE) USING DELTA"
)
spark.sql(
    f"CREATE TABLE IF NOT EXISTS {NS}.`transactions` "
    "(signature STRING, block_time TIMESTAMP, slot BIGINT, success BOOLEAN, err STRING, "
    "fee_lamports BIGINT, num_instructions INT, fetched_at TIMESTAMP) USING DELTA"
)
spark.sql(
    f"CREATE TABLE IF NOT EXISTS {NS}.`instructions` "
    "(signature STRING, ix_index INT, is_inner BOOLEAN, program STRING, program_id STRING, "
    "ix_type STRING, block_time TIMESTAMP) USING DELTA"
)


def rpc(method, params):
    for attempt in range(6):
        try:
            r = requests.post(
                RPC,
                json={"jsonrpc": "2.0", "id": 1, "method": method, "params": params},
                timeout=30,
            )
            out = r.json()
            if "error" in out:
                raise RuntimeError(out["error"])
            return out["result"]
        except Exception:
            if attempt == 5:
                raise
            time.sleep(1.5 * (attempt + 1))


def newest_stored_sig():
    rows = spark.sql(f"SELECT signature FROM {TX} ORDER BY block_time DESC LIMIT 1").collect()
    return rows[0].signature if rows else None


def new_signatures(until_sig):
    collected, before = [], None
    while True:
        opts = {"limit": 1000}
        if before:
            opts["before"] = before
        if until_sig:
            opts["until"] = until_sig
        page = rpc("getSignaturesForAddress", [ADDR, opts])
        if not page:
            break
        for s in page:
            collected.append(s["signature"])
            if len(collected) >= MAX_SIGNATURES:
                print(f"hit max_signatures cap {MAX_SIGNATURES}; older history skipped")
                return collected
        if len(page) < 1000:
            break
        before = page[-1]["signature"]
    return collected


def parse_ix(ins):
    parsed = ins.get("parsed")
    ix_type = parsed.get("type") if isinstance(parsed, dict) else None
    return ins.get("program"), ins.get("programId"), ix_type


def ts(epoch):
    return datetime.fromtimestamp(epoch, tz=timezone.utc) if epoch is not None else None


sigs = new_signatures(newest_stored_sig())
print(f"new signatures: {len(sigs)}")

now = datetime.now(timezone.utc)
tx_rows, ix_rows = [], []
for sig in sigs:
    tx = rpc(
        "getTransaction",
        [sig, {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0}],
    )
    if tx is None:
        continue
    meta = tx.get("meta") or {}
    bt = ts(tx.get("blockTime"))
    err = meta.get("err")
    top = tx["transaction"]["message"].get("instructions", [])
    tx_rows.append(
        (sig, bt, tx.get("slot"), err is None, str(err) if err is not None else None,
         meta.get("fee"), len(top), now)
    )
    for i, ins in enumerate(top):
        p, pid, t = parse_ix(ins)
        ix_rows.append((sig, i, False, p, pid, t, bt))
    for group in meta.get("innerInstructions", []) or []:
        for j, ins in enumerate(group.get("instructions", [])):
            p, pid, t = parse_ix(ins)
            ix_rows.append((sig, group["index"] * 1000 + j, True, p, pid, t, bt))

tx_schema = StructType([
    StructField("signature", StringType()),
    StructField("block_time", TimestampType()),
    StructField("slot", LongType()),
    StructField("success", BooleanType()),
    StructField("err", StringType()),
    StructField("fee_lamports", LongType()),
    StructField("num_instructions", IntegerType()),
    StructField("fetched_at", TimestampType()),
])
ix_schema = StructType([
    StructField("signature", StringType()),
    StructField("ix_index", IntegerType()),
    StructField("is_inner", BooleanType()),
    StructField("program", StringType()),
    StructField("program_id", StringType()),
    StructField("ix_type", StringType()),
    StructField("block_time", TimestampType()),
])

if tx_rows:
    spark.createDataFrame(tx_rows, tx_schema).write.format("delta").mode("append").saveAsTable(TX)
if ix_rows:
    spark.createDataFrame(ix_rows, ix_schema).write.format("delta").mode("append").saveAsTable(IX)

bal = rpc("getBalance", [ADDR])["value"]
bal_schema = StructType([
    StructField("snapshot_at", TimestampType()),
    StructField("address", StringType()),
    StructField("lamports", LongType()),
    StructField("sol", DoubleType()),
])
spark.createDataFrame(
    [(now, ADDR, bal, bal / 1_000_000_000)], bal_schema
).write.format("delta").mode("append").saveAsTable(BAL)

print(f"appended {len(tx_rows)} tx, {len(ix_rows)} ix, balance {bal / 1e9} SOL")
