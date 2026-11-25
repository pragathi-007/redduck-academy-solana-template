use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Claim {
    pub claimant: Pubkey,
    pub amount: u64,
    pub bump: u8,
}