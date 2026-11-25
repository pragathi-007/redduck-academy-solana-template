use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{CONFIG_SEED, VAULT_SEED};
use crate::state::Config;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + Config::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump,
    )]
    pub config: Account<'info, Config>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        seeds = [VAULT_SEED],
        bump,
        token::mint = mint,
        token::authority = config,
    )]
    pub vault: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn initialize(
    ctx: Context<Initialize>,
    merkle_root: [u8; 32],
    signer: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.mint = ctx.accounts.mint.key();
    config.merkle_root = merkle_root;
    config.signer = signer;
    config.bump = ctx.bumps.config;
    config.vault_bump = ctx.bumps.vault;
    Ok(())
}
