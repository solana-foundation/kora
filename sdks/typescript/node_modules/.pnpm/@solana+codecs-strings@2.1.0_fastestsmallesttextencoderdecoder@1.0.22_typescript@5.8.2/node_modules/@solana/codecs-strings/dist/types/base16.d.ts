import { VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
/**
 * Returns an encoder for base-16 (hexadecimal) strings.
 *
 * This encoder serializes strings using a base-16 encoding scheme.
 * The output consists of bytes representing the hexadecimal values of the input string.
 *
 * For more details, see {@link getBase16Codec}.
 *
 * @returns A `VariableSizeEncoder<string>` for encoding base-16 strings.
 *
 * @example
 * Encoding a base-16 string.
 * ```ts
 * const encoder = getBase16Encoder();
 * const bytes = encoder.encode('deadface'); // 0xdeadface
 * ```
 *
 * @see {@link getBase16Codec}
 */
export declare const getBase16Encoder: () => VariableSizeEncoder<string>;
/**
 * Returns a decoder for base-16 (hexadecimal) strings.
 *
 * This decoder deserializes base-16 encoded strings from a byte array.
 *
 * For more details, see {@link getBase16Codec}.
 *
 * @returns A `VariableSizeDecoder<string>` for decoding base-16 strings.
 *
 * @example
 * Decoding a base-16 string.
 * ```ts
 * const decoder = getBase16Decoder();
 * const value = decoder.decode(new Uint8Array([0xde, 0xad, 0xfa, 0xce])); // "deadface"
 * ```
 *
 * @see {@link getBase16Codec}
 */
export declare const getBase16Decoder: () => VariableSizeDecoder<string>;
/**
 * Returns a codec for encoding and decoding base-16 (hexadecimal) strings.
 *
 * This codec serializes strings using a base-16 encoding scheme.
 * The output consists of bytes representing the hexadecimal values of the input string.
 *
 * @returns A `VariableSizeCodec<string>` for encoding and decoding base-16 strings.
 *
 * @example
 * Encoding and decoding a base-16 string.
 * ```ts
 * const codec = getBase16Codec();
 * const bytes = codec.encode('deadface'); // 0xdeadface
 * const value = codec.decode(bytes);      // "deadface"
 * ```
 *
 * @remarks
 * This codec does not enforce a size boundary. It will encode and decode all bytes necessary to represent the string.
 *
 * If you need a fixed-size base-16 codec, consider using {@link fixCodecSize}.
 *
 * ```ts
 * const codec = fixCodecSize(getBase16Codec(), 8);
 * ```
 *
 * If you need a size-prefixed base-16 codec, consider using {@link addCodecSizePrefix}.
 *
 * ```ts
 * const codec = addCodecSizePrefix(getBase16Codec(), getU32Codec());
 * ```
 *
 * Separate {@link getBase16Encoder} and {@link getBase16Decoder} functions are available.
 *
 * ```ts
 * const bytes = getBase16Encoder().encode('deadface');
 * const value = getBase16Decoder().decode(bytes);
 * ```
 *
 * @see {@link getBase16Encoder}
 * @see {@link getBase16Decoder}
 */
export declare const getBase16Codec: () => VariableSizeCodec<string>;
//# sourceMappingURL=base16.d.ts.map