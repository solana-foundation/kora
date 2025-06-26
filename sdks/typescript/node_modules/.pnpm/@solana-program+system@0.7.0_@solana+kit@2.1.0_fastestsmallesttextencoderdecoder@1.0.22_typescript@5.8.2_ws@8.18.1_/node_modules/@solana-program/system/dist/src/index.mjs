import { getEnumEncoder, getU32Encoder, getEnumDecoder, getU32Decoder, combineCodec, getStructEncoder, getAddressEncoder, getU64Encoder, getStructDecoder, getAddressDecoder, getU64Decoder, decodeAccount, assertAccountExists, fetchEncodedAccount, assertAccountsExist, fetchEncodedAccounts, containsBytes, isProgramError, transformEncoder, addEncoderSizePrefix, getUtf8Encoder, addDecoderSizePrefix, getUtf8Decoder, BASE_ACCOUNT_SIZE, AccountRole, upgradeRoleToSigner, isTransactionSigner as isTransactionSigner$1 } from '@solana/kit';

// src/generated/accounts/nonce.ts
var NonceState = /* @__PURE__ */ ((NonceState2) => {
  NonceState2[NonceState2["Uninitialized"] = 0] = "Uninitialized";
  NonceState2[NonceState2["Initialized"] = 1] = "Initialized";
  return NonceState2;
})(NonceState || {});
function getNonceStateEncoder() {
  return getEnumEncoder(NonceState, { size: getU32Encoder() });
}
function getNonceStateDecoder() {
  return getEnumDecoder(NonceState, { size: getU32Decoder() });
}
function getNonceStateCodec() {
  return combineCodec(getNonceStateEncoder(), getNonceStateDecoder());
}
var NonceVersion = /* @__PURE__ */ ((NonceVersion2) => {
  NonceVersion2[NonceVersion2["Legacy"] = 0] = "Legacy";
  NonceVersion2[NonceVersion2["Current"] = 1] = "Current";
  return NonceVersion2;
})(NonceVersion || {});
function getNonceVersionEncoder() {
  return getEnumEncoder(NonceVersion, { size: getU32Encoder() });
}
function getNonceVersionDecoder() {
  return getEnumDecoder(NonceVersion, { size: getU32Decoder() });
}
function getNonceVersionCodec() {
  return combineCodec(getNonceVersionEncoder(), getNonceVersionDecoder());
}

// src/generated/accounts/nonce.ts
function getNonceEncoder() {
  return getStructEncoder([
    ["version", getNonceVersionEncoder()],
    ["state", getNonceStateEncoder()],
    ["authority", getAddressEncoder()],
    ["blockhash", getAddressEncoder()],
    ["lamportsPerSignature", getU64Encoder()]
  ]);
}
function getNonceDecoder() {
  return getStructDecoder([
    ["version", getNonceVersionDecoder()],
    ["state", getNonceStateDecoder()],
    ["authority", getAddressDecoder()],
    ["blockhash", getAddressDecoder()],
    ["lamportsPerSignature", getU64Decoder()]
  ]);
}
function getNonceCodec() {
  return combineCodec(getNonceEncoder(), getNonceDecoder());
}
function decodeNonce(encodedAccount) {
  return decodeAccount(
    encodedAccount,
    getNonceDecoder()
  );
}
async function fetchNonce(rpc, address, config) {
  const maybeAccount = await fetchMaybeNonce(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}
async function fetchMaybeNonce(rpc, address, config) {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeNonce(maybeAccount);
}
async function fetchAllNonce(rpc, addresses, config) {
  const maybeAccounts = await fetchAllMaybeNonce(rpc, addresses, config);
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}
async function fetchAllMaybeNonce(rpc, addresses, config) {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
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
  if (containsBytes(data, getU32Encoder().encode(0), 0)) {
    return 0 /* CreateAccount */;
  }
  if (containsBytes(data, getU32Encoder().encode(1), 0)) {
    return 1 /* Assign */;
  }
  if (containsBytes(data, getU32Encoder().encode(2), 0)) {
    return 2 /* TransferSol */;
  }
  if (containsBytes(data, getU32Encoder().encode(3), 0)) {
    return 3 /* CreateAccountWithSeed */;
  }
  if (containsBytes(data, getU32Encoder().encode(4), 0)) {
    return 4 /* AdvanceNonceAccount */;
  }
  if (containsBytes(data, getU32Encoder().encode(5), 0)) {
    return 5 /* WithdrawNonceAccount */;
  }
  if (containsBytes(data, getU32Encoder().encode(6), 0)) {
    return 6 /* InitializeNonceAccount */;
  }
  if (containsBytes(data, getU32Encoder().encode(7), 0)) {
    return 7 /* AuthorizeNonceAccount */;
  }
  if (containsBytes(data, getU32Encoder().encode(8), 0)) {
    return 8 /* Allocate */;
  }
  if (containsBytes(data, getU32Encoder().encode(9), 0)) {
    return 9 /* AllocateWithSeed */;
  }
  if (containsBytes(data, getU32Encoder().encode(10), 0)) {
    return 10 /* AssignWithSeed */;
  }
  if (containsBytes(data, getU32Encoder().encode(11), 0)) {
    return 11 /* TransferSolWithSeed */;
  }
  if (containsBytes(data, getU32Encoder().encode(12), 0)) {
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
  return isProgramError(
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
        role: AccountRole.READONLY
      });
    }
    const writableRole = account.isWritable ? AccountRole.WRITABLE : AccountRole.READONLY;
    return Object.freeze({
      address: expectAddress(account.value),
      role: isTransactionSigner(account.value) ? upgradeRoleToSigner(writableRole) : writableRole,
      ...isTransactionSigner(account.value) ? { signer: account.value } : {}
    });
  };
}
function isTransactionSigner(value) {
  return !!value && typeof value === "object" && "address" in value && isTransactionSigner$1(value);
}

