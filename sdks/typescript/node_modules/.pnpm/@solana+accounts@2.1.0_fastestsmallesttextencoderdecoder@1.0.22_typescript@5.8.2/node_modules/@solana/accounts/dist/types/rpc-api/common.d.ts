export type JsonParsedDataResponse<TData = object> = Readonly<{
    parsed: {
        info?: TData;
        type: string;
    };
    program: string;
    space: bigint;
}>;
//# sourceMappingURL=common.d.ts.map