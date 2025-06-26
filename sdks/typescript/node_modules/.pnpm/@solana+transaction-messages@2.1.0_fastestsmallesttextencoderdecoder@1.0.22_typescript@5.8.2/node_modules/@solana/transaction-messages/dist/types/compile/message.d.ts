import { CompilableTransactionMessage } from '../compilable-transaction-message';
import { getCompiledAddressTableLookups } from './address-table-lookups';
import { getCompiledMessageHeader } from './header';
import { getCompiledInstructions } from './instructions';
import { getCompiledLifetimeToken } from './lifetime-token';
import { getCompiledStaticAccounts } from './static-accounts';
type BaseCompiledTransactionMessage = Readonly<{
    header: ReturnType<typeof getCompiledMessageHeader>;
    instructions: ReturnType<typeof getCompiledInstructions>;
    lifetimeToken: ReturnType<typeof getCompiledLifetimeToken>;
    staticAccounts: ReturnType<typeof getCompiledStaticAccounts>;
}>;
export type CompiledTransactionMessage = LegacyCompiledTransactionMessage | VersionedCompiledTransactionMessage;
type LegacyCompiledTransactionMessage = BaseCompiledTransactionMessage & Readonly<{
    version: 'legacy';
}>;
type VersionedCompiledTransactionMessage = BaseCompiledTransactionMessage & Readonly<{
    addressTableLookups?: ReturnType<typeof getCompiledAddressTableLookups>;
    version: number;
}>;
export declare function compileTransactionMessage(transaction: CompilableTransactionMessage & Readonly<{
    version: 'legacy';
}>): LegacyCompiledTransactionMessage;
export declare function compileTransactionMessage(transaction: CompilableTransactionMessage): VersionedCompiledTransactionMessage;
export {};
//# sourceMappingURL=message.d.ts.map