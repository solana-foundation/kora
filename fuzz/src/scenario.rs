use arbitrary::Arbitrary;
use kora_lib::{
    config::FeePayerPolicy,
    constant::{BPF_LOADER_UPGRADEABLE_PROGRAM_ID, LOADER_V4_PROGRAM_ID},
    Config,
};
use serde_json::{json, Value};
use solana_message::{v0, v1, Message, VersionedMessage};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};

pub const FEE_PAYER: Pubkey = Pubkey::new_from_array([1; 32]);

const POOL: [Pubkey; 4] = [
    FEE_PAYER,
    Pubkey::new_from_array([2; 32]),
    Pubkey::new_from_array([3; 32]),
    Pubkey::new_from_array([4; 32]),
];
const MAX_OPS: usize = 6;
const MAX_BATCH_OPS: usize = 3;
const MAX_SIGNERS: usize = 3;
const MAX_BYTES: usize = 32;
const MAX_RAW_ACCOUNTS: usize = 8;
const CREATE_ACCOUNT_ALLOW_PREFUND_TAG: u32 = 13;

#[derive(Arbitrary, Debug, Clone, Copy)]
pub struct Key(u8);

impl Key {
    pub fn pubkey(self) -> Pubkey {
        POOL[usize::from(self.0) % POOL.len()]
    }

    pub fn is_fee_payer(self) -> bool {
        self.pubkey() == FEE_PAYER
    }
}

#[derive(Arbitrary, Debug, Clone)]
pub struct Signers(Vec<Key>);

impl Signers {
    pub fn keys(&self) -> &[Key] {
        &self.0[..self.0.len().min(MAX_SIGNERS)]
    }

    pub fn contains_fee_payer(&self) -> bool {
        self.keys().iter().any(|key| key.is_fee_payer())
    }

    fn pubkeys(&self) -> Vec<Pubkey> {
        self.keys().iter().map(|key| key.pubkey()).collect()
    }
}

#[derive(Arbitrary, Debug, Clone, Copy)]
pub struct Seed(u8);

impl Seed {
    fn as_string(self) -> String {
        format!("seed{}", self.0 % 4)
    }
}

#[derive(Arbitrary, Debug, Clone)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    fn to_vec(&self) -> Vec<u8> {
        self.0[..self.0.len().min(MAX_BYTES)].to_vec()
    }
}

#[derive(Arbitrary, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProgram {
    Spl,
    Token2022,
}

impl TokenProgram {
    pub fn id(self) -> Pubkey {
        match self {
            Self::Spl => spl_token_interface::ID,
            Self::Token2022 => spl_token_2022_interface::ID,
        }
    }
}

#[derive(Arbitrary, Debug, Clone, Copy)]
pub enum MessageVersion {
    Legacy,
    V0,
    V1,
}

#[derive(Arbitrary, Debug, Clone)]
pub enum SystemOp {
    Transfer {
        from: Key,
        to: Key,
        lamports: u64,
    },
    TransferWithSeed {
        base: Key,
        seed: Seed,
        owner: Key,
        to: Key,
        lamports: u64,
    },
    CreateAccount {
        from: Key,
        to: Key,
        lamports: u64,
        space: u16,
        owner: Key,
    },
    CreateAccountWithSeed {
        from: Key,
        base: Key,
        seed: Seed,
        lamports: u64,
        space: u16,
        owner: Key,
    },
    CreateAccountAllowPrefund {
        new_account: Key,
        funder: Option<Key>,
        lamports: u64,
        space: u16,
        owner: Key,
    },
    Assign {
        account: Key,
        owner: Key,
    },
    AssignWithSeed {
        base: Key,
        seed: Seed,
        owner: Key,
    },
    Allocate {
        account: Key,
        space: u16,
    },
    AllocateWithSeed {
        base: Key,
        seed: Seed,
        space: u16,
        owner: Key,
    },
    InitializeNonce {
        from: Key,
        nonce: Key,
        authority: Key,
    },
    AdvanceNonce {
        nonce: Key,
        authority: Key,
    },
    WithdrawNonce {
        nonce: Key,
        authority: Key,
        to: Key,
        lamports: u64,
    },
    AuthorizeNonce {
        nonce: Key,
        authority: Key,
        new_authority: Key,
    },
    UpgradeNonce {
        nonce: Key,
    },
}

