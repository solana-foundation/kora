import { Address } from '@solana/addresses';
import { MessageModifyingSigner } from './message-modifying-signer';
import { MessagePartialSigner } from './message-partial-signer';
/**
 * Defines a signer capable of signing messages.
 *
 * @see {@link MessageModifyingSigner} For signers that can modify messages before signing them.
 * @see {@link MessagePartialSigner} For signers that can be used in parallel.
 * @see {@link isMessageSigner}
 * @see {@link assertIsMessageSigner}
 */
export type MessageSigner<TAddress extends string = string> = MessageModifyingSigner<TAddress> | MessagePartialSigner<TAddress>;
/**
 * Checks whether the provided value implements the {@link MessageSigner} interface.
 *
 * @typeParam TAddress - The inferred type of the address provided.
 *
 * @example
 * ```ts
 * import { Address } from '@solana/addresses';
 * import { isMessageSigner } from '@solana/signers';
 *
 * const address = '1234..5678' as Address<'1234..5678'>;
 * isMessageSigner({ address, signMessages: async () => {} }); // true
 * isMessageSigner({ address, modifyAndSignMessages: async () => {} }); // true
 * isMessageSigner({ address }); // false
 * ```
 *
 * @see {@link assertIsMessageSigner}
 */
export declare function isMessageSigner<TAddress extends string>(value: {
    [key: string]: unknown;
    address: Address<TAddress>;
}): value is MessageSigner<TAddress>;
/**
 * Asserts that the provided value implements the {@link MessageSigner} interface.
 *
 * @typeParam TAddress - The inferred type of the address provided.
 *
 * @example
 * ```ts
 * import { Address } from '@solana/addresses';
 * import { assertIsMessageSigner } from '@solana/signers';
 *
 * const address = '1234..5678' as Address<'1234..5678'>;
 * assertIsMessageSigner({ address, signMessages: async () => {} }); // void
 * assertIsMessageSigner({ address, modifyAndSignMessages: async () => {} }); // void
 * assertIsMessageSigner({ address }); // Throws an error.
 * ```
 *
 * @see {@link isMessageSigner}
 */
export declare function assertIsMessageSigner<TAddress extends string>(value: {
    [key: string]: unknown;
    address: Address<TAddress>;
}): asserts value is MessageSigner<TAddress>;
//# sourceMappingURL=message-signer.d.ts.map