import { decompileTransactionMessage, getBase64Codec, getTransactionCodec, getCompiledTransactionMessageCodec } from "@solana/kit";

function deserializedBase64Transaction(transaction: string) {
    const transactionBytes = getBase64Codec().encode(transaction);
    const originalTransaction = getTransactionCodec().decode(transactionBytes);
    const originalMessage = getCompiledTransactionMessageCodec().decode(originalTransaction.messageBytes);
    const decompiledMessage = decompileTransactionMessage(originalMessage);
    return decompiledMessage;
}

export { deserializedBase64Transaction };