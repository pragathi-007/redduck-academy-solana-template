use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    ed25519_program,
    sysvar::instructions::{load_current_index_checked, load_instruction_at_checked},
};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{CLAIM_SEED, CONFIG_SEED, VAULT_SEED};
use crate::error::MerkleAirdropError;
use crate::state::{Claim, Config};

#[derive(Accounts)]
pub struct ClaimWithSignature<'info> {
    #[account(mut)]
    pub claimant: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = mint,
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = claimant,
        space = 8 + Claim::INIT_SPACE,
        seeds = [CLAIM_SEED, claimant.key().as_ref()],
        bump,
    )]
    pub claim_status: Account<'info, Claim>,

    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump = config.vault_bump,
        token::mint = mint,
        token::authority = config,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = claimant,
        associated_token::mint = mint,
        associated_token::authority = claimant,
    )]
    pub claimant_token_account: Account<'info, TokenAccount>,

    /// CHECK: Verified via sysvar instructions address
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn claim_with_signature(ctx: Context<ClaimWithSignature>, amount: u64) -> Result<()> {
    require!(amount > 0, MerkleAirdropError::InvalidAmount);

    let config = &ctx.accounts.config;
    let ix_sysvar = &ctx.accounts.instructions.to_account_info();
    let current_index = load_current_index_checked(ix_sysvar)
        .map_err(|_| MerkleAirdropError::InvalidSignatureInstruction)?;
    require!(
        current_index >= 1,
        MerkleAirdropError::InvalidSignatureInstruction
    );

    let ed25519_ix = load_instruction_at_checked((current_index - 1) as usize, ix_sysvar)
        .map_err(|_| MerkleAirdropError::InvalidSignatureInstruction)?;
    require_keys_eq!(
        ed25519_ix.program_id,
        ed25519_program::ID,
        MerkleAirdropError::InvalidSignatureInstruction
    );

    let data = &ed25519_ix.data;
    require!(
        data.len() >= 16 && data[0] == 1,
        MerkleAirdropError::InvalidSignatureInstruction
    );

    let sig_offset = u16::from_le_bytes([data[2], data[3]]) as usize;
    let sig_ix_idx = u16::from_le_bytes([data[4], data[5]]);
    let pk_offset = u16::from_le_bytes([data[6], data[7]]) as usize;
    let pk_ix_idx = u16::from_le_bytes([data[8], data[9]]);
    let msg_offset = u16::from_le_bytes([data[10], data[11]]) as usize;
    let msg_size = u16::from_le_bytes([data[12], data[13]]) as usize;
    let msg_ix_idx = u16::from_le_bytes([data[14], data[15]]);

    require!(
        sig_ix_idx == u16::MAX && pk_ix_idx == u16::MAX && msg_ix_idx == u16::MAX,
        MerkleAirdropError::InvalidSignatureInstruction
    );
    require!(
        sig_offset.saturating_add(64) <= data.len()
            && pk_offset.saturating_add(32) <= data.len()
            && msg_offset.saturating_add(msg_size) <= data.len(),
        MerkleAirdropError::InvalidSignatureInstruction
    );

    let signer_bytes: [u8; 32] = data[pk_offset..pk_offset + 32]
        .try_into()
        .map_err(|_| MerkleAirdropError::InvalidSignature)?;
    let recovered_signer = Pubkey::new_from_array(signer_bytes);
    require_keys_eq!(
        recovered_signer,
        config.signer,
        MerkleAirdropError::InvalidSignature
    );

    let mut expected_msg = Vec::with_capacity(32 + 32 + 8);
    expected_msg.extend_from_slice(crate::ID.as_ref());
    expected_msg.extend_from_slice(ctx.accounts.claimant.key().as_ref());
    expected_msg.extend_from_slice(&amount.to_le_bytes());

    let actual_msg = &data[msg_offset..msg_offset + msg_size];
    require!(
        actual_msg == expected_msg.as_slice(),
        MerkleAirdropError::InvalidSignature
    );

    let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, &[config.bump]]];
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.claimant_token_account.to_account_info(),
        authority: ctx.accounts.config.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;

    let claim_status = &mut ctx.accounts.claim_status;
    claim_status.claimant = ctx.accounts.claimant.key();
    claim_status.amount = amount;
    claim_status.bump = ctx.bumps.claim_status;

    Ok(())
}
