import { BN } from "@coral-xyz/anchor";
import {
  Ed25519Program,
  Keypair,
  PublicKey,
  TransactionInstruction,
} from "@solana/web3.js";

export const buildClaimSignatureIx = (
  signer: Keypair,
  claimant: PublicKey,
  amount: BN,
  programId: PublicKey,
): TransactionInstruction => {
  const message = Buffer.concat([
    programId.toBuffer(),
    claimant.toBuffer(),
    amount.toArrayLike(Buffer, "le", 8),
  ]);

  return Ed25519Program.createInstructionWithPrivateKey({
    privateKey: signer.secretKey,
    message,
  });
};