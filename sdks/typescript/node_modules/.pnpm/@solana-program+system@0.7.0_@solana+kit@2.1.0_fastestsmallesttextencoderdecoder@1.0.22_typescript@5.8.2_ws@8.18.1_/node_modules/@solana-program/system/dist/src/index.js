'use strict';

var kit = require('@solana/kit');

// src/generated/accounts/nonce.ts
var NonceState = /* @__PURE__ */ ((NonceState2) => {
  NonceState2[NonceState2["Uninitialized"] = 0] = "Uninitialized";
  NonceState2[NonceState2["Initialized"] = 1] = "Initialized";
  return NonceState2;
})(NonceState || {});
function getNonceStateEncoder() {
  return kit.getEnumEncoder(NonceState, { size: kit.getU32Encoder() });
}
function getNonceStateDecoder() {
  return kit.getEnumDecoder(NonceState, { size: kit.getU32Decoder() });
}
function getNonceStateCodec() {
  return kit.combineCodec(getNonceStateEncoder(), getNonceStateDecoder());
}
var NonceVersion = /* @__PURE__ */ ((NonceVersion2) => {
  NonceVersion2[NonceVersion2["Legacy"] = 0] = "Legacy";
  NonceVersion2[NonceVersion2["Current"] = 1] = "Current";
  return NonceVersion2;
})(NonceVersion || {});
function getNonceVersionEncoder() {
  return kit.getEnumEncoder(NonceVersion, { size: kit.getU32Encoder() });
}
function getNonceVersionDecoder() {
  return kit.getEnumDecoder(NonceVersion, { size: kit.getU32Decoder() });
}
function getNonceVersionCodec() {
  return kit.combineCodec(getNonceVersionEncoder(), getNonceVersionDecoder());
}

// src/generated/accounts/nonce.ts
function getNonceEncoder() {
  return kit.getStructEncoder([
    ["version", getNonceVersionEncoder()],
    ["state", getNonceStateEncoder()],
    ["authority", kit.getAddressEncoder()],
    ["blockhash", kit.getAddressEncoder()],
    ["lamportsPerSignature", kit.getU64Encoder()]
  ]);
}
function getNonceDecoder() {
  return kit.getStructDecoder([
    ["version", getNonceVersionDecoder()],
    ["state", getNonceStateDecoder()],
    ["authority", kit.getAddressDecoder()],
    ["blockhash", kit.getAddressDecoder()],
    ["lamportsPerSignature", kit.getU64Decoder()]
  ]);
}
function getNonceCodec() {
  return kit.combineCodec(getNonceEncoder(), getNonceDecoder());
}
function decodeNonce(encodedAccount) {
  return kit.decodeAccount(
    encodedAccount,
    getNonceDecoder()
  );
}
async function fetchNonce(rpc, address, config) {
  const maybeAccount = await fetchMaybeNonce(rpc, address, config);
  kit.assertAccountExists(maybeAccount);
  return maybeAccount;
}
async function fetchMaybeNonce(rpc, address, config) {
  const maybeAccount = await kit.fetchEncodedAccount(rpc, address, config);
  return decodeNonce(maybeAccount);
}
async function fetchAllNonce(rpc, addresses, config) {
  const maybeAccounts = await fetchAllMaybeNonce(rpc, addresses, config);
  kit.assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}
async function fetchAllMaybeNonce(rpc, addresses, config) {
  const maybeAccounts = await kit.fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) => decodeNonce(maybeAccount));
}
function getNonceSize() {
  return 80;
}
var SYSTEM_PROGRAM_ADDRESS = "11111111111111111111111111111111";
var SystemAccount = /* @__PURE__ */ ((SystemAccount2) => {
  SystemAccount2[SystemAccount2["Nonce"] = 0] = "Nonce";
  return SystemAccount2;
})(SystemAccount || {});
var SystemInstruction = /* @__PURE__ */ ((SystemInstruction2) => {
  SystemInstruction2[SystemInstruction2["CreateAccount"] = 0] = "CreateAccount";
  SystemInstruction2[SystemInstruction2["Assign"] = 1] = "Assign";
  SystemInstruction2[SystemInstruction2["TransferSol"] = 2] = "TransferSol";
  SystemInstruction2[SystemInstruction2["CreateAccountWithSeed"] = 3] = "CreateAccountWithSeed";
  SystemInstruction2[SystemInstruction2["AdvanceNonceAccount"] = 4] = "AdvanceNonceAccount";
  SystemInstruction2[SystemInstruction2["WithdrawNonceAccount"] = 5] = "WithdrawNonceAccount";
  SystemInstruction2[SystemInstruction2["InitializeNonceAccount"] = 6] = "InitializeNonceAccount";
  SystemInstruction2[SystemInstruction2["AuthorizeNonceAccount"] = 7] = "AuthorizeNonceAccount";
  SystemInstruction2[SystemInstruction2["Allocate"] = 8] = "Allocate";
  SystemInstruction2[SystemInstruction2["AllocateWithSeed"] = 9] = "AllocateWithSeed";
  SystemInstruction2[SystemInstruction2["AssignWithSeed"] = 10] = "AssignWithSeed";
  SystemInstruction2[SystemInstruction2["TransferSolWithSeed"] = 11] = "TransferSolWithSeed";
  SystemInstruction2[SystemInstruction2["UpgradeNonceAccount"] = 12] = "UpgradeNonceAccount";
  return SystemInstruction2;
})(SystemInstruction || {});
function identifySystemInstruction(instruction) {
  const data = "data" in instruction ? instruction.data : instruction;
  if (kit.containsBytes(data, kit.getU32Encoder().encode(0), 0)) {
    return 0 /* CreateAccount */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(1), 0)) {
    return 1 /* Assign */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(2), 0)) {
    return 2 /* TransferSol */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(3), 0)) {
    return 3 /* CreateAccountWithSeed */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(4), 0)) {
    return 4 /* AdvanceNonceAccount */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(5), 0)) {
    return 5 /* WithdrawNonceAccount */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(6), 0)) {
    return 6 /* InitializeNonceAccount */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(7), 0)) {
    return 7 /* AuthorizeNonceAccount */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(8), 0)) {
    return 8 /* Allocate */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(9), 0)) {
    return 9 /* AllocateWithSeed */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(10), 0)) {
    return 10 /* AssignWithSeed */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(11), 0)) {
    return 11 /* TransferSolWithSeed */;
  }
  if (kit.containsBytes(data, kit.getU32Encoder().encode(12), 0)) {
    return 12 /* UpgradeNonceAccount */;
  }
  throw new Error(
    "The provided instruction could not be identified as a system instruction."
  );
}

