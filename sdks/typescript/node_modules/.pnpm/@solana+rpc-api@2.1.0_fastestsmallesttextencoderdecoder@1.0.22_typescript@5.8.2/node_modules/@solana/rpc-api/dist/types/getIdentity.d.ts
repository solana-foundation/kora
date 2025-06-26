import type { Address } from '@solana/addresses';
type GetIdentityApiResponse = Readonly<{
    identity: Address;
}>;
export type GetIdentityApi = {
    /**
     * Returns the identity pubkey for the current node
     */
    getIdentity(NO_CONFIG?: Record<string, never>): GetIdentityApiResponse;
};
export {};
//# sourceMappingURL=getIdentity.d.ts.map