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
pub const ALLOWED_PROGRAMS: &[&str] = &["11111111111111111111111111111111", "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"];
pub const ALLOWED_SPL_PAID_TOKENS: &[&str] = &["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"];
pub const ALLOWED_TOKENS: &[&str] = &["4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"];
pub const DISALLOWED_ACCOUNTS: &[&str] = &[];
pub const MAX_ALLOWED_LAMPORTS: u64 = 1000000;
pub const MAX_SIGNATURES: u32 = 10;
