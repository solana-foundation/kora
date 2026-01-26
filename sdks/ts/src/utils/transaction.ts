import {
    decompileTransactionMessage,
    getBase64Codec,
    getCompiledTransactionMessageCodec,
    Instruction,
} from '@solana/kit';

/**
 * Deserializes a base64-encoded transaction message.
 * @param message - Base64-encoded transaction message
 * @returns Decompiled transaction message
 * @internal
 */
function deserializeBase64Message(message: string) {
    const messageBytes = getBase64Codec().encode(message);
    const originalMessage = getCompiledTransactionMessageCodec().decode(messageBytes);
    const decompiledMessage = decompileTransactionMessage(originalMessage);
    return decompiledMessage;
}

/**
 * Extracts instructions from a base64-encoded transaction message.
 * @param message - Base64-encoded transaction message
 * @returns Array of instructions from the transaction
 * @internal
 */
export function getInstructionsFromBase64Message(message: string): Instruction[] {
    if (!message || message === '') {
        return [];
    }

    try {
        const decompiledMessage = deserializeBase64Message(message);
        return decompiledMessage.instructions as Instruction[];
    } catch {
        // Silently handle parsing errors and return empty array
        return [];
    }
}
