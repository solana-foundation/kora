import { FixedSizeCodec, FixedSizeDecoder, FixedSizeEncoder, ReadonlyUint8Array } from '@solana/codecs-core';
/**
 * Returns an encoder that always writes a predefined constant byte sequence.
 *
 * This encoder ensures that encoding always produces the specified byte array,
 * ignoring any input values.
 *
 * For more details, see {@link getConstantCodec}.
 *
 * @typeParam TConstant - The fixed byte sequence that will be written during encoding.
 *
 * @param constant - The predefined byte array to encode.
 * @returns A `FixedSizeEncoder<void, N>` where `N` is the length of the constant.
 *
 * @example
 * Encoding a constant magic number.
 * ```ts
 * const encoder = getConstantEncoder(new Uint8Array([1, 2, 3, 4]));
 *
 * const bytes = encoder.encode();
 * // 0x01020304
 * //   └──────┘ The predefined 4-byte constant.
 * ```
 *
 * @see {@link getConstantCodec}
 */
export declare function getConstantEncoder<TConstant extends ReadonlyUint8Array>(constant: TConstant): FixedSizeEncoder<void, TConstant['length']>;
/**
 * Returns a decoder that verifies a predefined constant byte sequence.
 *
 * This decoder reads the next bytes and checks that they match the provided constant.
 * If the bytes differ, it throws an error.
 *
 * For more details, see {@link getConstantCodec}.
 *
 * @typeParam TConstant - The fixed byte sequence expected during decoding.
 *
 * @param constant - The predefined byte array to verify.
 * @returns A `FixedSizeDecoder<void, N>` where `N` is the length of the constant.
 *
 * @example
 * Decoding a constant magic number.
 * ```ts
 * const decoder = getConstantDecoder(new Uint8Array([1, 2, 3]));
 *
 * decoder.decode(new Uint8Array([1, 2, 3])); // Passes
 * decoder.decode(new Uint8Array([1, 2, 4])); // Throws an error
 * ```
 *
 * @see {@link getConstantCodec}
 */
export declare function getConstantDecoder<TConstant extends ReadonlyUint8Array>(constant: TConstant): FixedSizeDecoder<void, TConstant['length']>;
/**
 * Returns a codec that encodes and decodes a predefined constant byte sequence.
 *
 * - **Encoding:** Always writes the specified byte array.
 * - **Decoding:** Asserts that the next bytes match the constant, throwing an error if they do not.
 *
 * This is useful for encoding fixed byte patterns required in a binary format or to use in
 * conjunction with other codecs such as {@link getHiddenPrefixCodec} or {@link getHiddenSuffixCodec}.
 *
 * @typeParam TConstant - The fixed byte sequence to encode and verify during decoding.
 *
 * @param constant - The predefined byte array to encode and assert during decoding.
 * @returns A `FixedSizeCodec<void, void, N>` where `N` is the length of the constant.
 *
 * @example
 * Encoding and decoding a constant magic number.
 * ```ts
 * const codec = getConstantCodec(new Uint8Array([1, 2, 3]));
 *
 * codec.encode(); // 0x010203
 * codec.decode(new Uint8Array([1, 2, 3])); // Passes
 * codec.decode(new Uint8Array([1, 2, 4])); // Throws an error
 * ```
 *
 * @remarks
 * Separate {@link getConstantEncoder} and {@link getConstantDecoder} functions are available.
 *
 * ```ts
 * const bytes = getConstantEncoder(new Uint8Array([1, 2, 3])).encode();
 * getConstantDecoder(new Uint8Array([1, 2, 3])).decode(bytes);
 * ```
 *
 * @see {@link getConstantEncoder}
 * @see {@link getConstantDecoder}
 */
export declare function getConstantCodec<TConstant extends ReadonlyUint8Array>(constant: TConstant): FixedSizeCodec<void, void, TConstant['length']>;
//# sourceMappingURL=constant.d.ts.map