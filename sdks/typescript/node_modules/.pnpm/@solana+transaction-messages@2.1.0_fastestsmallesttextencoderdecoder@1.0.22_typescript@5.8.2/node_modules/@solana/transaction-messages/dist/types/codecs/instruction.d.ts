import { VariableSizeCodec, VariableSizeDecoder, VariableSizeEncoder } from '@solana/codecs-core';
import { getCompiledInstructions } from '../compile/instructions';
type Instruction = ReturnType<typeof getCompiledInstructions>[number];
export declare function getInstructionEncoder(): VariableSizeEncoder<Instruction>;
export declare function getInstructionDecoder(): VariableSizeDecoder<Instruction>;
export declare function getInstructionCodec(): VariableSizeCodec<Instruction>;
export {};
//# sourceMappingURL=instruction.d.ts.map