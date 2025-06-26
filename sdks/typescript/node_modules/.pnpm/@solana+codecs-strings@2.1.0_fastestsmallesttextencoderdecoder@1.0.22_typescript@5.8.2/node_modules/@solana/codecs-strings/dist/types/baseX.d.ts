import { VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
/**
 * Returns an encoder for base-X encoded strings.
 *
 * This encoder serializes strings using a custom alphabet, treating the length of the alphabet as the base.
 * The encoding process involves converting the input string to a numeric value in base-X, then
 * encoding that value into bytes while preserving leading zeroes.
 *
 * For more details, see {@link getBaseXCodec}.
 *
 * @param alphabet - The set of characters defining the base-X encoding.
 * @returns A `VariableSizeEncoder<string>` for encoding base-X strings.
 *
 * @example
 * Encoding a base-X string using a custom alphabet.
 * ```ts
 * const encoder = getBaseXEncoder('0123456789abcdef');
 * const bytes = encoder.encode('deadface'); // 0xdeadface
 * ```
 *
 * @see {@link getBaseXCodec}
 */
export declare const getBaseXEncoder: (alphabet: string) => VariableSizeEncoder<string>;
/**
 * Returns a decoder for base-X encoded strings.
 *
 * This decoder deserializes base-X encoded strings from a byte array using a custom alphabet.
 * The decoding process converts the byte array into a numeric value in base-10, then
 * maps that value back to characters in the specified base-X alphabet.
 *
 * For more details, see {@link getBaseXCodec}.
 *
 * @param alphabet - The set of characters defining the base-X encoding.
 * @returns A `VariableSizeDecoder<string>` for decoding base-X strings.
 *
 * @example
 * Decoding a base-X string using a custom alphabet.
 * ```ts
 * const decoder = getBaseXDecoder('0123456789abcdef');
 * const value = decoder.decode(new Uint8Array([0xde, 0xad, 0xfa, 0xce])); // "deadface"
 * ```
 *
 * @see {@link getBaseXCodec}
 */
export declare const getBaseXDecoder: (alphabet: string) => VariableSizeDecoder<string>;
/**
 * Returns a codec for encoding and decoding base-X strings.
 *
 * This codec serializes strings using a custom alphabet, treating the length of the alphabet as the base.
 * The encoding process converts the input string into a numeric value in base-X, which is then encoded as bytes.
 * The decoding process reverses this transformation to reconstruct the original string.
 *
 * This codec supports leading zeroes by treating the first character of the alphabet as the zero character.
 *
 * @param alphabet - The set of characters defining the base-X encoding.
 * @returns A `VariableSizeCodec<string>` for encoding and decoding base-X strings.
 *
 * @example
 * Encoding and decoding a base-X string using a custom alphabet.
 * ```ts
 * const codec = getBaseXCodec('0123456789abcdef');
 * const bytes = codec.encode('deadface'); // 0xdeadface
 * const value = codec.decode(bytes);      // "deadface"
 * ```
 *
 * @remarks
 * This codec does not enforce a size boundary. It will encode and decode all bytes necessary to represent the string.
 *
 * If you need a fixed-size base-X codec, consider using {@link fixCodecSize}.
 *
 * ```ts
 * const codec = fixCodecSize(getBaseXCodec('0123456789abcdef'), 8);
 * ```
 *
 * If you need a size-prefixed base-X codec, consider using {@link addCodecSizePrefix}.
 *
 * ```ts
 * const codec = addCodecSizePrefix(getBaseXCodec('0123456789abcdef'), getU32Codec());
 * ```
 *
 * Separate {@link getBaseXEncoder} and {@link getBaseXDecoder} functions are available.
 *
 * ```ts
 * const bytes = getBaseXEncoder('0123456789abcdef').encode('deadface');
 * const value = getBaseXDecoder('0123456789abcdef').decode(bytes);
 * ```
 *
 * @see {@link getBaseXEncoder}
 * @see {@link getBaseXDecoder}
 */
export declare const getBaseXCodec: (alphabet: string) => VariableSizeCodec<string>;
//# sourceMappingURL=baseX.d.ts.map