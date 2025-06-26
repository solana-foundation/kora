import { FixedSizeCodec, FixedSizeDecoder, FixedSizeEncoder, VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
import { FixedSizeNumberCodec, FixedSizeNumberDecoder, FixedSizeNumberEncoder, NumberCodec, NumberDecoder, NumberEncoder } from '@solana/codecs-numbers';
/**
 * Defines the configuration options for boolean codecs.
 *
 * A boolean codec encodes `true` as `1` and `false` as `0`.
 * The `size` option allows customizing the number codec used for storage.
 *
 * @typeParam TSize - A number codec, encoder, or decoder used for boolean representation.
 *
 * @see {@link getBooleanEncoder}
 * @see {@link getBooleanDecoder}
 * @see {@link getBooleanCodec}
 */
export type BooleanCodecConfig<TSize extends NumberCodec | NumberDecoder | NumberEncoder> = {
    /**
     * The number codec used to store boolean values.
     *
     * - By default, booleans are stored as a `u8` (`1` for `true`, `0` for `false`).
     * - A custom number codec can be provided to change the storage size.
     *
     * @defaultValue `u8`
     */
    size?: TSize;
};
/**
 * Returns an encoder for boolean values.
 *
 * This encoder converts `true` into `1` and `false` into `0`.
 * The `size` option allows customizing the number codec used for storage.
 *
 * For more details, see {@link getBooleanCodec}.
 *
 * @param config - Configuration options for encoding booleans.
 * @returns A `FixedSizeEncoder<boolean, N>` where `N` is the size of the number codec.
 *
 * @example
 * Encoding booleans.
 * ```ts
 * const encoder = getBooleanEncoder();
 *
 * encoder.encode(false); // 0x00
 * encoder.encode(true);  // 0x01
 * ```
 *
 * @see {@link getBooleanCodec}
 */
export declare function getBooleanEncoder(): FixedSizeEncoder<boolean, 1>;
export declare function getBooleanEncoder<TSize extends number>(config: BooleanCodecConfig<NumberEncoder> & {
    size: FixedSizeNumberEncoder<TSize>;
}): FixedSizeEncoder<boolean, TSize>;
export declare function getBooleanEncoder(config: BooleanCodecConfig<NumberEncoder>): VariableSizeEncoder<boolean>;
/**
 * Returns a decoder for boolean values.
 *
 * This decoder reads a number and interprets `1` as `true` and `0` as `false`.
 * The `size` option allows customizing the number codec used for storage.
 *
 * For more details, see {@link getBooleanCodec}.
 *
 * @param config - Configuration options for decoding booleans.
 * @returns A `FixedSizeDecoder<boolean, N>` where `N` is the size of the number codec.
 *
 * @example
 * Decoding booleans.
 * ```ts
 * const decoder = getBooleanDecoder();
 *
 * decoder.decode(new Uint8Array([0x00])); // false
 * decoder.decode(new Uint8Array([0x01])); // true
 * ```
 *
 * @see {@link getBooleanCodec}
 */
export declare function getBooleanDecoder(): FixedSizeDecoder<boolean, 1>;
export declare function getBooleanDecoder<TSize extends number>(config: BooleanCodecConfig<NumberDecoder> & {
    size: FixedSizeNumberDecoder<TSize>;
}): FixedSizeDecoder<boolean, TSize>;
export declare function getBooleanDecoder(config: BooleanCodecConfig<NumberDecoder>): VariableSizeDecoder<boolean>;
/**
 * Returns a codec for encoding and decoding boolean values.
 *
 * By default, booleans are stored as a `u8` (`1` for `true`, `0` for `false`).
 * The `size` option allows customizing the number codec used for storage.
 *
 * @param config - Configuration options for encoding and decoding booleans.
 * @returns A `FixedSizeCodec<boolean, boolean, N>` where `N` is the size of the number codec.
 *
 * @example
 * Encoding and decoding booleans using a `u8` (default).
 * ```ts
 * const codec = getBooleanCodec();
 *
 * codec.encode(false); // 0x00
 * codec.encode(true);  // 0x01
 *
 * codec.decode(new Uint8Array([0x00])); // false
 * codec.decode(new Uint8Array([0x01])); // true
 * ```
 *
 * @example
 * Encoding and decoding booleans using a custom number codec.
 * ```ts
 * const codec = getBooleanCodec({ size: getU16Codec() });
 *
 * codec.encode(false); // 0x0000
 * codec.encode(true);  // 0x0100
 *
 * codec.decode(new Uint8Array([0x00, 0x00])); // false
 * codec.decode(new Uint8Array([0x01, 0x00])); // true
 * ```
 *
 * @remarks
 * Separate {@link getBooleanEncoder} and {@link getBooleanDecoder} functions are available.
 *
 * ```ts
 * const bytes = getBooleanEncoder().encode(true);
 * const value = getBooleanDecoder().decode(bytes);
 * ```
 *
 * @see {@link getBooleanEncoder}
 * @see {@link getBooleanDecoder}
 */
export declare function getBooleanCodec(): FixedSizeCodec<boolean, boolean, 1>;
export declare function getBooleanCodec<TSize extends number>(config: BooleanCodecConfig<NumberCodec> & {
    size: FixedSizeNumberCodec<TSize>;
}): FixedSizeCodec<boolean, boolean, TSize>;
export declare function getBooleanCodec(config: BooleanCodecConfig<NumberCodec>): VariableSizeCodec<boolean>;
//# sourceMappingURL=boolean.d.ts.map