#[derive(Arbitrary, Debug, Clone)]
pub enum TokenOp {
    Transfer {
        source: Key,
        destination: Key,
        owner: Key,
        signers: Signers,
        amount: u64,
    },
    TransferChecked {
        source: Key,
        mint: Key,
        destination: Key,
        owner: Key,
        signers: Signers,
        amount: u64,
        decimals: u8,
    },
    Approve {
        source: Key,
        delegate: Key,
        owner: Key,
        signers: Signers,
        amount: u64,
    },
    ApproveChecked {
        source: Key,
        mint: Key,
        delegate: Key,
        owner: Key,
        signers: Signers,
        amount: u64,
        decimals: u8,
    },
    Revoke {
        source: Key,
        owner: Key,
        signers: Signers,
    },
    SetAuthority {
        account: Key,
        new_authority: Option<Key>,
        authority_type: u8,
        owner: Key,
        signers: Signers,
    },
    MintTo {
        mint: Key,
        account: Key,
        authority: Key,
        signers: Signers,
        amount: u64,
    },
    MintToChecked {
        mint: Key,
        account: Key,
        authority: Key,
        signers: Signers,
        amount: u64,
        decimals: u8,
    },
    Burn {
        account: Key,
        mint: Key,
        owner: Key,
        signers: Signers,
        amount: u64,
    },
    BurnChecked {
        account: Key,
        mint: Key,
        owner: Key,
        signers: Signers,
        amount: u64,
        decimals: u8,
    },
    CloseAccount {
        account: Key,
        destination: Key,
        owner: Key,
        signers: Signers,
    },
    FreezeAccount {
        account: Key,
        mint: Key,
        authority: Key,
        signers: Signers,
    },
    ThawAccount {
        account: Key,
        mint: Key,
        authority: Key,
        signers: Signers,
    },
    InitializeMint {
        mint: Key,
        mint_authority: Key,
        freeze_authority: Option<Key>,
        decimals: u8,
        v2: bool,
    },
    InitializeAccount {
        account: Key,
        mint: Key,
        owner: Key,
        variant: u8,
    },
    InitializeMultisig {
        multisig: Key,
        signers: Signers,
        m: u8,
        v2: bool,
    },
    WithdrawExcessLamports {
        source: Key,
        destination: Key,
        authority: Key,
        signers: Signers,
    },
    UnwrapLamports {
        source: Key,
        destination: Key,
        authority: Key,
        signers: Signers,
        amount: Option<u64>,
    },
}

#[derive(Arbitrary, Debug, Clone)]
pub enum Token2022Op {
    Reallocate { account: Key, payer: Key, owner: Key, signers: Signers },
    Pause { mint: Key, authority: Key, signers: Signers },
    Resume { mint: Key, authority: Key, signers: Signers },
    InitializePausable { mint: Key, authority: Key },
    InitializeMintCloseAuthority { mint: Key, close_authority: Option<Key> },
    InitializePermanentDelegate { mint: Key, delegate: Key },
}

#[derive(Arbitrary, Debug, Clone)]
pub enum AltOp {
    Create { authority: Key, payer: Key, recent_slot: u64 },
    Extend { table: Key, authority: Key, payer: Option<Key>, addresses: Vec<Key> },
    Freeze { table: Key, authority: Key },
    Deactivate { table: Key, authority: Key },
    Close { table: Key, authority: Key, recipient: Key },
}

#[derive(Arbitrary, Debug, Clone)]
pub enum LoaderV3Op {
    InitializeBuffer {
        payer: Key,
        buffer: Key,
        authority: Option<Key>,
    },
    Write {
        buffer: Key,
        authority: Key,
        offset: u32,
        bytes: Bytes,
    },
    DeployWithMaxDataLen {
        payer: Key,
        program: Key,
        buffer: Key,
        authority: Key,
        lamports: u64,
        max_data_len: u16,
    },
    Upgrade {
        program: Key,
        buffer: Key,
        authority: Key,
        spill: Key,
    },
    SetBufferAuthority {
        buffer: Key,
        current: Key,
        new: Key,
    },
    SetBufferAuthorityChecked {
        buffer: Key,
        current: Key,
        new: Key,
    },
    SetUpgradeAuthority {
        program: Key,
        current: Key,
        new: Option<Key>,
    },
    SetUpgradeAuthorityChecked {
        program: Key,
        current: Key,
        new: Key,
    },
    Close {
        account: Key,
        recipient: Key,
        authority: Option<Key>,
        program: Option<Key>,
    },
    ExtendProgram {
        program: Key,
        payer: Option<Key>,
        additional_bytes: u32,
    },
    ExtendProgramChecked {
        program: Key,
        authority: Key,
        payer: Option<Key>,
        additional_bytes: u32,
    },
    Migrate {
        programdata: Key,
        program: Key,
        authority: Key,
    },
}

