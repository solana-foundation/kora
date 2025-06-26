import { ReadonlyUint8Array } from '@solana/codecs-core';
export declare function generateKeyPair(): Promise<CryptoKeyPair>;
export declare function createKeyPairFromBytes(bytes: ReadonlyUint8Array, extractable?: boolean): Promise<CryptoKeyPair>;
export declare function createKeyPairFromPrivateKeyBytes(bytes: ReadonlyUint8Array, extractable?: boolean): Promise<CryptoKeyPair>;
//# sourceMappingURL=key-pair.d.ts.map