import { decompileTransactionMessage, getBase64Codec, getTransactionCodec, getCompiledTransactionMessageCodec, Instruction } from "@solana/kit";

function deserializedBase64Message(message: string) {
    const messageBytes = getBase64Codec().encode(message);
    const originalMessage = getCompiledTransactionMessageCodec().decode(messageBytes);
    const decompiledMessage = decompileTransactionMessage(originalMessage);
    return decompiledMessage;
}

function getInstructionsFromBase64Message(message: string): Instruction[] {
    const decompiledMessage = deserializedBase64Message(message);
    return decompiledMessage.instructions as Instruction[];
}

export { getInstructionsFromBase64Message };
