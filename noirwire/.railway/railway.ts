import { defineRailway, github, preserve, project, service } from "railway/iac";

// The Railway services, as code. Apply with `railway config apply` from this folder (see
// ../deploy.md). Secrets and the public keys are never written here: they are set once with
// `railway variable set`, and `preserve()` tells Railway to keep them.

// This repository manages one slice of the Railway project it shares with the API, so
// applying this file never touches a service another repository declared.
export const partial = "relayer";

// How many relayers to run. Each replica is one Kora service with its OWN fee payer key and
// one refill service for that fee payer. All of them share the same payment wallet.
// Replica 1 is `kora-relayer-noirwire` and `refill`; replica 2 is `kora-2` and `refill-2`.
const REPLICAS = 1;

// Set RELAYER_GITHUB_REPO=owner/repo to have Railway build from GitHub on every push.
// Leave it unset to deploy from this machine with `railway up`.
const repo = process.env.RELAYER_GITHUB_REPO;
const source = (rootDirectory: string) =>
  repo ? github(repo, { branch: process.env.RELAYER_GITHUB_BRANCH ?? "main", rootDirectory }) : undefined;

// The fee relayer: a web service built from kora/Dockerfile (or, for the public relayer that
// only the gateway may call, kora/Dockerfile.public).
const kora = (name: string, dockerfilePath = "Dockerfile", privateKey = preserve()) =>
  service(name, {
    source: source("/noirwire/kora"),
    build: { builder: "DOCKERFILE", dockerfilePath },
    deploy: {
      healthcheckPath: "/liveness",
      healthcheckTimeout: 60,
      restartPolicyType: "ON_FAILURE",
      restartPolicyMaxRetries: 10,
    },
    env: {
      PORT: "8080",
      // Not "info": at that level Kora logs every transaction it is sent.
      RUST_LOG: "warn",
      RPC_URL: preserve(),
      KORA_PRIVATE_KEY: privateKey,
      KORA_API_KEY: preserve(),
      KORA_HMAC_SECRET: preserve(),
      JUPITER_API_KEY: preserve(),
    },
  });

// The refill job for one fee payer: runs once an hour and exits. Never restarted:
// a run that ends with a non-zero code must be looked at, not repeated. Jobs that share the
// payment wallet must not run at the same minute, so each replica's schedule is shifted
// (replica 1 at :00, replica 2 at :05, ...; Railway needs 5 minutes between runs).
const refill = (name: string, minute: number) =>
  service(name, {
    source: source("/noirwire/refill"),
    start: "node src/main.mjs",
    deploy: {
      cronSchedule: `${minute} * * * *`,
      restartPolicyType: "NEVER",
    },
    env: {
      TARGET_SOL: "0.1",
      REFILL_BELOW_SOL: "0.03",
      MAX_USDC_PER_RUN: "15",
      MAX_RUNS_PER_DAY: "6",
      USDC_FLOOR: "1",
      PAYMENT_WALLET_SOL_RESERVE: "0.01",
      MAX_SLIPPAGE_BPS: "100",
      DRY_RUN: "0",
      RPC_URL: preserve(),
      PAYMENT_WALLET_PRIVATE_KEY: preserve(),
      FEE_PAYER: preserve(),
      // Required, no default: the most USDC the job may ever pay for one SOL.
      MAX_USDC_PER_SOL: preserve(),
      JUPITER_API_KEY: preserve(),
    },
  });

// The public gateway (see ../gateway/README.md): the only caller of the public Kora. Its
// domain is created with `railway domain` like the private relayer's. Everything that
// identifies a deployment or is secret is set once with `railway variable set`.
const gateway = (name: string) =>
  service(name, {
    source: source("/noirwire/gateway"),
    start: "node src/main.mjs",
    deploy: {
      healthcheckPath: "/v1/health",
      healthcheckTimeout: 60,
      restartPolicyType: "ON_FAILURE",
      restartPolicyMaxRetries: 10,
    },
    env: {
      PORT: "8080",
      // Exactly one proxy hop (Railway's edge) is trusted. Verify that the edge appends the
      // client address to X-Forwarded-For before relying on it: see ../gateway/README.md.
      TRUSTED_PROXY_HOPS: "1",
      NETWORK: preserve(),
      CHARGE: preserve(),
      RPC_URL: preserve(),
      // Where the public Kora answers, normally its private network address.
      KORA_URL: preserve(),
      KORA_API_KEY: preserve(),
      KORA_HMAC_SECRET: preserve(),
      FEE_PAYER: preserve(),
      PAYMENT_USDC_ACCOUNT: preserve(),
      DATABASE_URL: preserve(),
    },
  });

export default defineRailway(() => {
  if (REPLICAS < 1 || REPLICAS > 2) throw new Error("REPLICAS must be 1 or 2: the refill schedules are 5 minutes apart");
  const resources = [];
  const firstKora = kora("kora-relayer-noirwire");
  resources.push(firstKora, refill("refill", 0));
  for (let replica = 2; replica <= REPLICAS; replica += 1) {
    resources.push(kora(`kora-${replica}`), refill(`refill-${replica}`, (replica - 1) * 5));
  }
  // The public relayer's Kora signs with the first relayer's fee payer key, read by reference so
  // the key is set in one place, and is paid to the same wallet: `refill` funds both.
  resources.push(kora("kora-public", "Dockerfile.public", firstKora.env.KORA_PRIVATE_KEY), gateway("gateway"));
  return project("noirwire-relayer", { resources });
});
