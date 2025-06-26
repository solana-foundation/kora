import { OrderedAccounts } from '../compile/accounts';
type MessageHeader = Readonly<{
    numReadonlyNonSignerAccounts: number;
    numReadonlySignerAccounts: number;
    numSignerAccounts: number;
}>;
export declare function getCompiledMessageHeader(orderedAccounts: OrderedAccounts): MessageHeader;
export {};
//# sourceMappingURL=header.d.ts.map