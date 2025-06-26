import { SolanaError, SOLANA_ERROR__RPC__TRANSPORT_HTTP_ERROR, SOLANA_ERROR__RPC__TRANSPORT_HTTP_HEADER_FORBIDDEN } from '@solana/errors';
import { stringifyJsonWithBigints, parseJsonWithBigInts } from '@solana/rpc-spec-types';
import { isJsonRpcPayload } from '@solana/rpc-spec';

// src/http-transport.ts
var DISALLOWED_HEADERS = {
  accept: true,
  "content-length": true,
  "content-type": true
};
var FORBIDDEN_HEADERS = /* @__PURE__ */ Object.assign(
  {
    "accept-charset": true,
    "access-control-request-headers": true,
    "access-control-request-method": true,
    connection: true,
    "content-length": true,
    cookie: true,
    date: true,
    dnt: true,
    expect: true,
    host: true,
    "keep-alive": true,
    origin: true,
    "permissions-policy": true,
    // Prefix matching is implemented in code, below.
    // 'proxy-': true,
    // 'sec-': true,
    referer: true,
    te: true,
    trailer: true,
    "transfer-encoding": true,
    upgrade: true,
    via: true
  },
  { "accept-encoding": true }
);
function assertIsAllowedHttpRequestHeaders(headers) {
  const badHeaders = Object.keys(headers).filter((headerName) => {
    const lowercaseHeaderName = headerName.toLowerCase();
    return DISALLOWED_HEADERS[headerName.toLowerCase()] === true || FORBIDDEN_HEADERS[headerName.toLowerCase()] === true || lowercaseHeaderName.startsWith("proxy-") || lowercaseHeaderName.startsWith("sec-");
  });
  if (badHeaders.length > 0) {
    throw new SolanaError(SOLANA_ERROR__RPC__TRANSPORT_HTTP_HEADER_FORBIDDEN, {
      headers: badHeaders
    });
  }
}
function normalizeHeaders(headers) {
  const out = {};
  for (const headerName in headers) {
    out[headerName.toLowerCase()] = headers[headerName];
  }
  return out;
}

// src/http-transport.ts
var didWarnDispatcherWasSuppliedInNonNodeEnvironment = false;
function warnDispatcherWasSuppliedInNonNodeEnvironment() {
  if (didWarnDispatcherWasSuppliedInNonNodeEnvironment) {
    return;
  }
  didWarnDispatcherWasSuppliedInNonNodeEnvironment = true;
  console.warn(
    "You have supplied a `Dispatcher` to `createHttpTransport()`. It has been ignored because Undici dispatchers only work in Node environments. To eliminate this warning, omit the `dispatcher_NODE_ONLY` property from your config when running in a non-Node environment."
  );
}
function createHttpTransport(config) {
  if (process.env.NODE_ENV !== "production" && true && "dispatcher_NODE_ONLY" in config) {
    warnDispatcherWasSuppliedInNonNodeEnvironment();
  }
  const { fromJson, headers, toJson, url } = config;
  if (process.env.NODE_ENV !== "production" && headers) {
    assertIsAllowedHttpRequestHeaders(headers);
  }
  let dispatcherConfig;
  const customHeaders = headers && normalizeHeaders(headers);
  return async function makeHttpRequest({
    payload,
    signal
  }) {
    const body = toJson ? toJson(payload) : JSON.stringify(payload);
    const requestInfo = {
      ...dispatcherConfig,
      body,
      headers: {
        ...customHeaders,
        // Keep these headers lowercase so they will override any user-supplied headers above.
        accept: "application/json",
        "content-length": body.length.toString(),
        "content-type": "application/json; charset=utf-8"
      },
      method: "POST",
      signal
    };
    const response = await fetch(url, requestInfo);
    if (!response.ok) {
      throw new SolanaError(SOLANA_ERROR__RPC__TRANSPORT_HTTP_ERROR, {
        headers: response.headers,
        message: response.statusText,
        statusCode: response.status
      });
    }
    if (fromJson) {
      return fromJson(await response.text(), payload);
    }
    return await response.json();
  };
}
var SOLANA_RPC_METHODS = [
  "getAccountInfo",
  "getBalance",
  "getBlock",
  "getBlockCommitment",
  "getBlockHeight",
  "getBlockProduction",
  "getBlocks",
  "getBlocksWithLimit",
  "getBlockTime",
  "getClusterNodes",
  "getEpochInfo",
  "getEpochSchedule",
  "getFeeForMessage",
  "getFirstAvailableBlock",
  "getGenesisHash",
  "getHealth",
  "getHighestSnapshotSlot",
  "getIdentity",
  "getInflationGovernor",
  "getInflationRate",
  "getInflationReward",
  "getLargestAccounts",
  "getLatestBlockhash",
  "getLeaderSchedule",
  "getMaxRetransmitSlot",
  "getMaxShredInsertSlot",
  "getMinimumBalanceForRentExemption",
  "getMultipleAccounts",
  "getProgramAccounts",
  "getRecentPerformanceSamples",
  "getRecentPrioritizationFees",
  "getSignaturesForAddress",
  "getSignatureStatuses",
  "getSlot",
  "getSlotLeader",
  "getSlotLeaders",
  "getStakeMinimumDelegation",
  "getSupply",
  "getTokenAccountBalance",
  "getTokenAccountsByDelegate",
  "getTokenAccountsByOwner",
  "getTokenLargestAccounts",
  "getTokenSupply",
  "getTransaction",
  "getTransactionCount",
  "getVersion",
  "getVoteAccounts",
  "index",
  "isBlockhashValid",
  "minimumLedgerSlot",
  "requestAirdrop",
  "sendTransaction",
  "simulateTransaction"
];
function isSolanaRequest(payload) {
  return isJsonRpcPayload(payload) && SOLANA_RPC_METHODS.includes(payload.method);
}

// src/http-transport-for-solana-rpc.ts
function createHttpTransportForSolanaRpc(config) {
  return createHttpTransport({
    ...config,
    fromJson: (rawResponse, payload) => isSolanaRequest(payload) ? parseJsonWithBigInts(rawResponse) : JSON.parse(rawResponse),
    toJson: (payload) => isSolanaRequest(payload) ? stringifyJsonWithBigints(payload) : JSON.stringify(payload)
  });
}

export { createHttpTransport, createHttpTransportForSolanaRpc };
//# sourceMappingURL=index.browser.mjs.map
//# sourceMappingURL=index.browser.mjs.map