#[derive(Arbitrary, Debug, Clone)]
pub enum LoaderV4Op {
    Write {
        program: Key,
        authority: Key,
        offset: u32,
        bytes: Bytes,
    },
    Copy {
        program: Key,
        authority: Key,
        source: Key,
        destination_offset: u32,
        source_offset: u32,
        length: u32,
    },
    SetProgramLength {
        program: Key,
        authority: Key,
        new_size: u32,
        recipient: Option<Key>,
    },
    Deploy {
        program: Key,
        authority: Key,
        source: Option<Key>,
    },
    Retract {
        program: Key,
        authority: Key,
    },
    TransferAuthority {
        program: Key,
        current: Key,
        new: Key,
    },
    Finalize {
        program: Key,
        authority: Key,
        next_version: Key,
    },
}

#[derive(Arbitrary, Debug, Clone, Copy)]
pub enum RawProgram {
    System,
    SplToken,
    Token2022,
    AssociatedToken,
    AddressLookupTable,
    LoaderV3,
    LoaderV4,
}

impl RawProgram {
    fn id(self) -> Pubkey {
        match self {
            Self::System => solana_system_interface::program::ID,
            Self::SplToken => spl_token_interface::ID,
            Self::Token2022 => spl_token_2022_interface::ID,
            Self::AssociatedToken => spl_associated_token_account_interface::program::ID,
            Self::AddressLookupTable => solana_address_lookup_table_interface::program::ID,
            Self::LoaderV3 => BPF_LOADER_UPGRADEABLE_PROGRAM_ID,
            Self::LoaderV4 => LOADER_V4_PROGRAM_ID,
        }
    }
}

#[derive(Arbitrary, Debug, Clone)]
pub struct RawAccount {
    key: Key,
    is_signer: bool,
    is_writable: bool,
}

#[derive(Arbitrary, Debug, Clone)]
pub enum Op {
    System(SystemOp),
    Token(TokenProgram, TokenOp),
    Token2022(Token2022Op),
    SplBatch(Vec<TokenOp>),
    CreateAta { funder: Key, wallet: Key, mint: Key, program: TokenProgram, idempotent: bool },
    Alt(AltOp),
    LoaderV3(LoaderV3Op),
    LoaderV4(LoaderV4Op),
    Raw { program: RawProgram, accounts: Vec<RawAccount>, data: Vec<u8> },
}

impl Op {
    pub fn batch_ops(ops: &[TokenOp]) -> &[TokenOp] {
        &ops[..ops.len().min(MAX_BATCH_OPS)]
    }

    pub fn instruction(&self) -> Option<Instruction> {
        match self {
            Self::System(op) => op.instruction(),
            Self::Token(program, op) => op.instruction(*program),
            Self::Token2022(op) => op.instruction(),
            Self::SplBatch(ops) => {
                let instructions = Self::batch_ops(ops)
                    .iter()
                    .map(|op| op.instruction(TokenProgram::Spl))
                    .collect::<Option<Vec<_>>>()?;
                spl_token_interface::instruction::batch(&spl_token_interface::ID, &instructions)
                    .ok()
            }
            Self::CreateAta { funder, wallet, mint, program, idempotent } => {
                let build = if *idempotent {
                    spl_associated_token_account_interface::instruction::create_associated_token_account_idempotent
                } else {
                    spl_associated_token_account_interface::instruction::create_associated_token_account
                };
                Some(build(&funder.pubkey(), &wallet.pubkey(), &mint.pubkey(), &program.id()))
            }
            Self::Alt(op) => Some(op.instruction()),
            Self::LoaderV3(op) => op.instruction(),
            Self::LoaderV4(op) => Some(op.instruction()),
            Self::Raw { program, accounts, data } => Some(Instruction {
                program_id: program.id(),
                accounts: accounts
                    .iter()
                    .take(MAX_RAW_ACCOUNTS)
                    .map(|account| AccountMeta {
                        pubkey: account.key.pubkey(),
                        is_signer: account.is_signer,
                        is_writable: account.is_writable,
                    })
                    .collect(),
                data: data.clone(),
            }),
        }
    }
}

