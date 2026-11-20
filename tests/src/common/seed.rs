#![cfg(test)]

use anyhow::Result;
use solana_program_pack::Pack;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, program_option::COption, pubkey::Pubkey, rent::Rent,
    signature::Signer,
};
use spl_associated_token_account_interface::address::get_associated_token_address_with_program_id;
use spl_token_2022_interface::{
    extension::{
        immutable_owner::ImmutableOwner,
        transfer_fee::{TransferFee, TransferFeeAmount, TransferFeeConfig},
        BaseStateWithExtensionsMut, ExtensionType, StateWithExtensionsMut,
    },
    state::{
        Account as Token2022Account, AccountState as Token2022AccountState, Mint as Token2022Mint,
    },
};
use surfpool_sdk::{cheatcodes::Cheatcodes, Surfnet};

use crate::common::{
    setup::{ts_auth_wallet, ts_free_wallet},
    FeePayerPolicyMintTestHelper, FeePayerTestHelper, RecipientTestHelper, SenderTestHelper,
    USDCMint2022TestHelper, USDCMintTestHelper,
};

const SOL_FUNDING: u64 = 10 * LAMPORTS_PER_SOL;
const TRANSFER_FEE_BASIS_POINTS: u16 = 100;
const MAXIMUM_TRANSFER_FEE: u64 = 1_000_000;

struct Holder {
    owner: Pubkey,
    amount: u64,
}

/// Writes every account the test phases expect to already exist on chain.
pub fn seed_accounts(surfnet: &Surfnet) -> Result<()> {
    let cheats = surfnet.cheatcodes();

    let sender_pubkey = SenderTestHelper::get_test_sender_keypair().pubkey();
    let fee_payer_pubkey = FeePayerTestHelper::get_fee_payer_pubkey();
    let recipient_pubkey = RecipientTestHelper::get_recipient_pubkey();

    let decimals = USDCMintTestHelper::get_test_usdc_mint_decimals();
    let mint_amount = 1_000_000 * 10_u64.pow(decimals as u32);

    for owner in
        [sender_pubkey, recipient_pubkey, fee_payer_pubkey, ts_auth_wallet(), ts_free_wallet()]
    {
        cheats.fund_sol(&owner, SOL_FUNDING)?;
    }

    let usdc_mint_pubkey = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let usdc_holders = [
        Holder { owner: sender_pubkey, amount: mint_amount },
        Holder { owner: recipient_pubkey, amount: 0 },
        Holder { owner: fee_payer_pubkey, amount: 0 },
        Holder { owner: ts_auth_wallet(), amount: mint_amount },
        Holder { owner: ts_free_wallet(), amount: mint_amount },
    ];
    write_spl_mint(&cheats, &usdc_mint_pubkey, &sender_pubkey, decimals, &usdc_holders)?;

    let usdc_mint_2022_pubkey = USDCMint2022TestHelper::get_test_usdc_mint_2022_pubkey();
    let usdc_2022_holders = [
        Holder { owner: sender_pubkey, amount: mint_amount },
        Holder { owner: recipient_pubkey, amount: 0 },
        Holder { owner: fee_payer_pubkey, amount: 0 },
    ];
    write_token_2022_mint(
        &cheats,
        &usdc_mint_2022_pubkey,
        &sender_pubkey,
        decimals,
        true,
        &usdc_2022_holders,
    )?;

    let fee_payer_policy_mint_pubkey =
        FeePayerPolicyMintTestHelper::get_fee_payer_policy_mint_pubkey();
    let policy_holders = [
        Holder { owner: sender_pubkey, amount: mint_amount },
        Holder { owner: recipient_pubkey, amount: 0 },
        Holder { owner: fee_payer_pubkey, amount: 0 },
    ];
    write_spl_mint(
        &cheats,
        &fee_payer_policy_mint_pubkey,
        &fee_payer_pubkey,
        decimals,
        &policy_holders,
    )?;

    let fee_payer_policy_mint_2022_pubkey =
        FeePayerPolicyMintTestHelper::get_fee_payer_policy_mint_2022_pubkey();
    write_token_2022_mint(
        &cheats,
        &fee_payer_policy_mint_2022_pubkey,
        &fee_payer_pubkey,
        decimals,
        false,
        &policy_holders,
    )?;

    Ok(())
}

