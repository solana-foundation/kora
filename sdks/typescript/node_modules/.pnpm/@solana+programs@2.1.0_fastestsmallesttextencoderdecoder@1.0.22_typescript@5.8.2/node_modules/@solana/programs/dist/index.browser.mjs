import { isSolanaError, SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM } from '@solana/errors';

// src/program-error.ts
function isProgramError(error, transactionMessage, programAddress, code) {
  if (!isSolanaError(error, SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM)) {
    return false;
  }
  const instructionProgramAddress = transactionMessage.instructions[error.context.index]?.programAddress;
  if (!instructionProgramAddress || instructionProgramAddress !== programAddress) {
    return false;
  }
  return typeof code === "undefined" || error.context.code === code;
}

export { isProgramError };
//# sourceMappingURL=index.browser.mjs.map
//# sourceMappingURL=index.browser.mjs.map