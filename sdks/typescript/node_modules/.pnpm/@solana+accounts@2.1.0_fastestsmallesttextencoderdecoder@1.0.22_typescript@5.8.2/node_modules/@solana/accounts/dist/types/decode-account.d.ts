import type { Decoder, ReadonlyUint8Array } from '@solana/codecs-core';
import type { Account, EncodedAccount } from './account';
import type { MaybeAccount, MaybeEncodedAccount } from './maybe-account';
/**
 * Transforms an {@link EncodedAccount} into an {@link Account} (or a {@link MaybeEncodedAccount}
 * into a {@link MaybeAccount}) by decoding the account data using the provided {@link Decoder}
 * instance.
 *
 * @typeParam TAddress - Supply a string literal to define an account having a particular address.
 * @typeParam TData - The type of this account's data.
 *
 * @example
 * ```ts
 * type MyAccountData = { name: string; age: number };
 *
 * const myAccount: EncodedAccount<'1234..5678'>;
 * const myDecoder: Decoder<MyAccountData> = getStructDecoder([
 *     ['name', addDecoderSizePrefix(getUtf8Decoder(), getU32Decoder())],
 *     ['age', getU32Decoder()],
 * ]);
 *
 * const myDecodedAccount = decodeAccount(myAccount, myDecoder);
 * myDecodedAccount satisfies Account<MyAccountData, '1234..5678'>;
 * ```
 */
export declare function decodeAccount<TData extends object, TAddress extends string = string>(encodedAccount: EncodedAccount<TAddress>, decoder: Decoder<TData>): Account<TData, TAddress>;
export declare function decodeAccount<TData extends object, TAddress extends string = string>(encodedAccount: MaybeEncodedAccount<TAddress>, decoder: Decoder<TData>): MaybeAccount<TData, TAddress>;
/**
 * Asserts that an account stores decoded data, ie. not a `Uint8Array`.
 *
 * Note that it does not check the shape of the data matches the decoded type, only that it is not a
 * `Uint8Array`.
 *
 * @typeParam TAddress - Supply a string literal to define an account having a particular address.
 * @typeParam TData - The type of this account's data.
 *
 * @example
 * ```ts
 * type MyAccountData = { name: string; age: number };
 *
 * const myAccount: Account<MyAccountData | Uint8Array, '1234..5678'>;
 * assertAccountDecoded(myAccount);
 *
 * // now the account data can be used as MyAccountData
 * account.data satisfies MyAccountData;
 * ```
 *
 * This is particularly useful for narrowing the result of fetching a JSON parsed account.
 *
 * ```ts
 * const account: MaybeAccount<MockData | Uint8Array> = await fetchJsonParsedAccount<MockData>(
 *     rpc,
 *     '1234..5678' as Address,
 * );
 *
 * assertAccountDecoded(account);
 * // now we have a MaybeAccount<MockData>
 * account satisfies MaybeAccount<MockData>;
 * ```
 */
export declare function assertAccountDecoded<TData extends object, TAddress extends string = string>(account: Account<TData | Uint8Array, TAddress>): asserts account is Account<TData, TAddress>;
export declare function assertAccountDecoded<TData extends object, TAddress extends string = string>(account: MaybeAccount<TData | Uint8Array, TAddress>): asserts account is MaybeAccount<TData, TAddress>;
/**
 * Asserts that all input accounts store decoded data, ie. not a `Uint8Array`.
 *
 * As with {@link assertAccountDecoded} it does not check the shape of the data matches the decoded
 * type, only that it is not a `Uint8Array`.
 *
 * @example
 * ```ts
 * type MyAccountData = { name: string; age: number };
 *
 * const myAccounts: Account<MyAccountData | Uint8Array, Address>[];
 * assertAccountsDecoded(myAccounts);
 *
 * // now the account data can be used as MyAccountData
 * for (const a of account) {
 *     account.data satisfies MyAccountData;
 * }
 * ```
 */
export declare function assertAccountsDecoded<TData extends object, TAddress extends string = string>(accounts: Account<ReadonlyUint8Array | TData, TAddress>[]): asserts accounts is Account<TData, TAddress>[];
export declare function assertAccountsDecoded<TData extends object, TAddress extends string = string>(accounts: MaybeAccount<ReadonlyUint8Array | TData, TAddress>[]): asserts accounts is MaybeAccount<TData, TAddress>[];
//# sourceMappingURL=decode-account.d.ts.map