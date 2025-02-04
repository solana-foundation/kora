pub mod rpc_config {
pub const RATE_LIMIT: i64 = 0;
}
pub mod validation_config {
#[derive(Debug)]
pub struct AllowedInstruction {
    pub program: &'static str,
    pub instructions: &'static [&'static str],
}

pub const ALLOWED_INSTRUCTIONS: &[AllowedInstruction] = &[
    AllowedInstruction {
        program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        instructions: &["[4]"],
    },
    AllowedInstruction {
        program: "11111111111111111111111111111111",
        instructions: &["transfer"],
    },
    AllowedInstruction {
        program: "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
        instructions: &["*"],
    },
];
pub const MAX_ALLOWED_LAMPORTS: i64 = 1000000;
pub const MAX_SIGNATURES: i64 = 10;
}
