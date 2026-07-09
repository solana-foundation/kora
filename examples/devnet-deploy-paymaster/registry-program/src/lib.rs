use solana_program::{
    account_info::{next_account_info, AccountInfo},
    declare_id,
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use solana_system_interface::instruction as system_instruction;

declare_id!("CPoBCCbvawmR2S6joHjXgfFkh9pGqzSixBe2BaBwbVkx");

pub const BPF_LOADER_UPGRADEABLE: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

/// Entry layout: owner wallet (32) | rent payer (32) | bump (1).
pub const ENTRY_LEN: usize = 65;
pub const ENTRY_OWNER_OFFSET: usize = 0;
pub const ENTRY_PAYER_OFFSET: usize = 32;

pub const IX_REGISTER: u8 = 0;
pub const IX_CLOSE_ENTRY: u8 = 1;

pub fn entry_address(program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[program.as_ref()], &id())
}

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.first() {
        Some(&IX_REGISTER) => register(program_id, accounts),
        Some(&IX_CLOSE_ENTRY) => close_entry(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn register(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let payer = next_account_info(iter)?;
    let program = next_account_info(iter)?;
    let owner = next_account_info(iter)?;
    let entry = next_account_info(iter)?;
    let _system_program = next_account_info(iter)?;

    if !payer.is_signer || !program.is_signer || !owner.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (expected, bump) = entry_address(program.key);
    if entry.key != &expected {
        return Err(ProgramError::InvalidSeeds);
    }
    if entry.owner == program_id || !entry.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::get()?.minimum_balance(ENTRY_LEN);
    let seeds: &[&[u8]] = &[program.key.as_ref(), &[bump]];

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            entry.key,
            rent,
            ENTRY_LEN as u64,
            program_id,
        ),
        &[payer.clone(), entry.clone()],
        &[seeds],
    )?;

    let mut data = entry.try_borrow_mut_data()?;
    data[ENTRY_OWNER_OFFSET..ENTRY_OWNER_OFFSET + 32].copy_from_slice(owner.key.as_ref());
    data[ENTRY_PAYER_OFFSET..ENTRY_PAYER_OFFSET + 32].copy_from_slice(payer.key.as_ref());
    data[64] = bump;
    Ok(())
}

fn close_entry(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let iter = &mut accounts.iter();
    let entry = next_account_info(iter)?;
    let program = next_account_info(iter)?;
    let program_data = next_account_info(iter)?;
    let recipient = next_account_info(iter)?;

    if entry.owner != program_id || entry.data_len() != ENTRY_LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    let (expected, _) = entry_address(program.key);
    if entry.key != &expected {
        return Err(ProgramError::InvalidSeeds);
    }

    let (expected_program_data, _) =
        Pubkey::find_program_address(&[program.key.as_ref()], &BPF_LOADER_UPGRADEABLE);
    if program_data.key != &expected_program_data {
        return Err(ProgramError::InvalidSeeds);
    }
    if program_data.lamports() != 0 {
        return Err(ProgramError::InvalidAccountData);
    }

    let stored_payer =
        Pubkey::try_from(&entry.try_borrow_data()?[ENTRY_PAYER_OFFSET..ENTRY_PAYER_OFFSET + 32])
            .map_err(|_| ProgramError::InvalidAccountData)?;
    if recipient.key != &stored_payer {
        return Err(ProgramError::InvalidArgument);
    }

    let lamports = entry.lamports();
    **entry.try_borrow_mut_lamports()? = 0;
    **recipient.try_borrow_mut_lamports()? += lamports;
    entry.try_borrow_mut_data()?.fill(0);
    Ok(())
}