impl SystemOp {
    fn instruction(&self) -> Option<Instruction> {
        use solana_system_interface::instruction as system;

        Some(match self {
            Self::Transfer { from, to, lamports } => {
                system::transfer(&from.pubkey(), &to.pubkey(), *lamports)
            }
            Self::TransferWithSeed { base, seed, owner, to, lamports } => {
                let seed = seed.as_string();
                let from = Pubkey::create_with_seed(&base.pubkey(), &seed, &owner.pubkey()).ok()?;
                system::transfer_with_seed(
                    &from,
                    &base.pubkey(),
                    seed,
                    &owner.pubkey(),
                    &to.pubkey(),
                    *lamports,
                )
            }
            Self::CreateAccount { from, to, lamports, space, owner } => system::create_account(
                &from.pubkey(),
                &to.pubkey(),
                *lamports,
                u64::from(*space),
                &owner.pubkey(),
            ),
            Self::CreateAccountWithSeed { from, base, seed, lamports, space, owner } => {
                let seed = seed.as_string();
                let to = Pubkey::create_with_seed(&base.pubkey(), &seed, &owner.pubkey()).ok()?;
                system::create_account_with_seed(
                    &from.pubkey(),
                    &to,
                    &base.pubkey(),
                    &seed,
                    *lamports,
                    u64::from(*space),
                    &owner.pubkey(),
                )
            }
            Self::CreateAccountAllowPrefund { new_account, funder, lamports, space, owner } => {
                let mut data = CREATE_ACCOUNT_ALLOW_PREFUND_TAG.to_le_bytes().to_vec();
                data.extend_from_slice(&lamports.to_le_bytes());
                data.extend_from_slice(&u64::from(*space).to_le_bytes());
                data.extend_from_slice(owner.pubkey().as_ref());
                let mut accounts = vec![AccountMeta::new(new_account.pubkey(), true)];
                accounts.extend(funder.map(|funder| AccountMeta::new(funder.pubkey(), true)));
                Instruction { program_id: solana_system_interface::program::ID, accounts, data }
            }
            Self::Assign { account, owner } => system::assign(&account.pubkey(), &owner.pubkey()),
            Self::AssignWithSeed { base, seed, owner } => {
                let seed = seed.as_string();
                let address =
                    Pubkey::create_with_seed(&base.pubkey(), &seed, &owner.pubkey()).ok()?;
                system::assign_with_seed(&address, &base.pubkey(), &seed, &owner.pubkey())
            }
            Self::Allocate { account, space } => {
                system::allocate(&account.pubkey(), u64::from(*space))
            }
            Self::AllocateWithSeed { base, seed, space, owner } => {
                let seed = seed.as_string();
                let address =
                    Pubkey::create_with_seed(&base.pubkey(), &seed, &owner.pubkey()).ok()?;
                system::allocate_with_seed(
                    &address,
                    &base.pubkey(),
                    &seed,
                    u64::from(*space),
                    &owner.pubkey(),
                )
            }
            Self::InitializeNonce { from, nonce, authority } => system::create_nonce_account(
                &from.pubkey(),
                &nonce.pubkey(),
                &authority.pubkey(),
                1,
            )
            .pop()?,
            Self::AdvanceNonce { nonce, authority } => {
                system::advance_nonce_account(&nonce.pubkey(), &authority.pubkey())
            }
            Self::WithdrawNonce { nonce, authority, to, lamports } => {
                system::withdraw_nonce_account(
                    &nonce.pubkey(),
                    &authority.pubkey(),
                    &to.pubkey(),
                    *lamports,
                )
            }
            Self::AuthorizeNonce { nonce, authority, new_authority } => {
                system::authorize_nonce_account(
                    &nonce.pubkey(),
                    &authority.pubkey(),
                    &new_authority.pubkey(),
                )
            }
            Self::UpgradeNonce { nonce } => system::upgrade_nonce_account(nonce.pubkey()),
        })
    }
}

