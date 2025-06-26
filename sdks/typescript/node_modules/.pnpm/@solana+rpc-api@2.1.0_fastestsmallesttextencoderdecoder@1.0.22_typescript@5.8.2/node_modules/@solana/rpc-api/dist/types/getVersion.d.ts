type GetVersionApiResponse = Readonly<{
    /** Unique identifier of the current software's feature set */
    'feature-set': number;
    /** Software version of `solana-core` */
    'solana-core': string;
}>;
export type GetVersionApi = {
    /**
     * Returns the current Solana version running on the node
     */
    getVersion(NO_CONFIG?: Record<string, never>): GetVersionApiResponse;
};
export {};
//# sourceMappingURL=getVersion.d.ts.map