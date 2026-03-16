import { type Address, type Instruction, type TransactionSigner } from '@solana/kit';
import {
    findAssociatedTokenPda,
    getTransferInstruction,
    parseTransferInstruction,
    TOKEN_PROGRAM_ADDRESS,
    TRANSFER_DISCRIMINATOR,
} from '@solana-program/token';

/** Plugin that adds a `paymentAddress` to the client. */
export function koraPaymentAddress(paymentAddress?: Address) {
    return <T extends object>(client: T) => ({
        ...client,
        paymentAddress,
    });
}

/**
 * Builds a placeholder payment instruction (amount=0) to reserve transaction space
 * during planning. The executor later replaces it with the real fee amount.
 */
export async function buildPlaceholderPaymentInstruction(
    feePayerWallet: TransactionSigner,
    paymentAddress: Address,
    feeToken: Address,
    tokenProgramId?: Address,
): Promise<{ destinationTokenAccount: Address; instruction: Instruction; sourceTokenAccount: Address }> {
    const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ADDRESS;

    const [sourceTokenAccount] = await findAssociatedTokenPda({
        mint: feeToken,
        owner: feePayerWallet.address,
        tokenProgram,
    });

    const [destinationTokenAccount] = await findAssociatedTokenPda({
        mint: feeToken,
        owner: paymentAddress,
        tokenProgram,
    });

    const instruction = getTransferInstruction(
        {
            amount: 0,
            authority: feePayerWallet,
            destination: destinationTokenAccount,
            source: sourceTokenAccount,
        },
        { programAddress: tokenProgram },
    );

    return { destinationTokenAccount, instruction, sourceTokenAccount };
}

function isPlaceholderPaymentInstruction(
    ix: Instruction,
    sourceTokenAccount: Address,
    destinationTokenAccount: Address,
    feePayerWallet: TransactionSigner,
    tokenProgramId?: Address,
): boolean {
    const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ADDRESS;
    if (ix.programAddress !== tokenProgram) return false;
    if (ix.data?.[0] !== TRANSFER_DISCRIMINATOR) return false;

    const parsed = parseTransferInstruction(
        ix as Instruction & { accounts: NonNullable<Instruction['accounts']>; data: NonNullable<Instruction['data']> },
    );
    return (
        parsed.accounts.source.address === sourceTokenAccount &&
        parsed.accounts.destination.address === destinationTokenAccount &&
        parsed.accounts.authority.address === feePayerWallet.address &&
        parsed.data.amount === 0n
    );
}

/** Replaces the placeholder (amount=0) with the estimated fee amount. */
export function updatePaymentInstructionAmount(
    instructions: readonly Instruction[],
    feePayerWallet: TransactionSigner,
    sourceTokenAccount: Address,
    destinationTokenAccount: Address,
    amount: bigint | number,
    tokenProgramId?: Address,
): Instruction[] {
    let replaced = false;
    const result = instructions.map(ix => {
        if (
            !isPlaceholderPaymentInstruction(
                ix,
                sourceTokenAccount,
                destinationTokenAccount,
                feePayerWallet,
                tokenProgramId,
            )
        ) {
            return ix;
        }
        replaced = true;
        const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ADDRESS;
        return getTransferInstruction(
            {
                amount,
                authority: feePayerWallet,
                destination: destinationTokenAccount,
                source: sourceTokenAccount,
            },
            { programAddress: tokenProgram },
        );
    });

    if (!replaced) {
        throw new Error(
            'Failed to update payment instruction: no matching placeholder transfer instruction found. ' +
                'This is a Kora SDK internal error — the transaction message may have been modified between planning and execution.',
        );
    }

    return result;
}

/** Removes the placeholder payment instruction (used when fee is 0). */
export function removePaymentInstruction(
    instructions: readonly Instruction[],
    sourceTokenAccount: Address,
    destinationTokenAccount: Address,
    feePayerWallet: TransactionSigner,
    tokenProgramId?: Address,
): Instruction[] {
    return instructions.filter(
        ix =>
            !isPlaceholderPaymentInstruction(
                ix,
                sourceTokenAccount,
                destinationTokenAccount,
                feePayerWallet,
                tokenProgramId,
            ),
    );
}
