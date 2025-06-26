import { createJsonRpcApi } from '@solana/rpc-spec';
import { getDefaultResponseTransformerForSolanaRpc, getDefaultRequestTransformerForSolanaRpc, jsonParsedAccountsConfigs, innerInstructionsConfigs, KEYPATH_WILDCARD, messageConfig, jsonParsedTokenAccountsConfigs } from '@solana/rpc-transformers';

// src/index.ts
function createSolanaRpcApi(config) {
  return createJsonRpcApi({
    requestTransformer: getDefaultRequestTransformerForSolanaRpc(config),
    responseTransformer: getDefaultResponseTransformerForSolanaRpc({
      allowedNumericKeyPaths: getAllowedNumericKeypaths()
    })
  });
}
var memoizedKeypaths;
function getAllowedNumericKeypaths() {
  if (!memoizedKeypaths) {
    memoizedKeypaths = {
      getAccountInfo: jsonParsedAccountsConfigs.map((c) => ["value", ...c]),
      getBlock: [
        ["transactions", KEYPATH_WILDCARD, "meta", "preTokenBalances", KEYPATH_WILDCARD, "accountIndex"],
        [
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "preTokenBalances",
          KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        ["transactions", KEYPATH_WILDCARD, "meta", "postTokenBalances", KEYPATH_WILDCARD, "accountIndex"],
        [
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "postTokenBalances",
          KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        ["transactions", KEYPATH_WILDCARD, "meta", "rewards", KEYPATH_WILDCARD, "commission"],
        ...innerInstructionsConfigs.map((c) => [
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          KEYPATH_WILDCARD,
          ...c
        ]),
        ...messageConfig.map((c) => ["transactions", KEYPATH_WILDCARD, "transaction", "message", ...c]),
        ["rewards", KEYPATH_WILDCARD, "commission"]
      ],
      getClusterNodes: [
        [KEYPATH_WILDCARD, "featureSet"],
        [KEYPATH_WILDCARD, "shredVersion"]
      ],
      getInflationGovernor: [["initial"], ["foundation"], ["foundationTerm"], ["taper"], ["terminal"]],
      getInflationRate: [["foundation"], ["total"], ["validator"]],
      getInflationReward: [[KEYPATH_WILDCARD, "commission"]],
      getMultipleAccounts: jsonParsedAccountsConfigs.map((c) => ["value", KEYPATH_WILDCARD, ...c]),
      getProgramAccounts: jsonParsedAccountsConfigs.flatMap((c) => [
        ["value", KEYPATH_WILDCARD, "account", ...c],
        [KEYPATH_WILDCARD, "account", ...c]
      ]),
      getRecentPerformanceSamples: [[KEYPATH_WILDCARD, "samplePeriodSecs"]],
      getTokenAccountBalance: [
        ["value", "decimals"],
        ["value", "uiAmount"]
      ],
      getTokenAccountsByDelegate: jsonParsedTokenAccountsConfigs.map((c) => [
        "value",
        KEYPATH_WILDCARD,
        "account",
        ...c
      ]),
      getTokenAccountsByOwner: jsonParsedTokenAccountsConfigs.map((c) => [
        "value",
        KEYPATH_WILDCARD,
        "account",
        ...c
      ]),
      getTokenLargestAccounts: [
        ["value", KEYPATH_WILDCARD, "decimals"],
        ["value", KEYPATH_WILDCARD, "uiAmount"]
      ],
      getTokenSupply: [
        ["value", "decimals"],
        ["value", "uiAmount"]
      ],
      getTransaction: [
        ["meta", "preTokenBalances", KEYPATH_WILDCARD, "accountIndex"],
        ["meta", "preTokenBalances", KEYPATH_WILDCARD, "uiTokenAmount", "decimals"],
        ["meta", "postTokenBalances", KEYPATH_WILDCARD, "accountIndex"],
        ["meta", "postTokenBalances", KEYPATH_WILDCARD, "uiTokenAmount", "decimals"],
        ["meta", "rewards", KEYPATH_WILDCARD, "commission"],
        ...innerInstructionsConfigs.map((c) => ["meta", "innerInstructions", KEYPATH_WILDCARD, ...c]),
        ...messageConfig.map((c) => ["transaction", "message", ...c])
      ],
      getVersion: [["feature-set"]],
      getVoteAccounts: [
        ["current", KEYPATH_WILDCARD, "commission"],
        ["delinquent", KEYPATH_WILDCARD, "commission"]
      ],
      simulateTransaction: [
        ...jsonParsedAccountsConfigs.map((c) => ["value", "accounts", KEYPATH_WILDCARD, ...c]),
        ...innerInstructionsConfigs.map((c) => ["value", "innerInstructions", KEYPATH_WILDCARD, ...c])
      ]
    };
  }
  return memoizedKeypaths;
}

export { createSolanaRpcApi };
//# sourceMappingURL=index.browser.mjs.map
//# sourceMappingURL=index.browser.mjs.map