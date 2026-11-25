use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{CLAIM_SEED, CONFIG_SEED, VAULT_SEED};
use crate::error::MerkleAirdropError;
use crate::state::{Claim, Config};

#[derive(Accounts)]
pub struct ClaimMerkle<'info> {
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

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn claim(ctx: Context<ClaimMerkle>, amount: u64, proof: Vec<[u8; 32]>) -> Result<()> {
    let mut node = hashv(&[
        ctx.accounts.claimant.key().as_ref(),
        &amount.to_le_bytes(),
    ])
    .to_bytes();

    for sibling in proof.iter() {
        node = if node <= *sibling {
            hashv(&[&node, sibling]).to_bytes()
        } else {
            hashv(&[sibling, &node]).to_bytes()
        };
    }

    require!(
        node == ctx.accounts.config.merkle_root,
        MerkleAirdropError::InvalidProof
    );

    let config = &ctx.accounts.config;
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
