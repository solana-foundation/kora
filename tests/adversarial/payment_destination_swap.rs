use crate::common::{assertions::RpcErrorAssertions, *};
use jsonrpsee::rpc_params;
use solana_program_pack::Pack;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction::{create_account, transfer as system_transfer};
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_token_interface::{
    instruction::{self as token_ix, AuthorityType},
    state::{Account as TokenAccount, Mint},
};

const AMOUNT: u64 = 2_000_000;

struct Attacker {
    wallet: Keypair,
    account_rent: u64,
}

impl Attacker {
    async fn new(ctx: &TestContext) -> Self {
        let wallet = create_funded_wallet(ctx).await;
        let account_rent = ctx
            .rpc_client()
            .get_minimum_balance_for_rent_exemption(TokenAccount::LEN)
            .await
            .unwrap();
        Self { wallet, account_rent }
    }

    fn create_token_account(&self, address: &Pubkey) -> Instruction {
        create_account(
            &self.wallet.pubkey(),
            address,
            self.account_rent,
            TokenAccount::LEN as u64,
            &spl_token_interface::id(),
        )
    }

    fn initialize(&self, address: &Pubkey, mint: &Pubkey) -> Instruction {
        token_ix::initialize_account3(
            &spl_token_interface::id(),
            address,
            mint,
            &self.wallet.pubkey(),
        )
        .unwrap()
    }

    fn close(&self, address: &Pubkey) -> Instruction {
        token_ix::close_account(
            &spl_token_interface::id(),
            address,
            &self.wallet.pubkey(),
            &self.wallet.pubkey(),
            &[],
        )
        .unwrap()
    }

    /// A token account of `mint` at a keypair address, owned by Kora's fee payer. With
    /// `keep_close_authority` the attacker stays its close authority.
    fn kora_owned_account(
        &self,
        address: &Pubkey,
        mint: &Pubkey,
        keep_close_authority: bool,
    ) -> Vec<Instruction> {
        let mut instructions =
            vec![self.create_token_account(address), self.initialize(address, mint)];
        if keep_close_authority {
            instructions.push(
                token_ix::set_authority(
                    &spl_token_interface::id(),
                    address,
                    Some(&self.wallet.pubkey()),
                    AuthorityType::CloseAccount,
                    &self.wallet.pubkey(),
                    &[],
                )
                .unwrap(),
            );
        }
        instructions.push(
            token_ix::set_authority(
                &spl_token_interface::id(),
                address,
                Some(&FeePayerTestHelper::get_fee_payer_pubkey()),
                AuthorityType::AccountOwner,
                &self.wallet.pubkey(),
                &[],
            )
            .unwrap(),
        );
        instructions
    }

    async fn send(&self, ctx: &TestContext, instructions: &[Instruction], signers: &[&Keypair]) {
        let mut all_signers = vec![&self.wallet];
        all_signers.extend_from_slice(signers);
        let blockhash = ctx.rpc_client().get_latest_blockhash().await.unwrap();
        let transaction = Transaction::new_signed_with_payer(
            instructions,
            Some(&self.wallet.pubkey()),
            &all_signers,
            blockhash,
        );
        ctx.rpc_client().send_and_confirm_transaction(&transaction).await.expect("attacker setup");
    }
}

async fn sign(
    ctx: &TestContext,
    instructions: Vec<Instruction>,
    signers: &[&Keypair],
) -> anyhow::Result<serde_json::Value> {
    let mut builder =
        ctx.transaction_builder().with_fee_payer(FeePayerTestHelper::get_fee_payer_pubkey());
    for signer in signers {
        builder = builder.with_signer(signer);
    }
    for instruction in instructions {
        builder = builder.with_instruction(instruction);
    }
    let transaction = builder.build().await.expect("build transaction");
    ctx.rpc_call("signTransaction", rpc_params![transaction]).await
}