macro_rules! token_ix {
    ($program:expr, $name:ident($($arg:expr),* $(,)?)) => {
        match $program {
            TokenProgram::Spl => {
                spl_token_interface::instruction::$name(&spl_token_interface::ID, $($arg),*).ok()
            }
            TokenProgram::Token2022 => spl_token_2022_interface::instruction::$name(
                &spl_token_2022_interface::ID,
                $($arg),*
            )
            .ok(),
        }
    };
}

impl TokenOp {
    #[allow(deprecated)]
    fn instruction(&self, program: TokenProgram) -> Option<Instruction> {
        match self {
            Self::Transfer { source, destination, owner, signers, amount } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    transfer(
                        &source.pubkey(),
                        &destination.pubkey(),
                        &owner.pubkey(),
                        &signers,
                        *amount
                    )
                )
            }
            Self::TransferChecked {
                source,
                mint,
                destination,
                owner,
                signers,
                amount,
                decimals,
            } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    transfer_checked(
                        &source.pubkey(),
                        &mint.pubkey(),
                        &destination.pubkey(),
                        &owner.pubkey(),
                        &signers,
                        *amount,
                        *decimals
                    )
                )
            }
            Self::Approve { source, delegate, owner, signers, amount } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    approve(
                        &source.pubkey(),
                        &delegate.pubkey(),
                        &owner.pubkey(),
                        &signers,
                        *amount
                    )
                )
            }
            Self::ApproveChecked { source, mint, delegate, owner, signers, amount, decimals } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    approve_checked(
                        &source.pubkey(),
                        &mint.pubkey(),
                        &delegate.pubkey(),
                        &owner.pubkey(),
                        &signers,
                        *amount,
                        *decimals
                    )
                )
            }
            Self::Revoke { source, owner, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(program, revoke(&source.pubkey(), &owner.pubkey(), &signers))
            }
            Self::SetAuthority { account, new_authority, authority_type, owner, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                let new_authority = new_authority.map(Key::pubkey);
                match program {
                    TokenProgram::Spl => {
                        use spl_token_interface::instruction::AuthorityType;
                        let authority_type = match authority_type % 4 {
                            0 => AuthorityType::MintTokens,
                            1 => AuthorityType::FreezeAccount,
                            2 => AuthorityType::AccountOwner,
                            _ => AuthorityType::CloseAccount,
                        };
                        spl_token_interface::instruction::set_authority(
                            &spl_token_interface::ID,
                            &account.pubkey(),
                            new_authority.as_ref(),
                            authority_type,
                            &owner.pubkey(),
                            &signers,
                        )
                        .ok()
                    }
                    TokenProgram::Token2022 => {
                        use spl_token_2022_interface::instruction::AuthorityType;
                        let authority_type = match authority_type % 4 {
                            0 => AuthorityType::MintTokens,
                            1 => AuthorityType::FreezeAccount,
                            2 => AuthorityType::AccountOwner,
                            _ => AuthorityType::CloseAccount,
                        };
                        spl_token_2022_interface::instruction::set_authority(
                            &spl_token_2022_interface::ID,
                            &account.pubkey(),
                            new_authority.as_ref(),
                            authority_type,
                            &owner.pubkey(),
                            &signers,
                        )
                        .ok()
                    }
                }
            }
            Self::MintTo { mint, account, authority, signers, amount } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    mint_to(
                        &mint.pubkey(),
                        &account.pubkey(),
                        &authority.pubkey(),
                        &signers,
                        *amount
                    )
                )
            }
            Self::MintToChecked { mint, account, authority, signers, amount, decimals } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    mint_to_checked(
                        &mint.pubkey(),
                        &account.pubkey(),
                        &authority.pubkey(),
                        &signers,
                        *amount,
                        *decimals
                    )
                )
            }
            Self::Burn { account, mint, owner, signers, amount } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    burn(&account.pubkey(), &mint.pubkey(), &owner.pubkey(), &signers, *amount)
                )
            }
            Self::BurnChecked { account, mint, owner, signers, amount, decimals } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    burn_checked(
                        &account.pubkey(),
                        &mint.pubkey(),
                        &owner.pubkey(),
                        &signers,
                        *amount,
                        *decimals
                    )
                )
            }
            Self::CloseAccount { account, destination, owner, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    close_account(
                        &account.pubkey(),
                        &destination.pubkey(),
                        &owner.pubkey(),
                        &signers
                    )
                )
            }
            Self::FreezeAccount { account, mint, authority, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    freeze_account(
                        &account.pubkey(),
                        &mint.pubkey(),
                        &authority.pubkey(),
                        &signers
                    )
                )
            }
            Self::ThawAccount { account, mint, authority, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    thaw_account(&account.pubkey(), &mint.pubkey(), &authority.pubkey(), &signers)
                )
            }
            Self::InitializeMint { mint, mint_authority, freeze_authority, decimals, v2 } => {
                let freeze_authority = freeze_authority.map(Key::pubkey);
                if *v2 {
                    token_ix!(
                        program,
                        initialize_mint2(
                            &mint.pubkey(),
                            &mint_authority.pubkey(),
                            freeze_authority.as_ref(),
                            *decimals
                        )
                    )
                } else {
                    token_ix!(
                        program,
                        initialize_mint(
                            &mint.pubkey(),
                            &mint_authority.pubkey(),
                            freeze_authority.as_ref(),
                            *decimals
                        )
                    )
                }
            }
            Self::InitializeAccount { account, mint, owner, variant } => match variant % 3 {
                0 => token_ix!(
                    program,
                    initialize_account(&account.pubkey(), &mint.pubkey(), &owner.pubkey())
                ),
                1 => token_ix!(
                    program,
                    initialize_account2(&account.pubkey(), &mint.pubkey(), &owner.pubkey())
                ),
                _ => token_ix!(
                    program,
                    initialize_account3(&account.pubkey(), &mint.pubkey(), &owner.pubkey())
                ),
            },
            Self::InitializeMultisig { multisig, signers, m, v2 } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                if *v2 {
                    token_ix!(program, initialize_multisig2(&multisig.pubkey(), &signers, *m))
                } else {
                    token_ix!(program, initialize_multisig(&multisig.pubkey(), &signers, *m))
                }
            }
            Self::WithdrawExcessLamports { source, destination, authority, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                token_ix!(
                    program,
                    withdraw_excess_lamports(
                        &source.pubkey(),
                        &destination.pubkey(),
                        &authority.pubkey(),
                        &signers
                    )
                )
            }
            Self::UnwrapLamports { source, destination, authority, signers, amount } => {
                if program != TokenProgram::Spl {
                    return None;
                }
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                spl_token_interface::instruction::unwrap_lamports(
                    &spl_token_interface::ID,
                    &source.pubkey(),
                    &destination.pubkey(),
                    &authority.pubkey(),
                    &signers,
                    *amount,
                )
                .ok()
            }
        }
    }
}

