import { createRpcSubscriptionsApi, executeRpcPubSubSubscriptionPlan } from '@solana/rpc-subscriptions-spec';
import { getDefaultRequestTransformerForSolanaRpc, getDefaultResponseTransformerForSolanaRpcSubscriptions, jsonParsedAccountsConfigs, KEYPATH_WILDCARD } from '@solana/rpc-transformers';

// src/index.ts
function createSolanaRpcSubscriptionsApi_INTERNAL(config) {
  const requestTransformer = getDefaultRequestTransformerForSolanaRpc(config);
  const responseTransformer = getDefaultResponseTransformerForSolanaRpcSubscriptions({
    allowedNumericKeyPaths: getAllowedNumericKeypaths()
  });
  return createRpcSubscriptionsApi({
    planExecutor({ request, ...rest }) {
      return executeRpcPubSubSubscriptionPlan({
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
      accountNotifications: jsonParsedAccountsConfigs.map((c) => ["value", ...c]),
      blockNotifications: [
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "preTokenBalances",
          KEYPATH_WILDCARD,
          "accountIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "preTokenBalances",
          KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "postTokenBalances",
          KEYPATH_WILDCARD,
          "accountIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "postTokenBalances",
          KEYPATH_WILDCARD,
          "uiTokenAmount",
          "decimals"
        ],
        ["value", "block", "transactions", KEYPATH_WILDCARD, "meta", "rewards", KEYPATH_WILDCARD, "commission"],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          KEYPATH_WILDCARD,
          "index"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          KEYPATH_WILDCARD,
          "instructions",
          KEYPATH_WILDCARD,
          "programIdIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "meta",
          "innerInstructions",
          KEYPATH_WILDCARD,
          "instructions",
          KEYPATH_WILDCARD,
          "accounts",
          KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "transaction",
          "message",
          "addressTableLookups",
          KEYPATH_WILDCARD,
          "writableIndexes",
          KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "transaction",
          "message",
          "addressTableLookups",
          KEYPATH_WILDCARD,
          "readonlyIndexes",
          KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "transaction",
          "message",
          "instructions",
          KEYPATH_WILDCARD,
          "programIdIndex"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "transaction",
          "message",
          "instructions",
          KEYPATH_WILDCARD,
          "accounts",
          KEYPATH_WILDCARD
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "transaction",
          "message",
          "header",
          "numReadonlySignedAccounts"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "transaction",
          "message",
          "header",
          "numReadonlyUnsignedAccounts"
        ],
        [
          "value",
          "block",
          "transactions",
          KEYPATH_WILDCARD,
          "transaction",
          "message",
          "header",
          "numRequiredSignatures"
        ],
        ["value", "block", "rewards", KEYPATH_WILDCARD, "commission"]
      ],
      programNotifications: jsonParsedAccountsConfigs.flatMap((c) => [
        ["value", KEYPATH_WILDCARD, "account", ...c],
        [KEYPATH_WILDCARD, "account", ...c]
      ])
    };
  }
  return memoizedKeypaths;
}

export { createSolanaRpcSubscriptionsApi, createSolanaRpcSubscriptionsApi_UNSTABLE };
//# sourceMappingURL=index.browser.mjs.map
//# sourceMappingURL=index.browser.mjs.map