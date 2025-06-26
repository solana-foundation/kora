'use strict';

var errors = require('@solana/errors');
var codecsStrings = require('@solana/codecs-strings');
var promises = require('@solana/promises');
var rpcTypes = require('@solana/rpc-types');
var transactions = require('@solana/transactions');

// src/confirmation-strategy-blockheight.ts

// ../event-target-impl/dist/index.browser.mjs
var o = globalThis.AbortController;

// src/confirmation-strategy-blockheight.ts
function createBlockHeightExceedencePromiseFactory({
  rpc,
  rpcSubscriptions
}) {
  return async function getBlockHeightExceedencePromise({
    abortSignal: callerAbortSignal,
    commitment,
    lastValidBlockHeight
  }) {
    callerAbortSignal.throwIfAborted();
    const abortController = new o();
    const handleAbort = () => {
      abortController.abort();
    };
    callerAbortSignal.addEventListener("abort", handleAbort, { signal: abortController.signal });
    async function getBlockHeightAndDifferenceBetweenSlotHeightAndBlockHeight() {
      const { absoluteSlot, blockHeight } = await rpc.getEpochInfo({ commitment }).send({ abortSignal: abortController.signal });
      return {
        blockHeight,
        differenceBetweenSlotHeightAndBlockHeight: absoluteSlot - blockHeight
      };
    }
    try {
      const [slotNotifications, { blockHeight: initialBlockHeight, differenceBetweenSlotHeightAndBlockHeight }] = await Promise.all([
        rpcSubscriptions.slotNotifications().subscribe({ abortSignal: abortController.signal }),
        getBlockHeightAndDifferenceBetweenSlotHeightAndBlockHeight()
      ]);
      callerAbortSignal.throwIfAborted();
      let currentBlockHeight = initialBlockHeight;
      if (currentBlockHeight <= lastValidBlockHeight) {
        let lastKnownDifferenceBetweenSlotHeightAndBlockHeight = differenceBetweenSlotHeightAndBlockHeight;
        for await (const slotNotification of slotNotifications) {
          const { slot } = slotNotification;
          if (slot - lastKnownDifferenceBetweenSlotHeightAndBlockHeight > lastValidBlockHeight) {
            const {
              blockHeight: recheckedBlockHeight,
              differenceBetweenSlotHeightAndBlockHeight: currentDifferenceBetweenSlotHeightAndBlockHeight
            } = await getBlockHeightAndDifferenceBetweenSlotHeightAndBlockHeight();
            currentBlockHeight = recheckedBlockHeight;
            if (currentBlockHeight > lastValidBlockHeight) {
              break;
            } else {
              lastKnownDifferenceBetweenSlotHeightAndBlockHeight = currentDifferenceBetweenSlotHeightAndBlockHeight;
            }
          }
        }
      }
      callerAbortSignal.throwIfAborted();
      throw new errors.SolanaError(errors.SOLANA_ERROR__BLOCK_HEIGHT_EXCEEDED, {
        currentBlockHeight,
        lastValidBlockHeight
      });
    } finally {
      abortController.abort();
    }
  };
}
var NONCE_VALUE_OFFSET = 4 + // version(u32)
4 + // state(u32)
32;
function createNonceInvalidationPromiseFactory({
  rpc,
  rpcSubscriptions
}) {
  return async function getNonceInvalidationPromise({
    abortSignal: callerAbortSignal,
    commitment,
    currentNonceValue: expectedNonceValue,
    nonceAccountAddress
  }) {
    const abortController = new o();
    function handleAbort() {
      abortController.abort();
    }
    callerAbortSignal.addEventListener("abort", handleAbort, { signal: abortController.signal });
    const accountNotifications = await rpcSubscriptions.accountNotifications(nonceAccountAddress, { commitment, encoding: "base64" }).subscribe({ abortSignal: abortController.signal });
    const base58Decoder = codecsStrings.getBase58Decoder();
    const base64Encoder = codecsStrings.getBase64Encoder();
    function getNonceFromAccountData([base64EncodedBytes]) {
      const data = base64Encoder.encode(base64EncodedBytes);
      const nonceValueBytes = data.slice(NONCE_VALUE_OFFSET, NONCE_VALUE_OFFSET + 32);
      return base58Decoder.decode(nonceValueBytes);
    }
    const nonceAccountDidAdvancePromise = (async () => {
      for await (const accountNotification of accountNotifications) {
        const nonceValue = getNonceFromAccountData(accountNotification.value.data);
        if (nonceValue !== expectedNonceValue) {
          throw new errors.SolanaError(errors.SOLANA_ERROR__INVALID_NONCE, {
            actualNonceValue: nonceValue,
            expectedNonceValue
          });
        }
      }
    })();
    const nonceIsAlreadyInvalidPromise = (async () => {
      const { value: nonceAccount } = await rpc.getAccountInfo(nonceAccountAddress, {
        commitment,
        dataSlice: { length: 32, offset: NONCE_VALUE_OFFSET },
        encoding: "base58"
      }).send({ abortSignal: abortController.signal });
      if (!nonceAccount) {
        throw new errors.SolanaError(errors.SOLANA_ERROR__NONCE_ACCOUNT_NOT_FOUND, {
          nonceAccountAddress
        });
      }
      const nonceValue = (
        // This works because we asked for the exact slice of data representing the nonce
        // value, and furthermore asked for it in `base58` encoding.
        nonceAccount.data[0]
      );
      if (nonceValue !== expectedNonceValue) {
        throw new errors.SolanaError(errors.SOLANA_ERROR__INVALID_NONCE, {
          actualNonceValue: nonceValue,
          expectedNonceValue
        });
      } else {
        await new Promise(() => {
        });
      }
    })();
    try {
      return await promises.safeRace([nonceAccountDidAdvancePromise, nonceIsAlreadyInvalidPromise]);
    } finally {
      abortController.abort();
    }
  };
}
function createRecentSignatureConfirmationPromiseFactory({
  rpc,
  rpcSubscriptions
}) {
  return async function getRecentSignatureConfirmationPromise({
    abortSignal: callerAbortSignal,
    commitment,
    signature
  }) {
    const abortController = new o();
    function handleAbort() {
      abortController.abort();
    }
    callerAbortSignal.addEventListener("abort", handleAbort, { signal: abortController.signal });
    const signatureStatusNotifications = await rpcSubscriptions.signatureNotifications(signature, { commitment }).subscribe({ abortSignal: abortController.signal });
    const signatureDidCommitPromise = (async () => {
      for await (const signatureStatusNotification of signatureStatusNotifications) {
        if (signatureStatusNotification.value.err) {
          throw errors.getSolanaErrorFromTransactionError(signatureStatusNotification.value.err);
        } else {
          return;
        }
      }
    })();
    const signatureStatusLookupPromise = (async () => {
      const { value: signatureStatusResults } = await rpc.getSignatureStatuses([signature]).send({ abortSignal: abortController.signal });
      const signatureStatus = signatureStatusResults[0];
      if (signatureStatus && signatureStatus.confirmationStatus && rpcTypes.commitmentComparator(signatureStatus.confirmationStatus, commitment) >= 0) {
        return;
      } else {
        await new Promise(() => {
        });
      }
    })();
    try {
      return await promises.safeRace([signatureDidCommitPromise, signatureStatusLookupPromise]);
    } finally {
      abortController.abort();
    }
  };
}