fn write_spl_mint(
    cheats: &Cheatcodes,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
    holders: &[Holder],
) -> Result<()> {
    let state = spl_token_interface::state::Mint {
        mint_authority: COption::Some(*authority),
        supply: holders.iter().map(|holder| holder.amount).sum(),
        decimals,
        is_initialized: true,
        freeze_authority: COption::Some(*authority),
    };
    let mut data = vec![0u8; spl_token_interface::state::Mint::LEN];
    state.pack_into_slice(&mut data);
    write_account(cheats, mint, &data, &spl_token_interface::id())?;

    holders
        .iter()
        .map(|holder| {
            let account = spl_token_interface::state::Account {
                mint: *mint,
                owner: holder.owner,
                amount: holder.amount,
                state: spl_token_interface::state::AccountState::Initialized,
                ..Default::default()
            };
            let mut data = vec![0u8; spl_token_interface::state::Account::LEN];
            account.pack_into_slice(&mut data);

            let address = get_associated_token_address_with_program_id(
                &holder.owner,
                mint,
                &spl_token_interface::id(),
            );
            write_account(cheats, &address, &data, &spl_token_interface::id())?;
            Ok(())
        })
        .collect()
}

fn write_token_2022_mint(
    cheats: &Cheatcodes,
    mint: &Pubkey,
    authority: &Pubkey,
    decimals: u8,
    transfer_fee: bool,
    holders: &[Holder],
) -> Result<()> {
    let extensions: &[ExtensionType] =
        if transfer_fee { &[ExtensionType::TransferFeeConfig] } else { &[] };
    let mut data =
        vec![0u8; ExtensionType::try_calculate_account_len::<Token2022Mint>(extensions)?];
    {
        let mut state = StateWithExtensionsMut::<Token2022Mint>::unpack_uninitialized(&mut data)?;
        if transfer_fee {
            let fee = TransferFee {
                epoch: 0.into(),
                maximum_fee: MAXIMUM_TRANSFER_FEE.into(),
                transfer_fee_basis_points: TRANSFER_FEE_BASIS_POINTS.into(),
            };
            let config = state.init_extension::<TransferFeeConfig>(true)?;
            config.transfer_fee_config_authority = (*authority).into();
            config.withdraw_withheld_authority = (*authority).into();
            config.withheld_amount = 0.into();
            config.older_transfer_fee = fee;
            config.newer_transfer_fee = fee;
        }
        state.base = Token2022Mint {
            mint_authority: COption::Some(*authority),
            supply: holders.iter().map(|holder| holder.amount).sum(),
            decimals,
            is_initialized: true,
            freeze_authority: COption::Some(*authority),
        };
        state.pack_base();
        state.init_account_type()?;
    }
    write_account(cheats, mint, &data, &spl_token_2022_interface::id())?;

    holders
        .iter()
        .map(|holder| {
            let address = get_associated_token_address_with_program_id(
                &holder.owner,
                mint,
                &spl_token_2022_interface::id(),
            );
            write_account(
                cheats,
                &address,
                &token_2022_account_data(mint, holder, transfer_fee)?,
                &spl_token_2022_interface::id(),
            )?;
            Ok(())
        })
        .collect()
}

/// The associated token account program always adds `ImmutableOwner`, and adds
/// `TransferFeeAmount` for a mint that charges a transfer fee.
fn token_2022_account_data(mint: &Pubkey, holder: &Holder, transfer_fee: bool) -> Result<Vec<u8>> {
    let mut extensions = vec![ExtensionType::ImmutableOwner];
    if transfer_fee {
        extensions.push(ExtensionType::TransferFeeAmount);
    }

    let mut data =
        vec![0u8; ExtensionType::try_calculate_account_len::<Token2022Account>(&extensions)?];
    let mut state = StateWithExtensionsMut::<Token2022Account>::unpack_uninitialized(&mut data)?;
    state.init_extension::<ImmutableOwner>(true)?;
    if transfer_fee {
        state.init_extension::<TransferFeeAmount>(true)?.withheld_amount = 0.into();
    }
    state.base = Token2022Account {
        mint: *mint,
        owner: holder.owner,
        amount: holder.amount,
        state: Token2022AccountState::Initialized,
        ..Default::default()
    };
    state.pack_base();
    state.init_account_type()?;

    Ok(data)
}

fn write_account(cheats: &Cheatcodes, address: &Pubkey, data: &[u8], owner: &Pubkey) -> Result<()> {
    cheats.set_account(address, Rent::default().minimum_balance(data.len()), data, owner)?;
    Ok(())
}
