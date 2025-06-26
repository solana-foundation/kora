'use strict';

var rpcSubscriptionsSpec = require('@solana/rpc-subscriptions-spec');
var rpcTransformers = require('@solana/rpc-transformers');

// src/index.ts
function createSolanaRpcSubscriptionsApi_INTERNAL(config) {
  const requestTransformer = rpcTransformers.getDefaultRequestTransformerForSolanaRpc(config);
  const responseTransformer = rpcTransformers.getDefaultResponseTransformerForSolanaRpcSubscriptions({
    allowedNumericKeyPaths: getAllowedNumericKeypaths()
  });
  return rpcSubscriptionsSpec.createRpcSubscriptionsApi({
    planExecutor({ request, ...rest }) {
      return rpcSubscriptionsSpec.executeRpcPubSubSubscriptionPlan({
        ...rest,
        responseTransformer,
        subscribeRequest: { ...request, methodName: request.methodName.replace(/Notifications$/, "Subscribe") },
        unsubscribeMethodName: request.methodName.replace(/Notifications$/, "Unsubscribe")
      });
    },
    requestTransformer
  });
}
function createSolanaRpcSubscriptionsApi(config) {
  return createSolanaRpcSubscriptionsApi_INTERNAL(config);
}
function createSolanaRpcSubscriptionsApi_UNSTABLE(config) {
  return createSolanaRpcSubscriptionsApi_INTERNAL(
    config
  );
}
var memoizedKeypaths;
function getAllowedNumericKeypaths() {
  if (!memoizedKeypaths) {
    memoizedKeypaths = {
      accountNotifications: rpcTransformers.jsonParsedAccountsConfigs.map((c) => ["value", ...c]),
      blockNotifications: [
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "preTokenBalances",
          rpcTransformers.KEYPATH_WILDCARD,
          "accountIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "preTokenBalances",
          rpcTransformers.KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "postTokenBalances",
          rpcTransformers.KEYPATH_WILDCARD,
          "accountIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "postTokenBalances",
          rpcTransformers.KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        ["value", "block", "transactions", rpcTransformers.KEYPATH_WILDCARD, "meta", "rewards", rpcTransformers.KEYPATH_WILDCARD, "commission"],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          rpcTransformers.KEYPATH_WILDCARD,
          "index"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          rpcTransformers.KEYPATH_WILDCARD,
          "instructions",
          rpcTransformers.KEYPATH_WILDCARD,
          "programIdIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          rpcTransformers.KEYPATH_WILDCARD,
          "instructions",
          rpcTransformers.KEYPATH_WILDCARD,
          "accounts",
          rpcTransformers.KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "transaction",
          "message",
          "addressTableLookups",
          rpcTransformers.KEYPATH_WILDCARD,
          "writableIndexes",
          rpcTransformers.KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "transaction",
          "message",
          "addressTableLookups",
          rpcTransformers.KEYPATH_WILDCARD,
          "readonlyIndexes",
          rpcTransformers.KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "transaction",
          "message",
          "instructions",
          rpcTransformers.KEYPATH_WILDCARD,
          "programIdIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "transaction",
          "message",
          "instructions",
          rpcTransformers.KEYPATH_WILDCARD,
          "accounts",
          rpcTransformers.KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "transaction",
          "message",
          "header",
          "numReadonlySignedAccounts"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "transaction",
          "message",
          "header",
          "numReadonlyUnsignedAccounts"
        ],
        [
          "value",
          "block",
          "transactions",
          rpcTransformers.KEYPATH_WILDCARD,
          "transaction",
          "message",
          "header",
          "numRequiredSignatures"
        ],
        ["value", "block", "rewards", rpcTransformers.KEYPATH_WILDCARD, "commission"]
      ],
      programNotifications: rpcTransformers.jsonParsedAccountsConfigs.flatMap((c) => [
        ["value", rpcTransformers.KEYPATH_WILDCARD, "account", ...c],
        [rpcTransformers.KEYPATH_WILDCARD, "account", ...c]
      ])
    };
  }
  return memoizedKeypaths;
}

exports.createSolanaRpcSubscriptionsApi = createSolanaRpcSubscriptionsApi;
exports.createSolanaRpcSubscriptionsApi_UNSTABLE = createSolanaRpcSubscriptionsApi_UNSTABLE;
//# sourceMappingURL=index.node.cjs.map
//# sourceMappingURL=index.node.cjs.map