import { FixedSizeCodec, FixedSizeDecoder, FixedSizeEncoder } from '@solana/codecs-core';
import { getCompiledMessageHeader } from '../compile/header';
type MessageHeader = ReturnType<typeof getCompiledMessageHeader>;
export declare function getMessageHeaderEncoder(): FixedSizeEncoder<MessageHeader, 3>;
export declare function getMessageHeaderDecoder(): FixedSizeDecoder<MessageHeader, 3>;
export declare function getMessageHeaderCodec(): FixedSizeCodec<MessageHeader, MessageHeader, 3>;
export {};
//# sourceMappingURL=header.d.ts.map