// src/generated/errors/system.ts
var SYSTEM_ERROR__ACCOUNT_ALREADY_IN_USE = 0;
var SYSTEM_ERROR__RESULT_WITH_NEGATIVE_LAMPORTS = 1;
var SYSTEM_ERROR__INVALID_PROGRAM_ID = 2;
var SYSTEM_ERROR__INVALID_ACCOUNT_DATA_LENGTH = 3;
var SYSTEM_ERROR__MAX_SEED_LENGTH_EXCEEDED = 4;
var SYSTEM_ERROR__ADDRESS_WITH_SEED_MISMATCH = 5;
var SYSTEM_ERROR__NONCE_NO_RECENT_BLOCKHASHES = 6;
var SYSTEM_ERROR__NONCE_BLOCKHASH_NOT_EXPIRED = 7;
var SYSTEM_ERROR__NONCE_UNEXPECTED_BLOCKHASH_VALUE = 8;
var systemErrorMessages;
if (process.env.NODE_ENV !== "production") {
  systemErrorMessages = {
    [SYSTEM_ERROR__ACCOUNT_ALREADY_IN_USE]: `an account with the same address already exists`,
    [SYSTEM_ERROR__ADDRESS_WITH_SEED_MISMATCH]: `provided address does not match addressed derived from seed`,
    [SYSTEM_ERROR__INVALID_ACCOUNT_DATA_LENGTH]: `cannot allocate account data of this length`,
    [SYSTEM_ERROR__INVALID_PROGRAM_ID]: `cannot assign account to this program id`,
    [SYSTEM_ERROR__MAX_SEED_LENGTH_EXCEEDED]: `length of requested seed is too long`,
    [SYSTEM_ERROR__NONCE_BLOCKHASH_NOT_EXPIRED]: `stored nonce is still in recent_blockhashes`,
    [SYSTEM_ERROR__NONCE_NO_RECENT_BLOCKHASHES]: `advancing stored nonce requires a populated RecentBlockhashes sysvar`,
    [SYSTEM_ERROR__NONCE_UNEXPECTED_BLOCKHASH_VALUE]: `specified nonce does not match stored nonce`,
    [SYSTEM_ERROR__RESULT_WITH_NEGATIVE_LAMPORTS]: `account does not have enough SOL to perform the operation`
  };
}
function getSystemErrorMessage(code) {
  if (process.env.NODE_ENV !== "production") {
    return systemErrorMessages[code];
  }
  return "Error message not available in production bundles.";
}
function isSystemError(error, transactionMessage, code) {
  return kit.isProgramError(
    error,
    transactionMessage,
    SYSTEM_PROGRAM_ADDRESS,
    code
  );
}
function expectAddress(value) {
  if (!value) {
    throw new Error("Expected a Address.");
  }
  if (typeof value === "object" && "address" in value) {
    return value.address;
  }
  if (Array.isArray(value)) {
    return value[0];
  }
  return value;
}
function getAccountMetaFactory(programAddress, optionalAccountStrategy) {
  return (account) => {
    if (!account.value) {
      return Object.freeze({
        address: programAddress,
        role: kit.AccountRole.READONLY
      });
    }
    const writableRole = account.isWritable ? kit.AccountRole.WRITABLE : kit.AccountRole.READONLY;
    return Object.freeze({
      address: expectAddress(account.value),
      role: isTransactionSigner(account.value) ? kit.upgradeRoleToSigner(writableRole) : writableRole,
      ...isTransactionSigner(account.value) ? { signer: account.value } : {}
    });
  };
}
function isTransactionSigner(value) {
  return !!value && typeof value === "object" && "address" in value && kit.isTransactionSigner(value);
}

