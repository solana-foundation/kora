import { Codec, Decoder, Encoder, FixedSizeCodec, FixedSizeDecoder, FixedSizeEncoder, ReadonlyUint8Array, VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
import { FixedSizeNumberCodec, FixedSizeNumberDecoder, FixedSizeNumberEncoder, NumberCodec, NumberDecoder, NumberEncoder } from '@solana/codecs-numbers';
/**
 * Defines the configuration options for nullable codecs.
 *
 * This configuration controls how nullable values are encoded and decoded.
 *
 * By default, nullable values are prefixed with a `u8` (0 = `null`, 1 = present).
 * The `noneValue` and `prefix` options allow customizing this behavior.
 *
 * @typeParam TPrefix - A number codec, encoder, or decoder used as the presence prefix.
 *
 * @see {@link getNullableEncoder}
 * @see {@link getNullableDecoder}
 * @see {@link getNullableCodec}
 */
export type NullableCodecConfig<TPrefix extends NumberCodec | NumberDecoder | NumberEncoder> = {
    /**
     * Specifies how `null` values are represented in the encoded data.
     *
     * - By default, `null` values are omitted from encoding.
     * - `'zeroes'`: The bytes allocated for the value are filled with zeroes. This requires a fixed-size codec.
     * - Custom byte array: `null` values are replaced with a predefined byte sequence. This results in a variable-size codec.
     *
     * @defaultValue No explicit `noneValue` is used; `null` values are omitted.
     */
    noneValue?: ReadonlyUint8Array | 'zeroes';
    /**
     * The presence prefix used to distinguish between `null` and present values.
     *
     * - By default, a `u8` prefix is used (`0 = null`, `1 = present`).
     * - Custom number codec: Allows defining a different number size for the prefix.
     * - `null`: No prefix is used; `noneValue` (if provided) determines `null`.
     *   If no `noneValue` is set, `null` is identified by the absence of bytes.
     *
     * @defaultValue `u8` prefix.
     */
    prefix?: TPrefix | null;
};
/**
 * Returns an encoder for optional values, allowing `null` values to be encoded.
 *
 * This encoder serializes an optional value using a configurable approach:
 * - By default, a `u8` prefix is used (0 = `null`, 1 = present). This can be customized or disabled.
 * - If `noneValue: 'zeroes'` is set, `null` values are encoded as zeroes.
 * - If `noneValue` is a byte array, `null` values are replaced with the provided constant.
 *
 * For more details, see {@link getNullableCodec}.
 *
 * @typeParam TFrom - The type of the main value being encoded.
 *
 * @param item - The encoder for the value that may be present.
 * @param config - Configuration options for encoding optional values.
 * @returns A `FixedSizeEncoder` or `VariableSizeEncoder` for encoding nullable values.
 *
 * @example
 * Encoding an optional number.
 * ```ts
 * const encoder = getNullableEncoder(getU32Encoder());
 *
 * encoder.encode(null); // 0x00
 * encoder.encode(42);   // 0x012a000000
 * ```
 *
 * @see {@link getNullableCodec}
 */
export declare function getNullableEncoder<TFrom, TSize extends number>(item: FixedSizeEncoder<TFrom, TSize>, config: NullableCodecConfig<NumberEncoder> & {
    noneValue: 'zeroes';
    prefix: null;
}): FixedSizeEncoder<TFrom | null, TSize>;
export declare function getNullableEncoder<TFrom>(item: FixedSizeEncoder<TFrom>, config: NullableCodecConfig<FixedSizeNumberEncoder> & {
    noneValue: 'zeroes';
}): FixedSizeEncoder<TFrom | null>;
export declare function getNullableEncoder<TFrom>(item: FixedSizeEncoder<TFrom>, config: NullableCodecConfig<NumberEncoder> & {
    noneValue: 'zeroes';
}): VariableSizeEncoder<TFrom | null>;
export declare function getNullableEncoder<TFrom>(item: Encoder<TFrom>, config?: NullableCodecConfig<NumberEncoder> & {
    noneValue?: ReadonlyUint8Array;
}): VariableSizeEncoder<TFrom | null>;
/**
 * Returns a decoder for optional values, allowing `null` values to be recognized.
 *
 * This decoder deserializes an optional value using a configurable approach:
 * - By default, a `u8` prefix is used (0 = `null`, 1 = present). This can be customized or disabled.
 * - If `noneValue: 'zeroes'` is set, `null` values are identified by zeroes.
 * - If `noneValue` is a byte array, `null` values match the provided constant.
 *
 * For more details, see {@link getNullableCodec}.
 *
 * @typeParam TTo - The type of the main value being decoded.
 *
 * @param item - The decoder for the value that may be present.
 * @param config - Configuration options for decoding optional values.
 * @returns A `FixedSizeDecoder` or `VariableSizeDecoder` for decoding nullable values.
 *
 * @example
 * Decoding an optional number.
 * ```ts
 * const decoder = getNullableDecoder(getU32Decoder());
 *
 * decoder.decode(new Uint8Array([0x00])); // null
 * decoder.decode(new Uint8Array([0x01, 0x2a, 0x00, 0x00, 0x00])); // 42
 * ```
 *
 * @see {@link getNullableCodec}
 */
export declare function getNullableDecoder<TTo, TSize extends number>(item: FixedSizeDecoder<TTo, TSize>, config: NullableCodecConfig<NumberDecoder> & {
    noneValue: 'zeroes';
    prefix: null;
}): FixedSizeDecoder<TTo | null, TSize>;
export declare function getNullableDecoder<TTo>(item: FixedSizeDecoder<TTo>, config: NullableCodecConfig<FixedSizeNumberDecoder> & {
    noneValue: 'zeroes';
}): FixedSizeDecoder<TTo | null>;
export declare function getNullableDecoder<TTo>(item: FixedSizeDecoder<TTo>, config: NullableCodecConfig<NumberDecoder> & {
    noneValue: 'zeroes';
}): VariableSizeDecoder<TTo | null>;
export declare function getNullableDecoder<TTo>(item: Decoder<TTo>, config?: NullableCodecConfig<NumberDecoder> & {
    noneValue?: ReadonlyUint8Array;
}): VariableSizeDecoder<TTo | null>;
/**
 * Returns a codec for encoding and decoding optional values, allowing `null` values to be handled.
 *
 * This codec serializes and deserializes optional values using a configurable approach:
 * - By default, a `u8` prefix is used (0 = `null`, 1 = present).
 *    This can be customized using a custom number codec or even disabled by setting
 *    the `prefix` to `null`.
 * - If `noneValue: 'zeroes'` is set, `null` values are encoded/decoded as zeroes.
 * - If `noneValue` is a byte array, `null` values are represented by the provided constant.
 *
 * For more details on the configuration options, see {@link NullableCodecConfig}.
 *
 * @typeParam TFrom - The type of the main value being encoded.
 * @typeParam TTo - The type of the main value being decoded.
 *
 * @param item - The codec for the value that may be present.
 * @param config - Configuration options for encoding and decoding optional values.
 * @returns A `FixedSizeCodec` or `VariableSizeCodec` for encoding and decoding nullable values.
 *
 * @example
 * Encoding and decoding an optional number using a `u8` prefix (default).
 * ```ts
 * const codec = getNullableCodec(getU32Codec());
 *
 * codec.encode(null); // 0x00
 * codec.encode(42);   // 0x012a000000
 *
 * codec.decode(new Uint8Array([0x00])); // null
 * codec.decode(new Uint8Array([0x01, 0x2a, 0x00, 0x00, 0x00])); // 42
 * ```
 *
 * @example
 * Encoding and decoding an optional number using a fixed-size codec, by filling `null` values with zeroes.
 * ```ts
 * const codec = getNullableCodec(getU32Codec(), { noneValue: 'zeroes' });
 *
 * codec.encode(null); // 0x0000000000
 * codec.encode(42);   // 0x012a000000
 *
 * codec.decode(new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00])); // null
 * codec.decode(new Uint8Array([0x01, 0x2a, 0x00, 0x00, 0x00])); // 42
 * ```
 *
 * @example
 * Encoding and decoding `null` values with zeroes and no prefix.
 * ```ts
 * const codec = getNullableCodec(getU32Codec(), {
 *   noneValue: 'zeroes',
 *   prefix: null,
 * });
 *
 * codec.encode(null); // 0x00000000
 * codec.encode(42);   // 0x2a000000
 *
 * codec.decode(new Uint8Array([0x00, 0x00, 0x00, 0x00])); // null
 * codec.decode(new Uint8Array([0x2a, 0x00, 0x00, 0x00])); // 42
 * ```
 *
 * @example
 * Encoding and decoding `null` values with a custom byte sequence and no prefix.
 * ```ts
 * const codec = getNullableCodec(getU16Codec(), {
 *   noneValue: new Uint8Array([0xff, 0xff]),
 *   prefix: null,
 * });
 *
 * codec.encode(null); // 0xffff
 * codec.encode(42); // 0x2a00
 *
 * codec.decode(new Uint8Array([0xff, 0xff])); // null
 * codec.decode(new Uint8Array([0x2a, 0x00])); // 42
 * ```
 *
 * @example
 * Identifying `null` values by the absence of bytes.
 * ```ts
 * const codec = getNullableCodec(getU16Codec(), { prefix: null });
 *
 * codec.encode(null); // Empty bytes
 * codec.encode(42); // 0x2a00
 *
 * codec.decode(new Uint8Array([])); // null
 * codec.decode(new Uint8Array([0x2a, 0x00])); // 42
 * ```
 *
 * @remarks
 * Separate {@link getNullableEncoder} and {@link getNullableDecoder} functions are available.
 *
 * ```ts
 * const bytes = getNullableEncoder(getU32Encoder()).encode(42);
 * const value = getNullableDecoder(getU32Decoder()).decode(bytes);
 * ```
 *
 * @see {@link getNullableEncoder}
 * @see {@link getNullableDecoder}
 */
export declare function getNullableCodec<TFrom, TTo extends TFrom, TSize extends number>(item: FixedSizeCodec<TFrom, TTo, TSize>, config: NullableCodecConfig<NumberCodec> & {
    noneValue: 'zeroes';
    prefix: null;
}): FixedSizeCodec<TFrom | null, TTo | null, TSize>;
export declare function getNullableCodec<TFrom, TTo extends TFrom = TFrom>(item: FixedSizeCodec<TFrom, TTo>, config: NullableCodecConfig<FixedSizeNumberCodec> & {
    noneValue: 'zeroes';
}): FixedSizeCodec<TFrom | null, TTo | null>;
export declare function getNullableCodec<TFrom, TTo extends TFrom = TFrom>(item: FixedSizeCodec<TFrom, TTo>, config: NullableCodecConfig<NumberCodec> & {
    noneValue: 'zeroes';
}): VariableSizeCodec<TFrom | null, TTo | null>;
export declare function getNullableCodec<TFrom, TTo extends TFrom = TFrom>(item: Codec<TFrom, TTo>, config?: NullableCodecConfig<NumberCodec> & {
    noneValue?: ReadonlyUint8Array;
}): VariableSizeCodec<TFrom | null, TTo | null>;
//# sourceMappingURL=nullable.d.ts.map