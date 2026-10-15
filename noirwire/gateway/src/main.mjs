import { Connection } from "@solana/web3.js";
import pg from "pg";
import { createChain } from "./chain.mjs";
import { Sessions } from "./auth.mjs";
import { Challenges } from "./challenges.mjs";
import { startSweeper } from "./confirm.mjs";
import { ConfigError, loadConfig } from "./config.mjs";
import { refreshFeePayerHealth, startHealthChecker } from "./health.mjs";
import { createKoraClient } from "./kora.mjs";
import { PgStore } from "./pg-store.mjs";
import { readReferencePrice } from "./price.mjs";
import { QuoteBook } from "./quotes.mjs";
import { buildServer } from "./server.mjs";

async function main() {
  const config = loadConfig(process.env);
  const chain = createChain(new Connection(config.rpcUrl, "confirmed"));
  const store = new PgStore(new pg.Pool({ connectionString: config.databaseUrl, max: 10 }));
  await store.ping();

  const ctx = {
    config,
    chain,
    store,
    kora: createKoraClient({ url: config.koraUrl, apiKey: config.koraApiKey, hmacSecret: config.koraHmacSecret }),
    quotes: new QuoteBook(),
    challenges: new Challenges(),
    sessions: new Sessions(),
    readPrice: () => readReferencePrice(chain, Math.floor(Date.now() / 1000)),
    now: Date.now,
  };

  await refreshFeePayerHealth(ctx);
  startHealthChecker(ctx);
  startSweeper(ctx);
  const server = await buildServer(ctx);
  await server.listen({ port: config.port, host: "0.0.0.0" });

  for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => void server.close().then(() => process.exit(0)));
}

main().catch((error) => {
  // A config problem names the settings at fault and never their values.
  console.error(error instanceof ConfigError ? `invalid configuration: ${error.message}` : `failed to start: ${error.message}`);
  process.exit(1);
});
