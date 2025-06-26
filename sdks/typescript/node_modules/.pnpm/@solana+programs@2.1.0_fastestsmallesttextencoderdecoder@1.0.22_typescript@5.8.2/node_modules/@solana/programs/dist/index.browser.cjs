'use strict';

var errors = require('@solana/errors');

// src/program-error.ts
function isProgramError(error, transactionMessage, programAddress, code) {
  if (!errors.isSolanaError(error, errors.SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM)) {
    return false;
  }
  const instructionProgramAddress = transactionMessage.instructions[error.context.index]?.programAddress;
  if (!instructionProgramAddress || instructionProgramAddress !== programAddress) {
    return false;
  }
  return typeof code === "undefined" || error.context.code === code;
}

exports.isProgramError = isProgramError;
//# sourceMappingURL=index.browser.cjs.map
//# sourceMappingURL=index.browser.cjs.map