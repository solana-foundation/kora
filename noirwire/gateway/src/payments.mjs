import { createTransferCheckedInstruction, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";

export const USDC_DECIMALS = 6;

/** The wallet's USDC associated token account. Program-owned wallets are allowed. */
export const usdcAccountOf = (mint, wallet) => getAssociatedTokenAddressSync(mint, wallet, true, TOKEN_PROGRAM_ID);

/**
 * The transfers appended to the user's transaction, all from the user's USDC account:
 * B + S (or B + M when anonymous) to NoirWire, and C to the app when C is above zero.
 * Because the quote pins the message hash, these are exactly what the user signs.
 */
export function buildPaymentInstructions({ user, mint, noirwireAccount, appAccount, split }) {
  const source = usdcAccountOf(mint, user);
  const transfer = (destination, amount) =>
    createTransferCheckedInstruction(source, mint, destination, user, amount, USDC_DECIMALS, [], TOKEN_PROGRAM_ID);
  const instructions = [];
  const toNoirwire = split.networkCost + split.noirwire;
  if (toNoirwire > 0n) instructions.push(transfer(noirwireAccount, toNoirwire));
  if (split.app > 0n && appAccount) instructions.push(transfer(appAccount, split.app));
  return instructions;
}