/// A legacy Transfer between a source and a Kora-owned destination that the same transaction
/// closes and recreates with the attacker's mint, while the fee payer also sends SOL to the
/// attacker. Kora used to price it as a payment in the allowlisted mint.
#[tokio::test]
async fn test_transfer_between_recreated_accounts_is_not_a_payment() {
    let ctx = crate::ctx().await;
    let attacker = Attacker::new(&ctx).await;
    let allowed_mint = USDCMintTestHelper::get_test_usdc_mint_pubkey();
    let worthless_mint = Keypair::new();
    let source = Keypair::new();
    let destination = Keypair::new();
    let mint_rent =
        ctx.rpc_client().get_minimum_balance_for_rent_exemption(Mint::LEN).await.unwrap();

    let mut setup = vec![
        create_account(
            &attacker.wallet.pubkey(),
            &worthless_mint.pubkey(),
            mint_rent,
            Mint::LEN as u64,
            &spl_token_interface::id(),
        ),
        token_ix::initialize_mint2(
            &spl_token_interface::id(),
            &worthless_mint.pubkey(),
            &attacker.wallet.pubkey(),
            None,
            6,
        )
        .unwrap(),
        attacker.create_token_account(&source.pubkey()),
        attacker.initialize(&source.pubkey(), &allowed_mint),
    ];
    setup.extend(attacker.kora_owned_account(&destination.pubkey(), &allowed_mint, true));
    attacker.send(&ctx, &setup, &[&worthless_mint, &source, &destination]).await;

    #[allow(deprecated)]
    let attack = vec![
        attacker.close(&source.pubkey()),
        attacker.close(&destination.pubkey()),
        attacker.create_token_account(&source.pubkey()),
        attacker.create_token_account(&destination.pubkey()),
        attacker.initialize(&source.pubkey(), &worthless_mint.pubkey()),
        attacker.initialize(&destination.pubkey(), &worthless_mint.pubkey()),
        token_ix::mint_to(
            &spl_token_interface::id(),
            &worthless_mint.pubkey(),
            &source.pubkey(),
            &attacker.wallet.pubkey(),
            &[],
            AMOUNT,
        )
        .unwrap(),
        token_ix::transfer(
            &spl_token_interface::id(),
            &source.pubkey(),
            &destination.pubkey(),
            &attacker.wallet.pubkey(),
            &[],
            AMOUNT,
        )
        .unwrap(),
        system_transfer(
            &FeePayerTestHelper::get_fee_payer_pubkey(),
            &attacker.wallet.pubkey(),
            1_000_000,
        ),
    ];

    sign(&ctx, attack, &[&attacker.wallet, &source, &destination])
        .await
        .expect_err("Kora signed a transfer of the attacker's own mint as payment")
        .assert_contains_message("Insufficient token payment");
}

/// The destination's mint is checked before the transaction, so whoever can close it could
/// swap it out between signing and landing. Only Kora may be able to close a payment account.
#[tokio::test]
async fn test_payment_counts_only_when_kora_alone_can_close_the_destination() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let attacker = Attacker::new(&ctx).await;
    let mint = setup.usdc_mint.pubkey();
    let sender_account = get_associated_token_address(&setup.sender_keypair.pubkey(), &mint);

    for (keep_close_authority, accepted) in [(true, false), (false, true)] {
        let destination = Keypair::new();
        attacker
            .send(
                &ctx,
                &attacker.kora_owned_account(&destination.pubkey(), &mint, keep_close_authority),
                &[&destination],
            )
            .await;

        let payment = token_ix::transfer_checked(
            &spl_token_interface::id(),
            &sender_account,
            &mint,
            &destination.pubkey(),
            &setup.sender_keypair.pubkey(),
            &[],
            AMOUNT,
            6,
        )
        .unwrap();
        let result = sign(&ctx, vec![payment], &[&setup.sender_keypair]).await;

        if accepted {
            result.expect("payment into a Kora-owned account only Kora can close");
        } else {
            result
                .expect_err("Kora counted a payment into an account the attacker can close")
                .assert_contains_message("Insufficient token payment");
        }
    }
}

/// With `allow_close_account` on, Kora still refuses to close an account it owns: the caller
/// holds that address's keypair and could recreate it as a payment destination with another mint.
#[tokio::test]
async fn test_fee_payer_does_not_close_a_token_account_it_owns() {
    let ctx = crate::ctx().await;
    let setup = TestAccountSetup::new(ctx.rpc_client().clone()).await;
    let attacker = Attacker::new(&ctx).await;
    let fee_payer = FeePayerTestHelper::get_fee_payer_pubkey();
    let destination = Keypair::new();
    attacker
        .send(
            &ctx,
            &attacker.kora_owned_account(&destination.pubkey(), &setup.usdc_mint.pubkey(), false),
            &[&destination],
        )
        .await;

    let close = token_ix::close_account(
        &spl_token_interface::id(),
        &destination.pubkey(),
        &fee_payer,
        &fee_payer,
        &[],
    )
    .unwrap();

    sign(&ctx, vec![close], &[])
        .await
        .expect_err("Kora closed a token account it owns")
        .assert_contains_message("owned by Kora");
}
