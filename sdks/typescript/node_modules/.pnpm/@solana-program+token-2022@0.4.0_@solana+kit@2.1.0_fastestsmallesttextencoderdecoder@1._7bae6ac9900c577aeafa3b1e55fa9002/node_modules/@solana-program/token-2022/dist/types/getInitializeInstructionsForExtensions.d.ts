import { Address, IInstruction, TransactionSigner } from '@solana/kit';
import { ExtensionArgs } from './generated';
/**
 * Given a mint address and a list of mint extensions, returns a list of
 * instructions that MUST be run _before_ the `initializeMint` instruction
 * to properly initialize the given extensions on the mint account.
 */
export declare function getPreInitializeInstructionsForMintExtensions(mint: Address, extensions: ExtensionArgs[]): IInstruction[];
/**
 * Given a mint address and a list of mint extensions, returns a list of
 * instructions that MUST be run _after_ the `initializeMint` instruction
 * to properly initialize the given extensions on the mint account.
 */
export declare function getPostInitializeInstructionsForMintExtensions(mint: Address, authority: TransactionSigner, extensions: ExtensionArgs[]): IInstruction[];
/**
 * Given a token address, its owner and a list of token extensions, returns a list
 * of instructions that MUST be run _after_ the `initializeAccount` instruction
 * to properly initialize the given extensions on the token account.
 */
export declare function getPostInitializeInstructionsForTokenExtensions(token: Address, owner: TransactionSigner | Address, extensions: ExtensionArgs[], multiSigners?: TransactionSigner[]): IInstruction[];
//# sourceMappingURL=getInitializeInstructionsForExtensions.d.ts.map