export type StringifiedNumber = string & {
    readonly __brand: unique symbol;
};
export declare function isStringifiedNumber(putativeNumber: string): putativeNumber is StringifiedNumber;
export declare function assertIsStringifiedNumber(putativeNumber: string): asserts putativeNumber is StringifiedNumber;
export declare function stringifiedNumber(putativeNumber: string): StringifiedNumber;
//# sourceMappingURL=stringified-number.d.ts.map