// src/generated/instructions/advanceNonceAccount.ts
var ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR = 4;
function getAdvanceNonceAccountDiscriminatorBytes() {
  return kit.getU32Encoder().encode(ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getAdvanceNonceAccountInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([["discriminator", kit.getU32Encoder()]]),
    (value) => ({
      ...value,
      discriminator: ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getAdvanceNonceAccountInstructionDataDecoder() {
  return kit.getStructDecoder([["discriminator", kit.getU32Decoder()]]);
}
function getAdvanceNonceAccountInstructionDataCodec() {
  return kit.combineCodec(
    getAdvanceNonceAccountInstructionDataEncoder(),
    getAdvanceNonceAccountInstructionDataDecoder()
  );
}
function getAdvanceNonceAccountInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    nonceAccount: { value: input.nonceAccount ?? null, isWritable: true },
    recentBlockhashesSysvar: {
      value: input.recentBlockhashesSysvar ?? null,
      isWritable: false
    },
    nonceAuthority: { value: input.nonceAuthority ?? null, isWritable: false }
  };
  const accounts = originalAccounts;
  if (!accounts.recentBlockhashesSysvar.value) {
    accounts.recentBlockhashesSysvar.value = "SysvarRecentB1ockHashes11111111111111111111";
  }
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.nonceAccount),
      getAccountMeta(accounts.recentBlockhashesSysvar),
      getAccountMeta(accounts.nonceAuthority)
    ],
    programAddress,
    data: getAdvanceNonceAccountInstructionDataEncoder().encode({})
  };
  return instruction;
}
function parseAdvanceNonceAccountInstruction(instruction) {
  if (instruction.accounts.length < 3) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      nonceAccount: getNextAccount(),
      recentBlockhashesSysvar: getNextAccount(),
      nonceAuthority: getNextAccount()
    },
    data: getAdvanceNonceAccountInstructionDataDecoder().decode(
      instruction.data
    )
  };
}
var ALLOCATE_DISCRIMINATOR = 8;
function getAllocateDiscriminatorBytes() {
  return kit.getU32Encoder().encode(ALLOCATE_DISCRIMINATOR);
}
function getAllocateInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["space", kit.getU64Encoder()]
    ]),
    (value) => ({ ...value, discriminator: ALLOCATE_DISCRIMINATOR })
  );
}
function getAllocateInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["space", kit.getU64Decoder()]
  ]);
}
function getAllocateInstructionDataCodec() {
  return kit.combineCodec(
    getAllocateInstructionDataEncoder(),
    getAllocateInstructionDataDecoder()
  );
}
function getAllocateInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    newAccount: { value: input.newAccount ?? null, isWritable: true }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [getAccountMeta(accounts.newAccount)],
    programAddress,
    data: getAllocateInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseAllocateInstruction(instruction) {
  if (instruction.accounts.length < 1) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      newAccount: getNextAccount()
    },
    data: getAllocateInstructionDataDecoder().decode(instruction.data)
  };
}
var ALLOCATE_WITH_SEED_DISCRIMINATOR = 9;
function getAllocateWithSeedDiscriminatorBytes() {
  return kit.getU32Encoder().encode(ALLOCATE_WITH_SEED_DISCRIMINATOR);
}
function getAllocateWithSeedInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["base", kit.getAddressEncoder()],
      ["seed", kit.addEncoderSizePrefix(kit.getUtf8Encoder(), kit.getU64Encoder())],
      ["space", kit.getU64Encoder()],
      ["programAddress", kit.getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: ALLOCATE_WITH_SEED_DISCRIMINATOR })
  );
}
function getAllocateWithSeedInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["base", kit.getAddressDecoder()],
    ["seed", kit.addDecoderSizePrefix(kit.getUtf8Decoder(), kit.getU64Decoder())],
    ["space", kit.getU64Decoder()],
    ["programAddress", kit.getAddressDecoder()]
  ]);
}
function getAllocateWithSeedInstructionDataCodec() {
  return kit.combineCodec(
    getAllocateWithSeedInstructionDataEncoder(),
    getAllocateWithSeedInstructionDataDecoder()
  );
}
function getAllocateWithSeedInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    newAccount: { value: input.newAccount ?? null, isWritable: true },
    baseAccount: { value: input.baseAccount ?? null, isWritable: false }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.newAccount),
      getAccountMeta(accounts.baseAccount)
    ],
    programAddress,
    data: getAllocateWithSeedInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseAllocateWithSeedInstruction(instruction) {
  if (instruction.accounts.length < 2) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      newAccount: getNextAccount(),
      baseAccount: getNextAccount()
    },
    data: getAllocateWithSeedInstructionDataDecoder().decode(instruction.data)
  };
}
var ASSIGN_DISCRIMINATOR = 1;
function getAssignDiscriminatorBytes() {
  return kit.getU32Encoder().encode(ASSIGN_DISCRIMINATOR);
}
function getAssignInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["programAddress", kit.getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: ASSIGN_DISCRIMINATOR })
  );
}
function getAssignInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["programAddress", kit.getAddressDecoder()]
  ]);
}
function getAssignInstructionDataCodec() {
  return kit.combineCodec(
    getAssignInstructionDataEncoder(),
    getAssignInstructionDataDecoder()
  );
}
function getAssignInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    account: { value: input.account ?? null, isWritable: true }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [getAccountMeta(accounts.account)],
    programAddress,
    data: getAssignInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseAssignInstruction(instruction) {
  if (instruction.accounts.length < 1) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      account: getNextAccount()
    },
    data: getAssignInstructionDataDecoder().decode(instruction.data)
  };
}
var ASSIGN_WITH_SEED_DISCRIMINATOR = 10;
function getAssignWithSeedDiscriminatorBytes() {
  return kit.getU32Encoder().encode(ASSIGN_WITH_SEED_DISCRIMINATOR);
}
function getAssignWithSeedInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["base", kit.getAddressEncoder()],
      ["seed", kit.addEncoderSizePrefix(kit.getUtf8Encoder(), kit.getU64Encoder())],
      ["programAddress", kit.getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: ASSIGN_WITH_SEED_DISCRIMINATOR })
  );
}
function getAssignWithSeedInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["base", kit.getAddressDecoder()],
    ["seed", kit.addDecoderSizePrefix(kit.getUtf8Decoder(), kit.getU64Decoder())],
    ["programAddress", kit.getAddressDecoder()]
  ]);
}
function getAssignWithSeedInstructionDataCodec() {
  return kit.combineCodec(
    getAssignWithSeedInstructionDataEncoder(),
    getAssignWithSeedInstructionDataDecoder()
  );
}
function getAssignWithSeedInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    account: { value: input.account ?? null, isWritable: true },
    baseAccount: { value: input.baseAccount ?? null, isWritable: false }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.account),
      getAccountMeta(accounts.baseAccount)
    ],
    programAddress,
    data: getAssignWithSeedInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseAssignWithSeedInstruction(instruction) {
  if (instruction.accounts.length < 2) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      account: getNextAccount(),
      baseAccount: getNextAccount()
    },
    data: getAssignWithSeedInstructionDataDecoder().decode(instruction.data)
  };
}
var AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR = 7;
function getAuthorizeNonceAccountDiscriminatorBytes() {
  return kit.getU32Encoder().encode(AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getAuthorizeNonceAccountInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["newNonceAuthority", kit.getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getAuthorizeNonceAccountInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["newNonceAuthority", kit.getAddressDecoder()]
  ]);
}
function getAuthorizeNonceAccountInstructionDataCodec() {
  return kit.combineCodec(
    getAuthorizeNonceAccountInstructionDataEncoder(),
    getAuthorizeNonceAccountInstructionDataDecoder()
  );
}
function getAuthorizeNonceAccountInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    nonceAccount: { value: input.nonceAccount ?? null, isWritable: true },
    nonceAuthority: { value: input.nonceAuthority ?? null, isWritable: false }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.nonceAccount),
      getAccountMeta(accounts.nonceAuthority)
    ],
    programAddress,
    data: getAuthorizeNonceAccountInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseAuthorizeNonceAccountInstruction(instruction) {
  if (instruction.accounts.length < 2) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      nonceAccount: getNextAccount(),
      nonceAuthority: getNextAccount()
    },
    data: getAuthorizeNonceAccountInstructionDataDecoder().decode(
      instruction.data
    )
  };
}
var CREATE_ACCOUNT_DISCRIMINATOR = 0;
function getCreateAccountDiscriminatorBytes() {
  return kit.getU32Encoder().encode(CREATE_ACCOUNT_DISCRIMINATOR);
}
function getCreateAccountInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["lamports", kit.getU64Encoder()],
      ["space", kit.getU64Encoder()],
      ["programAddress", kit.getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: CREATE_ACCOUNT_DISCRIMINATOR })
  );
}
function getCreateAccountInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["lamports", kit.getU64Decoder()],
    ["space", kit.getU64Decoder()],
    ["programAddress", kit.getAddressDecoder()]
  ]);
}
function getCreateAccountInstructionDataCodec() {
  return kit.combineCodec(
    getCreateAccountInstructionDataEncoder(),
    getCreateAccountInstructionDataDecoder()
  );
}
function getCreateAccountInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    payer: { value: input.payer ?? null, isWritable: true },
    newAccount: { value: input.newAccount ?? null, isWritable: true }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const byteDelta = [Number(args.space) + kit.BASE_ACCOUNT_SIZE].reduce(
    (a, b) => a + b,
    0
  );
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.newAccount)
    ],
    programAddress,
    data: getCreateAccountInstructionDataEncoder().encode(
      args
    )
  };
  return Object.freeze({ ...instruction, byteDelta });
}
function parseCreateAccountInstruction(instruction) {
  if (instruction.accounts.length < 2) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      payer: getNextAccount(),
      newAccount: getNextAccount()
    },
    data: getCreateAccountInstructionDataDecoder().decode(instruction.data)
  };
}
var CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR = 3;
function getCreateAccountWithSeedDiscriminatorBytes() {
  return kit.getU32Encoder().encode(CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR);
}
function getCreateAccountWithSeedInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["base", kit.getAddressEncoder()],
      ["seed", kit.addEncoderSizePrefix(kit.getUtf8Encoder(), kit.getU64Encoder())],
      ["amount", kit.getU64Encoder()],
      ["space", kit.getU64Encoder()],
      ["programAddress", kit.getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR
    })
  );
}
function getCreateAccountWithSeedInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["base", kit.getAddressDecoder()],
    ["seed", kit.addDecoderSizePrefix(kit.getUtf8Decoder(), kit.getU64Decoder())],
    ["amount", kit.getU64Decoder()],
    ["space", kit.getU64Decoder()],
    ["programAddress", kit.getAddressDecoder()]
  ]);
}
function getCreateAccountWithSeedInstructionDataCodec() {
  return kit.combineCodec(
    getCreateAccountWithSeedInstructionDataEncoder(),
    getCreateAccountWithSeedInstructionDataDecoder()
  );
}
function getCreateAccountWithSeedInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    payer: { value: input.payer ?? null, isWritable: true },
    newAccount: { value: input.newAccount ?? null, isWritable: true },
    baseAccount: { value: input.baseAccount ?? null, isWritable: false }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.newAccount),
      getAccountMeta(accounts.baseAccount)
    ],
    programAddress,
    data: getCreateAccountWithSeedInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseCreateAccountWithSeedInstruction(instruction) {
  if (instruction.accounts.length < 3) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      payer: getNextAccount(),
      newAccount: getNextAccount(),
      baseAccount: getNextAccount()
    },
    data: getCreateAccountWithSeedInstructionDataDecoder().decode(
      instruction.data
    )
  };
}
var INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR = 6;
function getInitializeNonceAccountDiscriminatorBytes() {
  return kit.getU32Encoder().encode(INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getInitializeNonceAccountInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["nonceAuthority", kit.getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getInitializeNonceAccountInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["nonceAuthority", kit.getAddressDecoder()]
  ]);
}
function getInitializeNonceAccountInstructionDataCodec() {
  return kit.combineCodec(
    getInitializeNonceAccountInstructionDataEncoder(),
    getInitializeNonceAccountInstructionDataDecoder()
  );
}
function getInitializeNonceAccountInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    nonceAccount: { value: input.nonceAccount ?? null, isWritable: true },
    recentBlockhashesSysvar: {
      value: input.recentBlockhashesSysvar ?? null,
      isWritable: false
    },
    rentSysvar: { value: input.rentSysvar ?? null, isWritable: false }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  if (!accounts.recentBlockhashesSysvar.value) {
    accounts.recentBlockhashesSysvar.value = "SysvarRecentB1ockHashes11111111111111111111";
  }
  if (!accounts.rentSysvar.value) {
    accounts.rentSysvar.value = "SysvarRent111111111111111111111111111111111";
  }
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.nonceAccount),
      getAccountMeta(accounts.recentBlockhashesSysvar),
      getAccountMeta(accounts.rentSysvar)
    ],
    programAddress,
    data: getInitializeNonceAccountInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseInitializeNonceAccountInstruction(instruction) {
  if (instruction.accounts.length < 3) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      nonceAccount: getNextAccount(),
      recentBlockhashesSysvar: getNextAccount(),
      rentSysvar: getNextAccount()
    },
    data: getInitializeNonceAccountInstructionDataDecoder().decode(
      instruction.data
    )
  };
}
var TRANSFER_SOL_DISCRIMINATOR = 2;
function getTransferSolDiscriminatorBytes() {
  return kit.getU32Encoder().encode(TRANSFER_SOL_DISCRIMINATOR);
}
function getTransferSolInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["amount", kit.getU64Encoder()]
    ]),
    (value) => ({ ...value, discriminator: TRANSFER_SOL_DISCRIMINATOR })
  );
}
function getTransferSolInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["amount", kit.getU64Decoder()]
  ]);
}
function getTransferSolInstructionDataCodec() {
  return kit.combineCodec(
    getTransferSolInstructionDataEncoder(),
    getTransferSolInstructionDataDecoder()
  );
}
function getTransferSolInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    source: { value: input.source ?? null, isWritable: true },
    destination: { value: input.destination ?? null, isWritable: true }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.source),
      getAccountMeta(accounts.destination)
    ],
    programAddress,
    data: getTransferSolInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseTransferSolInstruction(instruction) {
  if (instruction.accounts.length < 2) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      source: getNextAccount(),
      destination: getNextAccount()
    },
    data: getTransferSolInstructionDataDecoder().decode(instruction.data)
  };
}
var TRANSFER_SOL_WITH_SEED_DISCRIMINATOR = 11;
function getTransferSolWithSeedDiscriminatorBytes() {
  return kit.getU32Encoder().encode(TRANSFER_SOL_WITH_SEED_DISCRIMINATOR);
}
function getTransferSolWithSeedInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["amount", kit.getU64Encoder()],
      ["fromSeed", kit.addEncoderSizePrefix(kit.getUtf8Encoder(), kit.getU64Encoder())],
      ["fromOwner", kit.getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: TRANSFER_SOL_WITH_SEED_DISCRIMINATOR
    })
  );
}
function getTransferSolWithSeedInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["amount", kit.getU64Decoder()],
    ["fromSeed", kit.addDecoderSizePrefix(kit.getUtf8Decoder(), kit.getU64Decoder())],
    ["fromOwner", kit.getAddressDecoder()]
  ]);
}
function getTransferSolWithSeedInstructionDataCodec() {
  return kit.combineCodec(
    getTransferSolWithSeedInstructionDataEncoder(),
    getTransferSolWithSeedInstructionDataDecoder()
  );
}
function getTransferSolWithSeedInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    source: { value: input.source ?? null, isWritable: true },
    baseAccount: { value: input.baseAccount ?? null, isWritable: false },
    destination: { value: input.destination ?? null, isWritable: true }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.source),
      getAccountMeta(accounts.baseAccount),
      getAccountMeta(accounts.destination)
    ],
    programAddress,
    data: getTransferSolWithSeedInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseTransferSolWithSeedInstruction(instruction) {
  if (instruction.accounts.length < 3) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      source: getNextAccount(),
      baseAccount: getNextAccount(),
      destination: getNextAccount()
    },
    data: getTransferSolWithSeedInstructionDataDecoder().decode(
      instruction.data
    )
  };
}
var UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR = 12;
function getUpgradeNonceAccountDiscriminatorBytes() {
  return kit.getU32Encoder().encode(UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getUpgradeNonceAccountInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([["discriminator", kit.getU32Encoder()]]),
    (value) => ({
      ...value,
      discriminator: UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getUpgradeNonceAccountInstructionDataDecoder() {
  return kit.getStructDecoder([["discriminator", kit.getU32Decoder()]]);
}
function getUpgradeNonceAccountInstructionDataCodec() {
  return kit.combineCodec(
    getUpgradeNonceAccountInstructionDataEncoder(),
    getUpgradeNonceAccountInstructionDataDecoder()
  );
}
function getUpgradeNonceAccountInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    nonceAccount: { value: input.nonceAccount ?? null, isWritable: true }
  };
  const accounts = originalAccounts;
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [getAccountMeta(accounts.nonceAccount)],
    programAddress,
    data: getUpgradeNonceAccountInstructionDataEncoder().encode({})
  };
  return instruction;
}
function parseUpgradeNonceAccountInstruction(instruction) {
  if (instruction.accounts.length < 1) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      nonceAccount: getNextAccount()
    },
    data: getUpgradeNonceAccountInstructionDataDecoder().decode(
      instruction.data
    )
  };
}
var WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR = 5;
function getWithdrawNonceAccountDiscriminatorBytes() {
  return kit.getU32Encoder().encode(WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getWithdrawNonceAccountInstructionDataEncoder() {
  return kit.transformEncoder(
    kit.getStructEncoder([
      ["discriminator", kit.getU32Encoder()],
      ["withdrawAmount", kit.getU64Encoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getWithdrawNonceAccountInstructionDataDecoder() {
  return kit.getStructDecoder([
    ["discriminator", kit.getU32Decoder()],
    ["withdrawAmount", kit.getU64Decoder()]
  ]);
}
function getWithdrawNonceAccountInstructionDataCodec() {
  return kit.combineCodec(
    getWithdrawNonceAccountInstructionDataEncoder(),
    getWithdrawNonceAccountInstructionDataDecoder()
  );
}
function getWithdrawNonceAccountInstruction(input, config) {
  const programAddress = config?.programAddress ?? SYSTEM_PROGRAM_ADDRESS;
  const originalAccounts = {
    nonceAccount: { value: input.nonceAccount ?? null, isWritable: true },
    recipientAccount: {
      value: input.recipientAccount ?? null,
      isWritable: true
    },
    recentBlockhashesSysvar: {
      value: input.recentBlockhashesSysvar ?? null,
      isWritable: false
    },
    rentSysvar: { value: input.rentSysvar ?? null, isWritable: false },
    nonceAuthority: { value: input.nonceAuthority ?? null, isWritable: false }
  };
  const accounts = originalAccounts;
  const args = { ...input };
  if (!accounts.recentBlockhashesSysvar.value) {
    accounts.recentBlockhashesSysvar.value = "SysvarRecentB1ockHashes11111111111111111111";
  }
  if (!accounts.rentSysvar.value) {
    accounts.rentSysvar.value = "SysvarRent111111111111111111111111111111111";
  }
  const getAccountMeta = getAccountMetaFactory(programAddress);
  const instruction = {
    accounts: [
      getAccountMeta(accounts.nonceAccount),
      getAccountMeta(accounts.recipientAccount),
      getAccountMeta(accounts.recentBlockhashesSysvar),
      getAccountMeta(accounts.rentSysvar),
      getAccountMeta(accounts.nonceAuthority)
    ],
    programAddress,
    data: getWithdrawNonceAccountInstructionDataEncoder().encode(
      args
    )
  };
  return instruction;
}
function parseWithdrawNonceAccountInstruction(instruction) {
  if (instruction.accounts.length < 5) {
    throw new Error("Not enough accounts");
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts[accountIndex];
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      nonceAccount: getNextAccount(),
      recipientAccount: getNextAccount(),
      recentBlockhashesSysvar: getNextAccount(),
      rentSysvar: getNextAccount(),
      nonceAuthority: getNextAccount()
    },
    data: getWithdrawNonceAccountInstructionDataDecoder().decode(
      instruction.data
    )
  };
}

exports.ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR = ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR;
exports.ALLOCATE_DISCRIMINATOR = ALLOCATE_DISCRIMINATOR;
exports.ALLOCATE_WITH_SEED_DISCRIMINATOR = ALLOCATE_WITH_SEED_DISCRIMINATOR;
exports.ASSIGN_DISCRIMINATOR = ASSIGN_DISCRIMINATOR;
exports.ASSIGN_WITH_SEED_DISCRIMINATOR = ASSIGN_WITH_SEED_DISCRIMINATOR;
exports.AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR = AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR;
exports.CREATE_ACCOUNT_DISCRIMINATOR = CREATE_ACCOUNT_DISCRIMINATOR;
exports.CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR = CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR;
exports.INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR = INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR;
exports.NonceState = NonceState;
exports.NonceVersion = NonceVersion;
exports.SYSTEM_ERROR__ACCOUNT_ALREADY_IN_USE = SYSTEM_ERROR__ACCOUNT_ALREADY_IN_USE;
exports.SYSTEM_ERROR__ADDRESS_WITH_SEED_MISMATCH = SYSTEM_ERROR__ADDRESS_WITH_SEED_MISMATCH;
exports.SYSTEM_ERROR__INVALID_ACCOUNT_DATA_LENGTH = SYSTEM_ERROR__INVALID_ACCOUNT_DATA_LENGTH;
exports.SYSTEM_ERROR__INVALID_PROGRAM_ID = SYSTEM_ERROR__INVALID_PROGRAM_ID;
exports.SYSTEM_ERROR__MAX_SEED_LENGTH_EXCEEDED = SYSTEM_ERROR__MAX_SEED_LENGTH_EXCEEDED;
exports.SYSTEM_ERROR__NONCE_BLOCKHASH_NOT_EXPIRED = SYSTEM_ERROR__NONCE_BLOCKHASH_NOT_EXPIRED;
exports.SYSTEM_ERROR__NONCE_NO_RECENT_BLOCKHASHES = SYSTEM_ERROR__NONCE_NO_RECENT_BLOCKHASHES;
exports.SYSTEM_ERROR__NONCE_UNEXPECTED_BLOCKHASH_VALUE = SYSTEM_ERROR__NONCE_UNEXPECTED_BLOCKHASH_VALUE;
exports.SYSTEM_ERROR__RESULT_WITH_NEGATIVE_LAMPORTS = SYSTEM_ERROR__RESULT_WITH_NEGATIVE_LAMPORTS;
exports.SYSTEM_PROGRAM_ADDRESS = SYSTEM_PROGRAM_ADDRESS;
exports.SystemAccount = SystemAccount;
exports.SystemInstruction = SystemInstruction;
exports.TRANSFER_SOL_DISCRIMINATOR = TRANSFER_SOL_DISCRIMINATOR;
exports.TRANSFER_SOL_WITH_SEED_DISCRIMINATOR = TRANSFER_SOL_WITH_SEED_DISCRIMINATOR;
exports.UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR = UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR;
exports.WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR = WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR;
exports.decodeNonce = decodeNonce;
exports.fetchAllMaybeNonce = fetchAllMaybeNonce;
exports.fetchAllNonce = fetchAllNonce;
exports.fetchMaybeNonce = fetchMaybeNonce;
exports.fetchNonce = fetchNonce;
exports.getAdvanceNonceAccountDiscriminatorBytes = getAdvanceNonceAccountDiscriminatorBytes;
exports.getAdvanceNonceAccountInstruction = getAdvanceNonceAccountInstruction;
exports.getAdvanceNonceAccountInstructionDataCodec = getAdvanceNonceAccountInstructionDataCodec;
exports.getAdvanceNonceAccountInstructionDataDecoder = getAdvanceNonceAccountInstructionDataDecoder;
exports.getAdvanceNonceAccountInstructionDataEncoder = getAdvanceNonceAccountInstructionDataEncoder;
exports.getAllocateDiscriminatorBytes = getAllocateDiscriminatorBytes;
exports.getAllocateInstruction = getAllocateInstruction;
exports.getAllocateInstructionDataCodec = getAllocateInstructionDataCodec;
exports.getAllocateInstructionDataDecoder = getAllocateInstructionDataDecoder;
exports.getAllocateInstructionDataEncoder = getAllocateInstructionDataEncoder;
exports.getAllocateWithSeedDiscriminatorBytes = getAllocateWithSeedDiscriminatorBytes;
exports.getAllocateWithSeedInstruction = getAllocateWithSeedInstruction;
exports.getAllocateWithSeedInstructionDataCodec = getAllocateWithSeedInstructionDataCodec;
exports.getAllocateWithSeedInstructionDataDecoder = getAllocateWithSeedInstructionDataDecoder;
exports.getAllocateWithSeedInstructionDataEncoder = getAllocateWithSeedInstructionDataEncoder;
exports.getAssignDiscriminatorBytes = getAssignDiscriminatorBytes;
exports.getAssignInstruction = getAssignInstruction;
exports.getAssignInstructionDataCodec = getAssignInstructionDataCodec;
exports.getAssignInstructionDataDecoder = getAssignInstructionDataDecoder;
exports.getAssignInstructionDataEncoder = getAssignInstructionDataEncoder;
exports.getAssignWithSeedDiscriminatorBytes = getAssignWithSeedDiscriminatorBytes;
exports.getAssignWithSeedInstruction = getAssignWithSeedInstruction;
exports.getAssignWithSeedInstructionDataCodec = getAssignWithSeedInstructionDataCodec;
exports.getAssignWithSeedInstructionDataDecoder = getAssignWithSeedInstructionDataDecoder;
exports.getAssignWithSeedInstructionDataEncoder = getAssignWithSeedInstructionDataEncoder;
exports.getAuthorizeNonceAccountDiscriminatorBytes = getAuthorizeNonceAccountDiscriminatorBytes;
exports.getAuthorizeNonceAccountInstruction = getAuthorizeNonceAccountInstruction;
exports.getAuthorizeNonceAccountInstructionDataCodec = getAuthorizeNonceAccountInstructionDataCodec;
exports.getAuthorizeNonceAccountInstructionDataDecoder = getAuthorizeNonceAccountInstructionDataDecoder;
exports.getAuthorizeNonceAccountInstructionDataEncoder = getAuthorizeNonceAccountInstructionDataEncoder;
exports.getCreateAccountDiscriminatorBytes = getCreateAccountDiscriminatorBytes;
exports.getCreateAccountInstruction = getCreateAccountInstruction;
exports.getCreateAccountInstructionDataCodec = getCreateAccountInstructionDataCodec;
exports.getCreateAccountInstructionDataDecoder = getCreateAccountInstructionDataDecoder;
exports.getCreateAccountInstructionDataEncoder = getCreateAccountInstructionDataEncoder;
exports.getCreateAccountWithSeedDiscriminatorBytes = getCreateAccountWithSeedDiscriminatorBytes;
exports.getCreateAccountWithSeedInstruction = getCreateAccountWithSeedInstruction;
exports.getCreateAccountWithSeedInstructionDataCodec = getCreateAccountWithSeedInstructionDataCodec;
exports.getCreateAccountWithSeedInstructionDataDecoder = getCreateAccountWithSeedInstructionDataDecoder;
exports.getCreateAccountWithSeedInstructionDataEncoder = getCreateAccountWithSeedInstructionDataEncoder;
exports.getInitializeNonceAccountDiscriminatorBytes = getInitializeNonceAccountDiscriminatorBytes;
exports.getInitializeNonceAccountInstruction = getInitializeNonceAccountInstruction;
exports.getInitializeNonceAccountInstructionDataCodec = getInitializeNonceAccountInstructionDataCodec;
exports.getInitializeNonceAccountInstructionDataDecoder = getInitializeNonceAccountInstructionDataDecoder;
exports.getInitializeNonceAccountInstructionDataEncoder = getInitializeNonceAccountInstructionDataEncoder;
exports.getNonceCodec = getNonceCodec;
exports.getNonceDecoder = getNonceDecoder;
exports.getNonceEncoder = getNonceEncoder;
exports.getNonceSize = getNonceSize;
exports.getNonceStateCodec = getNonceStateCodec;
exports.getNonceStateDecoder = getNonceStateDecoder;
exports.getNonceStateEncoder = getNonceStateEncoder;
exports.getNonceVersionCodec = getNonceVersionCodec;
exports.getNonceVersionDecoder = getNonceVersionDecoder;
exports.getNonceVersionEncoder = getNonceVersionEncoder;
exports.getSystemErrorMessage = getSystemErrorMessage;
exports.getTransferSolDiscriminatorBytes = getTransferSolDiscriminatorBytes;
exports.getTransferSolInstruction = getTransferSolInstruction;
exports.getTransferSolInstructionDataCodec = getTransferSolInstructionDataCodec;
exports.getTransferSolInstructionDataDecoder = getTransferSolInstructionDataDecoder;
exports.getTransferSolInstructionDataEncoder = getTransferSolInstructionDataEncoder;
exports.getTransferSolWithSeedDiscriminatorBytes = getTransferSolWithSeedDiscriminatorBytes;
exports.getTransferSolWithSeedInstruction = getTransferSolWithSeedInstruction;
exports.getTransferSolWithSeedInstructionDataCodec = getTransferSolWithSeedInstructionDataCodec;
exports.getTransferSolWithSeedInstructionDataDecoder = getTransferSolWithSeedInstructionDataDecoder;
exports.getTransferSolWithSeedInstructionDataEncoder = getTransferSolWithSeedInstructionDataEncoder;
exports.getUpgradeNonceAccountDiscriminatorBytes = getUpgradeNonceAccountDiscriminatorBytes;
exports.getUpgradeNonceAccountInstruction = getUpgradeNonceAccountInstruction;
exports.getUpgradeNonceAccountInstructionDataCodec = getUpgradeNonceAccountInstructionDataCodec;
exports.getUpgradeNonceAccountInstructionDataDecoder = getUpgradeNonceAccountInstructionDataDecoder;
exports.getUpgradeNonceAccountInstructionDataEncoder = getUpgradeNonceAccountInstructionDataEncoder;
exports.getWithdrawNonceAccountDiscriminatorBytes = getWithdrawNonceAccountDiscriminatorBytes;
exports.getWithdrawNonceAccountInstruction = getWithdrawNonceAccountInstruction;
exports.getWithdrawNonceAccountInstructionDataCodec = getWithdrawNonceAccountInstructionDataCodec;
exports.getWithdrawNonceAccountInstructionDataDecoder = getWithdrawNonceAccountInstructionDataDecoder;
exports.getWithdrawNonceAccountInstructionDataEncoder = getWithdrawNonceAccountInstructionDataEncoder;
exports.identifySystemInstruction = identifySystemInstruction;
exports.isSystemError = isSystemError;
exports.parseAdvanceNonceAccountInstruction = parseAdvanceNonceAccountInstruction;
exports.parseAllocateInstruction = parseAllocateInstruction;
exports.parseAllocateWithSeedInstruction = parseAllocateWithSeedInstruction;
exports.parseAssignInstruction = parseAssignInstruction;
exports.parseAssignWithSeedInstruction = parseAssignWithSeedInstruction;
exports.parseAuthorizeNonceAccountInstruction = parseAuthorizeNonceAccountInstruction;
exports.parseCreateAccountInstruction = parseCreateAccountInstruction;
exports.parseCreateAccountWithSeedInstruction = parseCreateAccountWithSeedInstruction;
exports.parseInitializeNonceAccountInstruction = parseInitializeNonceAccountInstruction;
exports.parseTransferSolInstruction = parseTransferSolInstruction;
exports.parseTransferSolWithSeedInstruction = parseTransferSolWithSeedInstruction;
exports.parseUpgradeNonceAccountInstruction = parseUpgradeNonceAccountInstruction;
exports.parseWithdrawNonceAccountInstruction = parseWithdrawNonceAccountInstruction;
//# sourceMappingURL=index.js.map
//# sourceMappingURL=index.js.map