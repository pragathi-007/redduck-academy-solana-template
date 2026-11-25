use anchor_lang::prelude::*;

#[error_code]
pub enum MerkleAirdropError {
    #[msg("Invalid Merkle proof")]
    InvalidProof,
    #[msg("Invalid claim amount")]
    InvalidAmount,
    #[msg("Missing or invalid Ed25519 signature verification instruction")]
    InvalidSignatureInstruction,
    #[msg("Invalid signature signer or message")]
    InvalidSignature,
}