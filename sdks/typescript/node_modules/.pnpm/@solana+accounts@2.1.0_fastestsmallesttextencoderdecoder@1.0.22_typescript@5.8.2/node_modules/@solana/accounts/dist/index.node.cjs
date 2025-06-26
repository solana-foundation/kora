'use strict';

var errors = require('@solana/errors');
var codecsStrings = require('@solana/codecs-strings');

// src/account.ts
var BASE_ACCOUNT_SIZE = 128;
function decodeAccount(encodedAccount, decoder) {
  try {
    if ("exists" in encodedAccount && !encodedAccount.exists) {
      return encodedAccount;
    }
    return Object.freeze({ ...encodedAccount, data: decoder.decode(encodedAccount.data) });
  } catch {
    throw new errors.SolanaError(errors.SOLANA_ERROR__ACCOUNTS__FAILED_TO_DECODE_ACCOUNT, {
      address: encodedAccount.address
    });
  }
}
function accountExists(account) {
  return !("exists" in account) || "exists" in account && account.exists;
}
function assertAccountDecoded(account) {
  if (accountExists(account) && account.data instanceof Uint8Array) {
    throw new errors.SolanaError(errors.SOLANA_ERROR__ACCOUNTS__EXPECTED_DECODED_ACCOUNT, {
      address: account.address
    });
  }
}
function assertAccountsDecoded(accounts) {
  const encoded = accounts.filter((a) => accountExists(a) && a.data instanceof Uint8Array);
  if (encoded.length > 0) {
    const encodedAddresses = encoded.map((a) => a.address);
    throw new errors.SolanaError(errors.SOLANA_ERROR__ACCOUNTS__EXPECTED_ALL_ACCOUNTS_TO_BE_DECODED, {
      addresses: encodedAddresses
    });
  }
}
function parseBase64RpcAccount(address, rpcAccount) {
  if (!rpcAccount) return Object.freeze({ address, exists: false });
  const data = codecsStrings.getBase64Encoder().encode(rpcAccount.data[0]);
  return Object.freeze({ ...parseBaseAccount(rpcAccount), address, data, exists: true });
}
function parseBase58RpcAccount(address, rpcAccount) {
  if (!rpcAccount) return Object.freeze({ address, exists: false });
  const data = codecsStrings.getBase58Encoder().encode(typeof rpcAccount.data === "string" ? rpcAccount.data : rpcAccount.data[0]);
  return Object.freeze({ ...parseBaseAccount(rpcAccount), address, data, exists: true });
}
function parseJsonRpcAccount(address, rpcAccount) {
  if (!rpcAccount) return Object.freeze({ address, exists: false });
  const data = rpcAccount.data.parsed.info;
  return Object.freeze({ ...parseBaseAccount(rpcAccount), address, data, exists: true });
}
function parseBaseAccount(rpcAccount) {
  return Object.freeze({
    executable: rpcAccount.executable,
    lamports: rpcAccount.lamports,
    programAddress: rpcAccount.owner,
    space: rpcAccount.space
  });
}

// src/fetch-account.ts
async function fetchEncodedAccount(rpc, address, config = {}) {
  const { abortSignal, ...rpcConfig } = config;
  const response = await rpc.getAccountInfo(address, { ...rpcConfig, encoding: "base64" }).send({ abortSignal });
  return parseBase64RpcAccount(address, response.value);
}
async function fetchJsonParsedAccount(rpc, address, config = {}) {
  const { abortSignal, ...rpcConfig } = config;
  const { value: account } = await rpc.getAccountInfo(address, { ...rpcConfig, encoding: "jsonParsed" }).send({ abortSignal });
  return !!account && typeof account === "object" && "parsed" in account.data ? parseJsonRpcAccount(address, account) : parseBase64RpcAccount(address, account);
}
async function fetchEncodedAccounts(rpc, addresses, config = {}) {
  const { abortSignal, ...rpcConfig } = config;
  const response = await rpc.getMultipleAccounts(addresses, { ...rpcConfig, encoding: "base64" }).send({ abortSignal });
  return response.value.map((account, index) => parseBase64RpcAccount(addresses[index], account));
}
async function fetchJsonParsedAccounts(rpc, addresses, config = {}) {
  const { abortSignal, ...rpcConfig } = config;
  const response = await rpc.getMultipleAccounts(addresses, { ...rpcConfig, encoding: "jsonParsed" }).send({ abortSignal });
  return response.value.map((account, index) => {
    return !!account && typeof account === "object" && "parsed" in account.data ? parseJsonRpcAccount(addresses[index], account) : parseBase64RpcAccount(addresses[index], account);
  });
}
function assertAccountExists(account) {
  if (!account.exists) {
    throw new errors.SolanaError(errors.SOLANA_ERROR__ACCOUNTS__ACCOUNT_NOT_FOUND, { address: account.address });
  }
}
function assertAccountsExist(accounts) {
  const missingAccounts = accounts.filter((a) => !a.exists);
  if (missingAccounts.length > 0) {
    const missingAddresses = missingAccounts.map((a) => a.address);
    throw new errors.SolanaError(errors.SOLANA_ERROR__ACCOUNTS__ONE_OR_MORE_ACCOUNTS_NOT_FOUND, { addresses: missingAddresses });
  }
}

exports.BASE_ACCOUNT_SIZE = BASE_ACCOUNT_SIZE;
exports.assertAccountDecoded = assertAccountDecoded;
exports.assertAccountExists = assertAccountExists;
exports.assertAccountsDecoded = assertAccountsDecoded;
exports.assertAccountsExist = assertAccountsExist;
exports.decodeAccount = decodeAccount;
exports.fetchEncodedAccount = fetchEncodedAccount;
exports.fetchEncodedAccounts = fetchEncodedAccounts;
exports.fetchJsonParsedAccount = fetchJsonParsedAccount;
exports.fetchJsonParsedAccounts = fetchJsonParsedAccounts;
exports.parseBase58RpcAccount = parseBase58RpcAccount;
exports.parseBase64RpcAccount = parseBase64RpcAccount;
exports.parseJsonRpcAccount = parseJsonRpcAccount;
//# sourceMappingURL=index.node.cjs.map
//# sourceMappingURL=index.node.cjs.map