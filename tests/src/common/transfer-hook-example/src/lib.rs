use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};

use spl_tlv_account_resolution::{account::ExtraAccountMeta, state::ExtraAccountMetaList};

solana_program::declare_id!("Bcdikjss8HWzKEuj6gEQoFq9TCnGnk6v3kUnRU1gb6hA");

// Program entrypoint
entrypoint!(process_instruction);

// Main processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match TransferHookInstruction::unpack(instruction_data)? {
        TransferHookInstruction::Execute { amount } => {
            msg!("Transfer hook execute: amount {}", amount);
            execute_transfer_hook(program_id, accounts, amount)
        }
        TransferHookInstruction::InitializeExtraAccountMetaList { extra_account_metas } => {
            msg!("Transfer hook initialize extra account meta list");
            initialize_extra_account_meta_list(program_id, accounts, &extra_account_metas)
        }
        TransferHookInstruction::UpdateExtraAccountMetaList { extra_account_metas } => {
            msg!("Transfer hook update extra account meta list");
            update_extra_account_meta_list(program_id, accounts, &extra_account_metas)
        }
    }
}

// Execute transfer hook logic
fn execute_transfer_hook(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    // Expected accounts for transfer hook Execute instruction:
    // 0. Source token account
    // 1. Mint
    // 2. Destination token account
    // 3. Owner/Authority
    // 4. Extra account meta list (required)
    // ... any additional accounts from ExtraAccountMetaList

    // Simple logic: block transfers over 1 million tokens (adjust decimals as needed)
    if amount > 1_000_000 {
        msg!("Transfer blocked: amount {} exceeds limit", amount);
        return Err(ProgramError::Custom(1));
    }

    msg!("Transfer allowed");
    Ok(())
}

// Initialize Extra Account Meta List using proper TLV format
fn initialize_extra_account_meta_list(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    extra_account_metas: &[ExtraAccountMeta],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    // Expected accounts for InitializeExtraAccountMetaList:
    // 0. Extra account meta list (writable, uninitialized PDA)
    // 1. Mint
    // 2. Payer/authority (signer)
    // 3. System program

    let extra_account_meta_list_info = next_account_info(accounts_iter)?;
    let mint_info = next_account_info(accounts_iter)?;
    let payer_info = next_account_info(accounts_iter)?;
    let system_program_info = next_account_info(accounts_iter)?;

    // Derive the expected PDA address
    let seeds = &[b"extra-account-metas", mint_info.key.as_ref()];
    let (expected_address, bump) = Pubkey::find_program_address(seeds, program_id);

    // Verify this is the correct PDA
    if expected_address != *extra_account_meta_list_info.key {
        msg!("Error: Extra Account Meta List address mismatch");
        msg!("Expected: {}, Got: {}", expected_address, extra_account_meta_list_info.key);
        return Err(ProgramError::InvalidSeeds);
    }

    // Check if account is already initialized
    if extra_account_meta_list_info.data_len() > 0 {
        msg!("Extra Account Meta List already initialized");
        return Ok(());
    }

    // Calculate the required space for the ExtraAccountMetaList using TLV format
    let account_size = ExtraAccountMetaList::size_of(extra_account_metas.len())?;
    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(account_size);

    msg!("Creating PDA with TLV size: {}, lamports: {}", account_size, required_lamports);

    // Create the PDA account
    let create_account_ix = system_instruction::create_account(
        payer_info.key,
        extra_account_meta_list_info.key,
        required_lamports,
        account_size as u64,
        program_id,
    );

    let signer_seeds = &[b"extra-account-metas", mint_info.key.as_ref(), &[bump]];

    invoke_signed(
        &create_account_ix,
        &[payer_info.clone(), extra_account_meta_list_info.clone(), system_program_info.clone()],
        &[signer_seeds],
    )?;

    // Initialize the account data with proper TLV format
    {
        let mut data = extra_account_meta_list_info.try_borrow_mut_data()?;
        ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, extra_account_metas)?;
    }

    msg!("Extra Account Meta List PDA created and initialized successfully with TLV format");
    Ok(())
}

// Update Extra Account Meta List using proper TLV format
fn update_extra_account_meta_list(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    extra_account_metas: &[ExtraAccountMeta],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let extra_account_meta_list_info = next_account_info(accounts_iter)?;
    let mint_info = next_account_info(accounts_iter)?;
    let _authority_info = next_account_info(accounts_iter)?;

    msg!("Updating Extra Account Meta List with TLV format");

    // Verify this is the correct PDA
    let seeds = &[b"extra-account-metas", mint_info.key.as_ref()];
    let (expected_address, _bump) = Pubkey::find_program_address(seeds, program_id);

    if expected_address != *extra_account_meta_list_info.key {
        msg!("Error: Extra Account Meta List address mismatch");
        return Err(ProgramError::InvalidSeeds);
    }

    // Update the account data
    {
        let mut data = extra_account_meta_list_info.try_borrow_mut_data()?;
        ExtraAccountMetaList::update::<ExecuteInstruction>(&mut data, extra_account_metas)?;
    }

    msg!("Extra Account Meta List updated successfully");
    Ok(())
}
