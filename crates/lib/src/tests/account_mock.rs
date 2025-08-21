use solana_program::program_pack::Pack;
use solana_sdk::{account::Account, program_option::COption, pubkey::Pubkey};
use spl_token::state::{Account as TokenAccount, AccountState, Mint};
use spl_token_2022::state::Mint as Mint2022;

pub struct AccountMockBuilder {
    lamports: u64,
    data: Vec<u8>,
    owner: Pubkey,
    executable: bool,
    rent_epoch: u64,
}

impl Default for AccountMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AccountMockBuilder {
    pub fn new() -> Self {
        Self {
            lamports: 1000000,
            data: vec![0u8; 100],
            owner: Pubkey::new_unique(),
            executable: false,
            rent_epoch: 0,
        }
    }

    pub fn with_lamports(mut self, lamports: u64) -> Self {
        self.lamports = lamports;
        self
    }

    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    pub fn with_owner(mut self, owner: Pubkey) -> Self {
        self.owner = owner;
        self
    }

    pub fn with_executable(mut self, executable: bool) -> Self {
        self.executable = executable;
        self
    }

    pub fn with_rent_epoch(mut self, rent_epoch: u64) -> Self {
        self.rent_epoch = rent_epoch;
        self
    }

    pub fn build(self) -> Account {
        Account {
            lamports: self.lamports,
            data: self.data,
            owner: self.owner,
            executable: self.executable,
            rent_epoch: self.rent_epoch,
        }
    }
}

pub struct TokenAccountMockBuilder {
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
    delegate: COption<Pubkey>,
    state: AccountState,
    is_native: COption<u64>,
    delegated_amount: u64,
    close_authority: COption<Pubkey>,
    lamports: u64,
    rent_epoch: u64,
}

impl Default for TokenAccountMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenAccountMockBuilder {
    pub fn new() -> Self {
        Self {
            mint: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            amount: 100,
            delegate: COption::None,
            state: AccountState::Initialized,
            is_native: COption::Some(0),
            delegated_amount: 0,
            close_authority: COption::None,
            lamports: 1000000,
            rent_epoch: 0,
        }
    }

    pub fn with_mint(mut self, mint: &Pubkey) -> Self {
        self.mint = *mint;
        self
    }

    pub fn with_owner(mut self, owner: &Pubkey) -> Self {
        self.owner = *owner;
        self
    }

    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn with_delegate(mut self, delegate: Option<Pubkey>) -> Self {
        self.delegate = match delegate {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    pub fn with_state(mut self, state: AccountState) -> Self {
        self.state = state;
        self
    }

    pub fn with_native(mut self, native_amount: Option<u64>) -> Self {
        self.is_native = match native_amount {
            Some(amount) => COption::Some(amount),
            None => COption::None,
        };
        self
    }

    pub fn with_delegated_amount(mut self, amount: u64) -> Self {
        self.delegated_amount = amount;
        self
    }

    pub fn with_close_authority(mut self, authority: Option<Pubkey>) -> Self {
        self.close_authority = match authority {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    pub fn with_lamports(mut self, lamports: u64) -> Self {
        self.lamports = lamports;
        self
    }

    pub fn build(self) -> Account {
        let token_account = TokenAccount {
            mint: self.mint,
            owner: self.owner,
            amount: self.amount,
            delegate: self.delegate,
            state: self.state,
            is_native: self.is_native,
            delegated_amount: self.delegated_amount,
            close_authority: self.close_authority,
        };

        let mut data = vec![0u8; TokenAccount::LEN];
        token_account.pack_into_slice(&mut data);

        Account {
            lamports: self.lamports,
            data,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: self.rent_epoch,
        }
    }
}

pub struct MintAccountMockBuilder {
    mint_authority: COption<Pubkey>,
    supply: u64,
    decimals: u8,
    is_initialized: bool,
    freeze_authority: COption<Pubkey>,
    lamports: u64,
    rent_epoch: u64,
}

impl Default for MintAccountMockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MintAccountMockBuilder {
    pub fn new() -> Self {
        Self {
            mint_authority: COption::Some(Pubkey::new_unique()),
            supply: 1_000_000_000_000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
            lamports: 0,
            rent_epoch: 0,
        }
    }

    pub fn with_mint_authority(mut self, authority: Option<Pubkey>) -> Self {
        self.mint_authority = match authority {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    pub fn with_supply(mut self, supply: u64) -> Self {
        self.supply = supply;
        self
    }

    pub fn with_decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn with_initialized(mut self, initialized: bool) -> Self {
        self.is_initialized = initialized;
        self
    }

    pub fn with_freeze_authority(mut self, authority: Option<Pubkey>) -> Self {
        self.freeze_authority = match authority {
            Some(key) => COption::Some(key),
            None => COption::None,
        };
        self
    }

    pub fn with_lamports(mut self, lamports: u64) -> Self {
        self.lamports = lamports;
        self
    }

    pub fn build(self) -> Account {
        let mint_data = Mint {
            mint_authority: self.mint_authority,
            supply: self.supply,
            decimals: self.decimals,
            is_initialized: self.is_initialized,
            freeze_authority: self.freeze_authority,
        };

        let mut data = vec![0u8; Mint::LEN];
        mint_data.pack_into_slice(&mut data);

        Account {
            lamports: self.lamports,
            data,
            owner: spl_token::id(),
            executable: false,
            rent_epoch: self.rent_epoch,
        }
    }

    pub fn build_token2022(self) -> Account {
        let mint_data = Mint2022 {
            mint_authority: self.mint_authority,
            supply: self.supply,
            decimals: self.decimals,
            is_initialized: self.is_initialized,
            freeze_authority: self.freeze_authority,
        };

        let mut data = vec![0u8; Mint2022::LEN];
        mint_data.pack_into_slice(&mut data);

        Account {
            lamports: self.lamports,
            data,
            owner: spl_token_2022::id(),
            executable: false,
            rent_epoch: self.rent_epoch,
        }
    }
}

pub fn create_mock_account() -> Account {
    AccountMockBuilder::new().build()
}

pub fn create_mock_program_account() -> Account {
    AccountMockBuilder::new()
        .with_executable(true)
        .with_owner(Pubkey::new_unique()) // Programs are owned by the loader
        .with_data(vec![0u8; 100])
        .build()
}

pub fn create_mock_non_executable_account() -> Account {
    AccountMockBuilder::new().with_executable(false).build()
}

pub fn create_mock_token_account(owner: &Pubkey, mint: &Pubkey) -> Account {
    TokenAccountMockBuilder::new().with_owner(owner).with_mint(mint).build()
}

pub fn create_mock_spl_mint_account(decimals: u8) -> Account {
    MintAccountMockBuilder::new().with_decimals(decimals).build()
}

pub fn create_mock_token2022_mint_account(decimals: u8) -> Account {
    MintAccountMockBuilder::new().with_decimals(decimals).build_token2022()
}

pub fn create_mock_account_with_balance(lamports: u64) -> Account {
    AccountMockBuilder::new().with_lamports(lamports).build()
}

pub fn create_mock_account_with_owner(owner: Pubkey) -> Account {
    AccountMockBuilder::new().with_owner(owner).build()
}

pub fn create_mock_usdc_mint_account() -> Account {
    MintAccountMockBuilder::new()
        .with_decimals(6)
        .with_supply(1_000_000_000_000) // 1M USDC with 6 decimals
        .build()
}

/// Create mock SOL wrapped token mint (9 decimals)
pub fn create_mock_wsol_mint_account() -> Account {
    MintAccountMockBuilder::new().with_decimals(9).build()
}