impl Token2022Op {
    fn instruction(&self) -> Option<Instruction> {
        use spl_token_2022_interface::{extension::pausable::instruction as pausable, instruction};

        let program = &spl_token_2022_interface::ID;
        match self {
            Self::Reallocate { account, payer, owner, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                instruction::reallocate(
                    program,
                    &account.pubkey(),
                    &payer.pubkey(),
                    &owner.pubkey(),
                    &signers,
                    &[],
                )
            }
            Self::Pause { mint, authority, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                pausable::pause(program, &mint.pubkey(), &authority.pubkey(), &signers)
            }
            Self::Resume { mint, authority, signers } => {
                let signers = signers.pubkeys();
                let signers: Vec<&Pubkey> = signers.iter().collect();
                pausable::resume(program, &mint.pubkey(), &authority.pubkey(), &signers)
            }
            Self::InitializePausable { mint, authority } => {
                pausable::initialize(program, &mint.pubkey(), &authority.pubkey())
            }
            Self::InitializeMintCloseAuthority { mint, close_authority } => {
                let close_authority = close_authority.map(Key::pubkey);
                instruction::initialize_mint_close_authority(
                    program,
                    &mint.pubkey(),
                    close_authority.as_ref(),
                )
            }
            Self::InitializePermanentDelegate { mint, delegate } => {
                instruction::initialize_permanent_delegate(
                    program,
                    &mint.pubkey(),
                    &delegate.pubkey(),
                )
            }
        }
        .ok()
    }
}

