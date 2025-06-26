import { Address } from './address';
/**
 * Given a public {@link CryptoKey}, this method will return its associated {@link Address}.
 *
 * @example
 * ```ts
 * import { getAddressFromPublicKey } from '@solana/addresses';
 *
 * const address = await getAddressFromPublicKey(publicKey);
 * ```
 */
export declare function getAddressFromPublicKey(publicKey: CryptoKey): Promise<Address>;
/**
 * Given an {@link Address}, return a {@link CryptoKey} that can be used to verify signatures.
 *
 * @example
 * ```ts
 * import { getAddressFromPublicKey } from '@solana/addresses';
 *
 * const publicKey = await getPublicKeyFromAddress(address);
 * ```
 */
export declare function getPublicKeyFromAddress(address: Address): Promise<CryptoKey>;
//# sourceMappingURL=public-key.d.ts.map