import { type VariableSizeCodec, type VariableSizeDecoder, type VariableSizeEncoder } from '@solana/codecs-core';
import type { getCompiledAddressTableLookups } from '../compile/address-table-lookups';
type AddressTableLookup = ReturnType<typeof getCompiledAddressTableLookups>[number];
export declare function getAddressTableLookupEncoder(): VariableSizeEncoder<AddressTableLookup>;
export declare function getAddressTableLookupDecoder(): VariableSizeDecoder<AddressTableLookup>;
export declare function getAddressTableLookupCodec(): VariableSizeCodec<AddressTableLookup>;
export {};
//# sourceMappingURL=address-table-lookup.d.ts.map