// src/generated/instructions/advanceNonceAccount.ts
var ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR = 4;
function getAdvanceNonceAccountDiscriminatorBytes() {
  return getU32Encoder().encode(ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getAdvanceNonceAccountInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([["discriminator", getU32Encoder()]]),
    (value) => ({
      ...value,
      discriminator: ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getAdvanceNonceAccountInstructionDataDecoder() {
  return getStructDecoder([["discriminator", getU32Decoder()]]);
}
function getAdvanceNonceAccountInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(ALLOCATE_DISCRIMINATOR);
}
function getAllocateInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["space", getU64Encoder()]
    ]),
    (value) => ({ ...value, discriminator: ALLOCATE_DISCRIMINATOR })
  );
}
function getAllocateInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["space", getU64Decoder()]
  ]);
}
function getAllocateInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(ALLOCATE_WITH_SEED_DISCRIMINATOR);
}
function getAllocateWithSeedInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["base", getAddressEncoder()],
      ["seed", addEncoderSizePrefix(getUtf8Encoder(), getU64Encoder())],
      ["space", getU64Encoder()],
      ["programAddress", getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: ALLOCATE_WITH_SEED_DISCRIMINATOR })
  );
}
function getAllocateWithSeedInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["base", getAddressDecoder()],
    ["seed", addDecoderSizePrefix(getUtf8Decoder(), getU64Decoder())],
    ["space", getU64Decoder()],
    ["programAddress", getAddressDecoder()]
  ]);
}
function getAllocateWithSeedInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(ASSIGN_DISCRIMINATOR);
}
function getAssignInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["programAddress", getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: ASSIGN_DISCRIMINATOR })
  );
}
function getAssignInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["programAddress", getAddressDecoder()]
  ]);
}
function getAssignInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(ASSIGN_WITH_SEED_DISCRIMINATOR);
}
function getAssignWithSeedInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["base", getAddressEncoder()],
      ["seed", addEncoderSizePrefix(getUtf8Encoder(), getU64Encoder())],
      ["programAddress", getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: ASSIGN_WITH_SEED_DISCRIMINATOR })
  );
}
function getAssignWithSeedInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["base", getAddressDecoder()],
    ["seed", addDecoderSizePrefix(getUtf8Decoder(), getU64Decoder())],
    ["programAddress", getAddressDecoder()]
  ]);
}
function getAssignWithSeedInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getAuthorizeNonceAccountInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["newNonceAuthority", getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getAuthorizeNonceAccountInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["newNonceAuthority", getAddressDecoder()]
  ]);
}
function getAuthorizeNonceAccountInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(CREATE_ACCOUNT_DISCRIMINATOR);
}
function getCreateAccountInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["lamports", getU64Encoder()],
      ["space", getU64Encoder()],
      ["programAddress", getAddressEncoder()]
    ]),
    (value) => ({ ...value, discriminator: CREATE_ACCOUNT_DISCRIMINATOR })
  );
}
function getCreateAccountInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["lamports", getU64Decoder()],
    ["space", getU64Decoder()],
    ["programAddress", getAddressDecoder()]
  ]);
}
function getCreateAccountInstructionDataCodec() {
  return combineCodec(
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
  const byteDelta = [Number(args.space) + BASE_ACCOUNT_SIZE].reduce(
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
  return getU32Encoder().encode(CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR);
}
function getCreateAccountWithSeedInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["base", getAddressEncoder()],
      ["seed", addEncoderSizePrefix(getUtf8Encoder(), getU64Encoder())],
      ["amount", getU64Encoder()],
      ["space", getU64Encoder()],
      ["programAddress", getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR
    })
  );
}
function getCreateAccountWithSeedInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["base", getAddressDecoder()],
    ["seed", addDecoderSizePrefix(getUtf8Decoder(), getU64Decoder())],
    ["amount", getU64Decoder()],
    ["space", getU64Decoder()],
    ["programAddress", getAddressDecoder()]
  ]);
}
function getCreateAccountWithSeedInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getInitializeNonceAccountInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["nonceAuthority", getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getInitializeNonceAccountInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["nonceAuthority", getAddressDecoder()]
  ]);
}
function getInitializeNonceAccountInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(TRANSFER_SOL_DISCRIMINATOR);
}
function getTransferSolInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["amount", getU64Encoder()]
    ]),
    (value) => ({ ...value, discriminator: TRANSFER_SOL_DISCRIMINATOR })
  );
}
function getTransferSolInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["amount", getU64Decoder()]
  ]);
}
function getTransferSolInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(TRANSFER_SOL_WITH_SEED_DISCRIMINATOR);
}
function getTransferSolWithSeedInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["amount", getU64Encoder()],
      ["fromSeed", addEncoderSizePrefix(getUtf8Encoder(), getU64Encoder())],
      ["fromOwner", getAddressEncoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: TRANSFER_SOL_WITH_SEED_DISCRIMINATOR
    })
  );
}
function getTransferSolWithSeedInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["amount", getU64Decoder()],
    ["fromSeed", addDecoderSizePrefix(getUtf8Decoder(), getU64Decoder())],
    ["fromOwner", getAddressDecoder()]
  ]);
}
function getTransferSolWithSeedInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getUpgradeNonceAccountInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([["discriminator", getU32Encoder()]]),
    (value) => ({
      ...value,
      discriminator: UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getUpgradeNonceAccountInstructionDataDecoder() {
  return getStructDecoder([["discriminator", getU32Decoder()]]);
}
function getUpgradeNonceAccountInstructionDataCodec() {
  return combineCodec(
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
  return getU32Encoder().encode(WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR);
}
function getWithdrawNonceAccountInstructionDataEncoder() {
  return transformEncoder(
    getStructEncoder([
      ["discriminator", getU32Encoder()],
      ["withdrawAmount", getU64Encoder()]
    ]),
    (value) => ({
      ...value,
      discriminator: WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR
    })
  );
}
function getWithdrawNonceAccountInstructionDataDecoder() {
  return getStructDecoder([
    ["discriminator", getU32Decoder()],
    ["withdrawAmount", getU64Decoder()]
  ]);
}
function getWithdrawNonceAccountInstructionDataCodec() {
  return combineCodec(
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

export { ADVANCE_NONCE_ACCOUNT_DISCRIMINATOR, ALLOCATE_DISCRIMINATOR, ALLOCATE_WITH_SEED_DISCRIMINATOR, ASSIGN_DISCRIMINATOR, ASSIGN_WITH_SEED_DISCRIMINATOR, AUTHORIZE_NONCE_ACCOUNT_DISCRIMINATOR, CREATE_ACCOUNT_DISCRIMINATOR, CREATE_ACCOUNT_WITH_SEED_DISCRIMINATOR, INITIALIZE_NONCE_ACCOUNT_DISCRIMINATOR, NonceState, NonceVersion, SYSTEM_ERROR__ACCOUNT_ALREADY_IN_USE, SYSTEM_ERROR__ADDRESS_WITH_SEED_MISMATCH, SYSTEM_ERROR__INVALID_ACCOUNT_DATA_LENGTH, SYSTEM_ERROR__INVALID_PROGRAM_ID, SYSTEM_ERROR__MAX_SEED_LENGTH_EXCEEDED, SYSTEM_ERROR__NONCE_BLOCKHASH_NOT_EXPIRED, SYSTEM_ERROR__NONCE_NO_RECENT_BLOCKHASHES, SYSTEM_ERROR__NONCE_UNEXPECTED_BLOCKHASH_VALUE, SYSTEM_ERROR__RESULT_WITH_NEGATIVE_LAMPORTS, SYSTEM_PROGRAM_ADDRESS, SystemAccount, SystemInstruction, TRANSFER_SOL_DISCRIMINATOR, TRANSFER_SOL_WITH_SEED_DISCRIMINATOR, UPGRADE_NONCE_ACCOUNT_DISCRIMINATOR, WITHDRAW_NONCE_ACCOUNT_DISCRIMINATOR, decodeNonce, fetchAllMaybeNonce, fetchAllNonce, fetchMaybeNonce, fetchNonce, getAdvanceNonceAccountDiscriminatorBytes, getAdvanceNonceAccountInstruction, getAdvanceNonceAccountInstructionDataCodec, getAdvanceNonceAccountInstructionDataDecoder, getAdvanceNonceAccountInstructionDataEncoder, getAllocateDiscriminatorBytes, getAllocateInstruction, getAllocateInstructionDataCodec, getAllocateInstructionDataDecoder, getAllocateInstructionDataEncoder, getAllocateWithSeedDiscriminatorBytes, getAllocateWithSeedInstruction, getAllocateWithSeedInstructionDataCodec, getAllocateWithSeedInstructionDataDecoder, getAllocateWithSeedInstructionDataEncoder, getAssignDiscriminatorBytes, getAssignInstruction, getAssignInstructionDataCodec, getAssignInstructionDataDecoder, getAssignInstructionDataEncoder, getAssignWithSeedDiscriminatorBytes, getAssignWithSeedInstruction, getAssignWithSeedInstructionDataCodec, getAssignWithSeedInstructionDataDecoder, getAssignWithSeedInstructionDataEncoder, getAuthorizeNonceAccountDiscriminatorBytes, getAuthorizeNonceAccountInstruction, getAuthorizeNonceAccountInstructionDataCodec, getAuthorizeNonceAccountInstructionDataDecoder, getAuthorizeNonceAccountInstructionDataEncoder, getCreateAccountDiscriminatorBytes, getCreateAccountInstruction, getCreateAccountInstructionDataCodec, getCreateAccountInstructionDataDecoder, getCreateAccountInstructionDataEncoder, getCreateAccountWithSeedDiscriminatorBytes, getCreateAccountWithSeedInstruction, getCreateAccountWithSeedInstructionDataCodec, getCreateAccountWithSeedInstructionDataDecoder, getCreateAccountWithSeedInstructionDataEncoder, getInitializeNonceAccountDiscriminatorBytes, getInitializeNonceAccountInstruction, getInitializeNonceAccountInstructionDataCodec, getInitializeNonceAccountInstructionDataDecoder, getInitializeNonceAccountInstructionDataEncoder, getNonceCodec, getNonceDecoder, getNonceEncoder, getNonceSize, getNonceStateCodec, getNonceStateDecoder, getNonceStateEncoder, getNonceVersionCodec, getNonceVersionDecoder, getNonceVersionEncoder, getSystemErrorMessage, getTransferSolDiscriminatorBytes, getTransferSolInstruction, getTransferSolInstructionDataCodec, getTransferSolInstructionDataDecoder, getTransferSolInstructionDataEncoder, getTransferSolWithSeedDiscriminatorBytes, getTransferSolWithSeedInstruction, getTransferSolWithSeedInstructionDataCodec, getTransferSolWithSeedInstructionDataDecoder, getTransferSolWithSeedInstructionDataEncoder, getUpgradeNonceAccountDiscriminatorBytes, getUpgradeNonceAccountInstruction, getUpgradeNonceAccountInstructionDataCodec, getUpgradeNonceAccountInstructionDataDecoder, getUpgradeNonceAccountInstructionDataEncoder, getWithdrawNonceAccountDiscriminatorBytes, getWithdrawNonceAccountInstruction, getWithdrawNonceAccountInstructionDataCodec, getWithdrawNonceAccountInstructionDataDecoder, getWithdrawNonceAccountInstructionDataEncoder, identifySystemInstruction, isSystemError, parseAdvanceNonceAccountInstruction, parseAllocateInstruction, parseAllocateWithSeedInstruction, parseAssignInstruction, parseAssignWithSeedInstruction, parseAuthorizeNonceAccountInstruction, parseCreateAccountInstruction, parseCreateAccountWithSeedInstruction, parseInitializeNonceAccountInstruction, parseTransferSolInstruction, parseTransferSolWithSeedInstruction, parseUpgradeNonceAccountInstruction, parseWithdrawNonceAccountInstruction };
//# sourceMappingURL=index.mjs.map
//# sourceMappingURL=index.mjs.map