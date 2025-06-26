import { Address } from '@solana/addresses';
import { OrderedAccounts } from '../compile/accounts';
type AddressTableLookup = Readonly<{
    lookupTableAddress: Address;
    readableIndices: readonly number[];
    writableIndices: readonly number[];
}>;
export declare function getCompiledAddressTableLookups(orderedAccounts: OrderedAccounts): AddressTableLookup[];
export {};
//# sourceMappingURL=address-table-lookups.d.ts.map