impl AltOp {
    fn instruction(&self) -> Instruction {
        use solana_address_lookup_table_interface::instruction as alt;

        match self {
            Self::Create { authority, payer, recent_slot } => {
                alt::create_lookup_table(authority.pubkey(), payer.pubkey(), *recent_slot).0
            }
            Self::Extend { table, authority, payer, addresses } => alt::extend_lookup_table(
                table.pubkey(),
                authority.pubkey(),
                payer.map(Key::pubkey),
                addresses.iter().take(MAX_SIGNERS).map(|key| key.pubkey()).collect(),
            ),
            Self::Freeze { table, authority } => {
                alt::freeze_lookup_table(table.pubkey(), authority.pubkey())
            }
            Self::Deactivate { table, authority } => {
                alt::deactivate_lookup_table(table.pubkey(), authority.pubkey())
            }
            Self::Close { table, authority, recipient } => {
                alt::close_lookup_table(table.pubkey(), authority.pubkey(), recipient.pubkey())
            }
        }
    }
}

impl LoaderV3Op {
    fn instruction(&self) -> Option<Instruction> {
        use solana_loader_v3_interface::instruction as v3;

        Some(match self {
            Self::InitializeBuffer { payer, buffer, authority } => {
                let mut instruction = v3::create_buffer(
                    &payer.pubkey(),
                    &buffer.pubkey(),
                    &authority.unwrap_or(*buffer).pubkey(),
                    1,
                    0,
                )
                .ok()?
                .pop()?;
                if authority.is_none() {
                    instruction.accounts.truncate(1);
                }
                instruction
            }
            Self::Write { buffer, authority, offset, bytes } => {
                v3::write(&buffer.pubkey(), &authority.pubkey(), *offset, bytes.to_vec())
            }
            Self::DeployWithMaxDataLen {
                payer,
                program,
                buffer,
                authority,
                lamports,
                max_data_len,
            } => v3::deploy_with_max_program_len(
                &payer.pubkey(),
                &program.pubkey(),
                &buffer.pubkey(),
                &authority.pubkey(),
                *lamports,
                usize::from(*max_data_len),
            )
            .ok()?
            .pop()?,
            Self::Upgrade { program, buffer, authority, spill } => v3::upgrade(
                &program.pubkey(),
                &buffer.pubkey(),
                &authority.pubkey(),
                &spill.pubkey(),
            ),
            Self::SetBufferAuthority { buffer, current, new } => {
                v3::set_buffer_authority(&buffer.pubkey(), &current.pubkey(), &new.pubkey())
            }
            Self::SetBufferAuthorityChecked { buffer, current, new } => {
                v3::set_buffer_authority_checked(&buffer.pubkey(), &current.pubkey(), &new.pubkey())
            }
            Self::SetUpgradeAuthority { program, current, new } => {
                let new = new.map(Key::pubkey);
                v3::set_upgrade_authority(&program.pubkey(), &current.pubkey(), new.as_ref())
            }
            Self::SetUpgradeAuthorityChecked { program, current, new } => {
                v3::set_upgrade_authority_checked(
                    &program.pubkey(),
                    &current.pubkey(),
                    &new.pubkey(),
                )
            }
            Self::Close { account, recipient, authority, program } => {
                let authority = authority.map(Key::pubkey);
                let program = program.map(Key::pubkey);
                v3::close_any(
                    &account.pubkey(),
                    &recipient.pubkey(),
                    authority.as_ref(),
                    program.as_ref(),
                )
            }
            Self::ExtendProgram { program, payer, additional_bytes } => {
                let payer = payer.map(Key::pubkey);
                v3::extend_program(&program.pubkey(), payer.as_ref(), *additional_bytes)
            }
            Self::ExtendProgramChecked { program, authority, payer, additional_bytes } => {
                let payer = payer.map(Key::pubkey);
                v3::extend_program_checked(
                    &program.pubkey(),
                    &authority.pubkey(),
                    payer.as_ref(),
                    *additional_bytes,
                )
            }
            Self::Migrate { programdata, program, authority } => {
                v3::migrate_program(&programdata.pubkey(), &program.pubkey(), &authority.pubkey())
            }
        })
    }
}

