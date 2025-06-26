'use strict';

var rpcSpec = require('@solana/rpc-spec');
var rpcTransformers = require('@solana/rpc-transformers');

// src/index.ts
function createSolanaRpcApi(config) {
  return rpcSpec.createJsonRpcApi({
    requestTransformer: rpcTransformers.getDefaultRequestTransformerForSolanaRpc(config),
    responseTransformer: rpcTransformers.getDefaultResponseTransformerForSolanaRpc({
      allowedNumericKeyPaths: getAllowedNumericKeypaths()
    })
  });
}
var memoizedKeypaths;
function getAllowedNumericKeypaths() {
  if (!memoizedKeypaths) {
    memoizedKeypaths = {
      getAccountInfo: rpcTransformers.jsonParsedAccountsConfigs.map((c) => ["value", ...c]),
      getBlock: [
        ["transactions", rpcTransformers.KEYPATH_WILDCARD, "meta", "preTokenBalances", rpcTransformers.KEYPATH_WILDCARD, "accountIndex"],
        [
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "preTokenBalances",
          rpcTransformers.KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        ["transactions", rpcTransformers.KEYPATH_WILDCARD, "meta", "postTokenBalances", rpcTransformers.KEYPATH_WILDCARD, "accountIndex"],
        [
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "postTokenBalances",
          rpcTransformers.KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        ["transactions", rpcTransformers.KEYPATH_WILDCARD, "meta", "rewards", rpcTransformers.KEYPATH_WILDCARD, "commission"],
        ...rpcTransformers.innerInstructionsConfigs.map((c) => [
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          rpcTransformers.KEYPATH_WILDCARD,
          ...c
        ]),
        ...rpcTransformers.messageConfig.map((c) => ["transactions", rpcTransformers.KEYPATH_WILDCARD, "transaction", "message", ...c]),
        ["rewards", rpcTransformers.KEYPATH_WILDCARD, "commission"]
      ],
      getClusterNodes: [
        [rpcTransformers.KEYPATH_WILDCARD, "featureSet"],
        [rpcTransformers.KEYPATH_WILDCARD, "shredVersion"]
      ],
      getInflationGovernor: [["initial"], ["foundation"], ["foundationTerm"], ["taper"], ["terminal"]],
      getInflationRate: [["foundation"], ["total"], ["validator"]],
      getInflationReward: [[rpcTransformers.KEYPATH_WILDCARD, "commission"]],
      getMultipleAccounts: rpcTransformers.jsonParsedAccountsConfigs.map((c) => ["value", rpcTransformers.KEYPATH_WILDCARD, ...c]),
      getProgramAccounts: rpcTransformers.jsonParsedAccountsConfigs.flatMap((c) => [
        ["value", rpcTransformers.KEYPATH_WILDCARD, "account", ...c],
        [rpcTransformers.KEYPATH_WILDCARD, "account", ...c]
      ]),
      getRecentPerformanceSamples: [[rpcTransformers.KEYPATH_WILDCARD, "samplePeriodSecs"]],
      getTokenAccountBalance: [
        ["value", "decimals"],
        ["value", "uiAmount"]
      ],
      getTokenAccountsByDelegate: rpcTransformers.jsonParsedTokenAccountsConfigs.map((c) => [
        "value",
        rpcTransformers.KEYPATH_WILDCARD,
        "account",
        ...c
      ]),
      getTokenAccountsByOwner: rpcTransformers.jsonParsedTokenAccountsConfigs.map((c) => [
        "value",
        rpcTransformers.KEYPATH_WILDCARD,
        "account",
        ...c
      ]),
      getTokenLargestAccounts: [
        ["value", rpcTransformers.KEYPATH_WILDCARD, "decimals"],
        ["value", rpcTransformers.KEYPATH_WILDCARD, "uiAmount"]
      ],
      getTokenSupply: [
        ["value", "decimals"],
        ["value", "uiAmount"]
      ],
      getTransaction: [
        ["meta", "preTokenBalances", rpcTransformers.KEYPATH_WILDCARD, "accountIndex"],
        ["meta", "preTokenBalances", rpcTransformers.KEYPATH_WILDCARD, "uiTokenAmount", "decimals"],
        ["meta", "postTokenBalances", rpcTransformers.KEYPATH_WILDCARD, "accountIndex"],
        ["meta", "postTokenBalances", rpcTransformers.KEYPATH_WILDCARD, "uiTokenAmount", "decimals"],
        ["meta", "rewards", rpcTransformers.KEYPATH_WILDCARD, "commission"],
        ...rpcTransformers.innerInstructionsConfigs.map((c) => ["meta", "innerInstructions", rpcTransformers.KEYPATH_WILDCARD, ...c]),
        ...rpcTransformers.messageConfig.map((c) => ["transaction", "message", ...c])
      ],
      getVersion: [["feature-set"]],
      getVoteAccounts: [
        ["current", rpcTransformers.KEYPATH_WILDCARD, "commission"],
        ["delinquent", rpcTransformers.KEYPATH_WILDCARD, "commission"]
      ],
      simulateTransaction: [
        ...rpcTransformers.jsonParsedAccountsConfigs.map((c) => ["value", "accounts", rpcTransformers.KEYPATH_WILDCARD, ...c]),
        ...rpcTransformers.innerInstructionsConfigs.map((c) => ["value", "innerInstructions", rpcTransformers.KEYPATH_WILDCARD, ...c])
      ]
    };
  }
  return memoizedKeypaths;
}

exports.createSolanaRpcApi = createSolanaRpcApi;
//# sourceMappingURL=index.browser.cjs.map
//# sourceMappingURL=index.browser.cjs.map