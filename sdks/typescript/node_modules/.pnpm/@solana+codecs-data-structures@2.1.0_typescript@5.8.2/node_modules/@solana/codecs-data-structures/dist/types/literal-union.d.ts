import { FixedSizeCodec, FixedSizeDecoder, FixedSizeEncoder, VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
import { FixedSizeNumberCodec, FixedSizeNumberDecoder, FixedSizeNumberEncoder, NumberCodec, NumberDecoder, NumberEncoder } from '@solana/codecs-numbers';
/**
 * Defines the configuration options for literal union codecs.
 *
 * A literal union codec encodes values from a predefined set of literals.
 * The `size` option determines the numerical encoding used for the discriminant.
 * By default, literals are stored as a `u8` (1 byte).
 *
 * @typeParam TDiscriminator - A number codec, encoder, or decoder used for the discriminant.
 */
export type LiteralUnionCodecConfig<TDiscriminator = NumberCodec | NumberDecoder | NumberEncoder> = {
    /**
     * The codec used to encode/decode the discriminator.
     * @defaultValue `u8` discriminator.
     */
    size?: TDiscriminator;
};
type Variant = bigint | boolean | number | string | null | undefined;
type GetTypeFromVariants<TVariants extends readonly Variant[]> = TVariants[number];
/**
 * Returns an encoder for literal unions.
 *
 * This encoder serializes a value from a predefined set of literals
 * as a numerical index representing its position in the `variants` array.
 *
 * For more details, see {@link getLiteralUnionCodec}.
 *
 * @typeParam TVariants - A tuple of allowed literal values.
 *
 * @param variants - The possible literal values for the union.
 * @param config - Configuration options for encoding the literal union.
 * @returns A `FixedSizeEncoder` or `VariableSizeEncoder` for encoding literal unions.
 *
 * @example
 * Encoding a union of string literals.
 * ```ts
 * type Size = 'small' | 'medium' | 'large';
 * const sizeEncoder = getLiteralUnionEncoder(['small', 'medium', 'large']);
 *
 * sizeEncoder.encode('small');  // 0x00
 * sizeEncoder.encode('medium'); // 0x01
 * sizeEncoder.encode('large');  // 0x02
 * ```
 *
 * @see {@link getLiteralUnionCodec}
 */
export declare function getLiteralUnionEncoder<const TVariants extends readonly Variant[]>(variants: TVariants): FixedSizeEncoder<GetTypeFromVariants<TVariants>, 1>;
export declare function getLiteralUnionEncoder<const TVariants extends readonly Variant[], TSize extends number>(variants: TVariants, config: LiteralUnionCodecConfig<NumberEncoder> & {
    size: FixedSizeNumberEncoder<TSize>;
}): FixedSizeEncoder<GetTypeFromVariants<TVariants>, TSize>;
export declare function getLiteralUnionEncoder<const TVariants extends readonly Variant[]>(variants: TVariants, config?: LiteralUnionCodecConfig<NumberEncoder>): VariableSizeEncoder<GetTypeFromVariants<TVariants>>;
/**
 * Returns a decoder for literal unions.
 *
 * This decoder deserializes a numerical index into a corresponding
 * value from a predefined set of literals.
 *
 * For more details, see {@link getLiteralUnionCodec}.
 *
 * @typeParam TVariants - A tuple of allowed literal values.
 *
 * @param variants - The possible literal values for the union.
 * @param config - Configuration options for decoding the literal union.
 * @returns A `FixedSizeDecoder` or `VariableSizeDecoder` for decoding literal unions.
 *
 * @example
 * Decoding a union of string literals.
 * ```ts
 * type Size = 'small' | 'medium' | 'large';
 * const sizeDecoder = getLiteralUnionDecoder(['small', 'medium', 'large']);
 *
 * sizeDecoder.decode(new Uint8Array([0x00])); // 'small'
 * sizeDecoder.decode(new Uint8Array([0x01])); // 'medium'
 * sizeDecoder.decode(new Uint8Array([0x02])); // 'large'
 * ```
 *
 * @see {@link getLiteralUnionCodec}
 */
export declare function getLiteralUnionDecoder<const TVariants extends readonly Variant[]>(variants: TVariants): FixedSizeDecoder<GetTypeFromVariants<TVariants>, 1>;
export declare function getLiteralUnionDecoder<const TVariants extends readonly Variant[], TSize extends number>(variants: TVariants, config: LiteralUnionCodecConfig<NumberDecoder> & {
    size: FixedSizeNumberDecoder<TSize>;
}): FixedSizeDecoder<GetTypeFromVariants<TVariants>, TSize>;
export declare function getLiteralUnionDecoder<const TVariants extends readonly Variant[]>(variants: TVariants, config?: LiteralUnionCodecConfig<NumberDecoder>): VariableSizeDecoder<GetTypeFromVariants<TVariants>>;
/**
 * Returns a codec for encoding and decoding literal unions.
 *
 * A literal union codec serializes and deserializes values
 * from a predefined set of literals, using a numerical index
 * to represent each value in the `variants` array.
 *
 * This allows efficient storage and retrieval of common
 * predefined values such as enum-like structures in TypeScript.
 *
 * @typeParam TVariants - A tuple of allowed literal values.
 *
 * @param variants - The possible literal values for the union.
 * @param config - Configuration options for encoding and decoding the literal union.
 * @returns A `FixedSizeCodec` or `VariableSizeCodec` for encoding and decoding literal unions.
 *
 * @example
 * Encoding and decoding a union of string literals.
 * ```ts
 * type Size = 'small' | 'medium' | 'large';
 * const sizeCodec = getLiteralUnionCodec(['small', 'medium', 'large']);
 *
 * sizeCodec.encode('small');  // 0x00
 * sizeCodec.encode('medium'); // 0x01
 * sizeCodec.encode('large');  // 0x02
 *
 * sizeCodec.decode(new Uint8Array([0x00])); // 'small'
 * sizeCodec.decode(new Uint8Array([0x01])); // 'medium'
 * sizeCodec.decode(new Uint8Array([0x02])); // 'large'
 * ```
 *
 * @example
 * Encoding and decoding a union of number literals.
 * ```ts
 * type Level = 10 | 20 | 30;
 * const levelCodec = getLiteralUnionCodec([10, 20, 30]);
 *
 * levelCodec.encode(10);  // 0x00
 * levelCodec.encode(20);  // 0x01
 * levelCodec.encode(30);  // 0x02
 *
 * levelCodec.decode(new Uint8Array([0x00])); // 10
 * levelCodec.decode(new Uint8Array([0x01])); // 20
 * levelCodec.decode(new Uint8Array([0x02])); // 30
 * ```
 *
 * @example
 * Using a custom discriminator size with different variant types.
 * ```ts
 * type MaybeBoolean = false | true | "either";
 * const codec = getLiteralUnionCodec([false, true, 'either'], { size: getU16Codec() });
 *
 * codec.encode(false);    // 0x0000
 * codec.encode(true);     // 0x0100
 * codec.encode('either'); // 0x0200
 *
 * codec.decode(new Uint8Array([0x00, 0x00])); // false
 * codec.decode(new Uint8Array([0x01, 0x00])); // true
 * codec.decode(new Uint8Array([0x02, 0x00])); // 'either'
 * ```
 *
 * @remarks
 * Separate {@link getLiteralUnionEncoder} and {@link getLiteralUnionDecoder} functions are available.
 *
 * ```ts
 * const bytes = getLiteralUnionEncoder(['red', 'green', 'blue']).encode('green');
 * const value = getLiteralUnionDecoder(['red', 'green', 'blue']).decode(bytes);
 * ```
 *
 * @see {@link getLiteralUnionEncoder}
 * @see {@link getLiteralUnionDecoder}
 */
export declare function getLiteralUnionCodec<const TVariants extends readonly Variant[]>(variants: TVariants): FixedSizeCodec<GetTypeFromVariants<TVariants>, GetTypeFromVariants<TVariants>, 1>;
export declare function getLiteralUnionCodec<const TVariants extends readonly Variant[], TSize extends number>(variants: TVariants, config: LiteralUnionCodecConfig<NumberCodec> & {
    size: FixedSizeNumberCodec<TSize>;
}): FixedSizeCodec<GetTypeFromVariants<TVariants>, GetTypeFromVariants<TVariants>, TSize>;
export declare function getLiteralUnionCodec<const TVariants extends readonly Variant[]>(variants: TVariants, config?: LiteralUnionCodecConfig<NumberCodec>): VariableSizeCodec<GetTypeFromVariants<TVariants>>;
export {};
//# sourceMappingURL=literal-union.d.ts.map