// src/confirmation-strategy-timeout.ts
async function getTimeoutPromise({ abortSignal: callerAbortSignal, commitment }) {
  return await new Promise((_, reject) => {
    const handleAbort = (e) => {
      clearTimeout(timeoutId);
      const abortError = new DOMException(e.target.reason, "AbortError");
      reject(abortError);
    };
    callerAbortSignal.addEventListener("abort", handleAbort);
    const timeoutMs = commitment === "processed" ? 3e4 : 6e4;
    const startMs = performance.now();
    const timeoutId = (
      // We use `setTimeout` instead of `AbortSignal.timeout()` because we want to measure
      // elapsed time instead of active time.
      // See https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal/timeout_static
      setTimeout(() => {
        const elapsedMs = performance.now() - startMs;
        reject(new DOMException(`Timeout elapsed after ${elapsedMs} ms`, "TimeoutError"));
      }, timeoutMs)
    );
  });
}
async function raceStrategies(signature, config, getSpecificStrategiesForRace) {
  const { abortSignal: callerAbortSignal, commitment, getRecentSignatureConfirmationPromise } = config;
  callerAbortSignal?.throwIfAborted();
  const abortController = new o();
  if (callerAbortSignal) {
    const handleAbort = () => {
      abortController.abort();
    };
    callerAbortSignal.addEventListener("abort", handleAbort, { signal: abortController.signal });
  }
  try {
    const specificStrategies = getSpecificStrategiesForRace({
      ...config,
      abortSignal: abortController.signal
    });
    return await promises.safeRace([
      getRecentSignatureConfirmationPromise({
        abortSignal: abortController.signal,
        commitment,
        signature
      }),
      ...specificStrategies
    ]);
  } finally {
    abortController.abort();
  }
}

// src/waiters.ts
async function waitForDurableNonceTransactionConfirmation(config) {
  await raceStrategies(
    transactions.getSignatureFromTransaction(config.transaction),
    config,
    function getSpecificStrategiesForRace({ abortSignal, commitment, getNonceInvalidationPromise, transaction }) {
      return [
        getNonceInvalidationPromise({
          abortSignal,
          commitment,
          currentNonceValue: transaction.lifetimeConstraint.nonce,
          nonceAccountAddress: transaction.lifetimeConstraint.nonceAccountAddress
        })
      ];
    }
  );
}
async function waitForRecentTransactionConfirmation(config) {
  await raceStrategies(
    transactions.getSignatureFromTransaction(config.transaction),
    config,
    function getSpecificStrategiesForRace({
      abortSignal,
      commitment,
      getBlockHeightExceedencePromise,
      transaction
    }) {
      return [
        getBlockHeightExceedencePromise({
          abortSignal,
          commitment,
          lastValidBlockHeight: transaction.lifetimeConstraint.lastValidBlockHeight
        })
      ];
    }
  );
}
async function waitForRecentTransactionConfirmationUntilTimeout(config) {
  await raceStrategies(
    config.signature,
    config,
    function getSpecificStrategiesForRace({ abortSignal, commitment, getTimeoutPromise: getTimeoutPromise2 }) {
      return [
        getTimeoutPromise2({
          abortSignal,
          commitment
        })
      ];
    }
  );
}

exports.createBlockHeightExceedencePromiseFactory = createBlockHeightExceedencePromiseFactory;
exports.createNonceInvalidationPromiseFactory = createNonceInvalidationPromiseFactory;
exports.createRecentSignatureConfirmationPromiseFactory = createRecentSignatureConfirmationPromiseFactory;
exports.getTimeoutPromise = getTimeoutPromise;
exports.waitForDurableNonceTransactionConfirmation = waitForDurableNonceTransactionConfirmation;
exports.waitForRecentTransactionConfirmation = waitForRecentTransactionConfirmation;
exports.waitForRecentTransactionConfirmationUntilTimeout = waitForRecentTransactionConfirmationUntilTimeout;
//# sourceMappingURL=index.browser.cjs.map
//# sourceMappingURL=index.browser.cjs.map