impl LoaderV4Op {
    fn instruction(&self) -> Instruction {
        use solana_loader_v4_interface::instruction as v4;

        match self {
            Self::Write { program, authority, offset, bytes } => {
                v4::write(&program.pubkey(), &authority.pubkey(), *offset, bytes.to_vec())
            }
            Self::Copy {
                program,
                authority,
                source,
                destination_offset,
                source_offset,
                length,
            } => v4::copy(
                &program.pubkey(),
                &authority.pubkey(),
                &source.pubkey(),
                *destination_offset,
                *source_offset,
                *length,
            ),
            Self::SetProgramLength { program, authority, new_size, recipient } => {
                let mut instruction = v4::set_program_length(
                    &program.pubkey(),
                    &authority.pubkey(),
                    *new_size,
                    &recipient.unwrap_or(*program).pubkey(),
                );
                if recipient.is_none() {
                    instruction.accounts.truncate(2);
                }
                instruction
            }
            Self::Deploy { program, authority, source: Some(source) } => {
                v4::deploy_from_source(&program.pubkey(), &authority.pubkey(), &source.pubkey())
            }
            Self::Deploy { program, authority, source: None } => {
                v4::deploy(&program.pubkey(), &authority.pubkey())
            }
            Self::Retract { program, authority } => {
                v4::retract(&program.pubkey(), &authority.pubkey())
            }
            Self::TransferAuthority { program, current, new } => {
                v4::transfer_authority(&program.pubkey(), &current.pubkey(), &new.pubkey())
            }
            Self::Finalize { program, authority, next_version } => {
                v4::finalize(&program.pubkey(), &authority.pubkey(), &next_version.pubkey())
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
pub struct Scenario {
    pub version: MessageVersion,
    pub policy_seed: u128,
    pub allow_durable_transactions: bool,
    pub ops: Vec<Op>,
}

impl Scenario {
    pub fn built_ops(&self) -> Vec<(&Op, Instruction)> {
        self.ops.iter().take(MAX_OPS).filter_map(|op| Some((op, op.instruction()?))).collect()
    }

    pub fn transaction(&self, instructions: &[Instruction]) -> Option<VersionedTransaction> {
        let message = match self.version {
            MessageVersion::Legacy => {
                VersionedMessage::Legacy(Message::new(instructions, Some(&FEE_PAYER)))
            }
            MessageVersion::V0 => VersionedMessage::V0(
                v0::Message::try_compile(&FEE_PAYER, instructions, &[], Hash::default()).ok()?,
            ),
            MessageVersion::V1 => VersionedMessage::V1(
                v1::Message::try_compile(&FEE_PAYER, instructions, Hash::default()).ok()?,
            ),
        };
        let signatures = usize::from(message.header().num_required_signatures);
        Some(VersionedTransaction { signatures: vec![Default::default(); signatures], message })
    }

    pub fn fee_payer_policy(&self) -> FeePayerPolicy {
        let mut policy =
            serde_json::to_value(FeePayerPolicy::default()).expect("FeePayerPolicy must serialize");
        set_flags(&mut policy, self.policy_seed, &mut 0);
        serde_json::from_value(policy).expect("randomized FeePayerPolicy must deserialize")
    }

    pub fn config(&self) -> Config {
        serde_json::from_value(json!({
            "validation": {
                "max_allowed_lamports": u64::MAX,
                "max_signatures": u64::MAX,
                "allowed_programs": "All",
                "allowed_tokens": [],
                "allowed_spl_paid_tokens": "All",
                "disallowed_accounts": [],
                "price_source": "Mock",
                "fee_payer_policy": self.fee_payer_policy(),
                "allow_durable_transactions": self.allow_durable_transactions,
            },
            "kora": {},
        }))
        .expect("fuzz Config must deserialize")
    }
}

fn set_flags(value: &mut Value, seed: u128, next_bit: &mut u32) {
    match value {
        Value::Bool(flag) => {
            *flag = (seed >> (*next_bit % u128::BITS)) & 1 == 1;
            *next_bit += 1;
        }
        Value::Object(fields) => {
            for field in fields.values_mut() {
                set_flags(field, seed, next_bit);
            }
        }
        _ => {}
    }
}
