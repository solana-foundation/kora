import { VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
/**
 * Returns an encoder for base-64 strings.
 *
 * This encoder serializes strings using a base-64 encoding scheme,
 * commonly used for data encoding in URLs, cryptographic keys, and binary-to-text encoding.
 *
 * For more details, see {@link getBase64Codec}.
 *
 * @returns A `VariableSizeEncoder<string>` for encoding base-64 strings.
 *
 * @example
 * Encoding a base-64 string.
 * ```ts
 * const encoder = getBase64Encoder();
 * const bytes = encoder.encode('hello+world'); // 0x85e965a3ec28ae57
 * ```
 *
 * @see {@link getBase64Codec}
 */
export declare const getBase64Encoder: () => VariableSizeEncoder<string>;
/**
 * Returns a decoder for base-64 strings.
 *
 * This decoder deserializes base-64 encoded strings from a byte array.
 *
 * For more details, see {@link getBase64Codec}.
 *
 * @returns A `VariableSizeDecoder<string>` for decoding base-64 strings.
 *
 * @example
 * Decoding a base-64 string.
 * ```ts
 * const decoder = getBase64Decoder();
 * const value = decoder.decode(new Uint8Array([0x85, 0xe9, 0x65, 0xa3, 0xec, 0x28, 0xae, 0x57])); // "hello+world"
 * ```
 *
 * @see {@link getBase64Codec}
 */
export declare const getBase64Decoder: () => VariableSizeDecoder<string>;
/**
 * Returns a codec for encoding and decoding base-64 strings.
 *
 * This codec serializes strings using a base-64 encoding scheme,
 * commonly used for data encoding in URLs, cryptographic keys, and binary-to-text encoding.
 *
 * @returns A `VariableSizeCodec<string>` for encoding and decoding base-64 strings.
 *
 * @example
 * Encoding and decoding a base-64 string.
 * ```ts
 * const codec = getBase64Codec();
 * const bytes = codec.encode('hello+world'); // 0x85e965a3ec28ae57
 * const value = codec.decode(bytes);         // "hello+world"
 * ```
 *
 * @remarks
 * This codec does not enforce a size boundary. It will encode and decode all bytes necessary to represent the string.
 *
 * If you need a fixed-size base-64 codec, consider using {@link fixCodecSize}.
 *
 * ```ts
 * const codec = fixCodecSize(getBase64Codec(), 8);
 * ```
 *
 * If you need a size-prefixed base-64 codec, consider using {@link addCodecSizePrefix}.
 *
 * ```ts
 * const codec = addCodecSizePrefix(getBase64Codec(), getU32Codec());
 * ```
 *
 * Separate {@link getBase64Encoder} and {@link getBase64Decoder} functions are available.
 *
 * ```ts
 * const bytes = getBase64Encoder().encode('hello+world');
 * const value = getBase64Decoder().decode(bytes);
 * ```
 *
 * @see {@link getBase64Encoder}
 * @see {@link getBase64Decoder}
 */
export declare const getBase64Codec: () => VariableSizeCodec<string>;
//# sourceMappingURL=base64.d.ts.map