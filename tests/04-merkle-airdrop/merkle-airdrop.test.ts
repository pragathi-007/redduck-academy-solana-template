import { PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  AccountLayout,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { assert } from "chai";

import { sendIxs } from "../helpers";
import { setupAirdrop, MerkleFixture } from "./fixture";
import { buildClaimSignatureIx } from "./signatures";

describe("04-merkle-airdrop", () => {
  let env: MerkleFixture;

  const claimPda = (claimant: PublicKey): PublicKey => {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("claim"), claimant.toBuffer()],
      env.program.programId,
    )[0];
  };

  before(async () => {
    env = await setupAirdrop();
  });

  it("claims an allocation with a valid Merkle proof", async () => {
    const { context, program, mint, vault, entries, tree, claimants } = env;

    const claimant = claimants[0];
    const amount = entries[0].amount;
    const proof = tree.getProof(0).map((node) => [...node]);
    const claimantAta = getAssociatedTokenAddressSync(mint, claimant.publicKey);

    const ix = await program.methods
      .claim(amount, proof)
      .accountsPartial({
        claimStatus: claimPda(claimant.publicKey),
        claimant: claimant.publicKey,
        claimantTokenAccount: claimantAta,
        mint,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
    await sendIxs(context, [claimant], ix);

    const acc = await context.banksClient.getAccount(claimantAta);
    const balance = AccountLayout.decode(acc!.data).amount;
    assert.equal(balance.toString(), amount.toString());
  });

  it("rejects a second Merkle claim by the same claimant", async () => {
    const { context, program, mint, vault, entries, tree, claimants } = env;

    const claimant = claimants[0];
    const amount = entries[0].amount;
    const proof = tree.getProof(0).map((node) => [...node]);
    const claimantAta = getAssociatedTokenAddressSync(mint, claimant.publicKey);

    const ix = await program.methods
      .claim(amount, proof)
      .accountsPartial({
        claimStatus: claimPda(claimant.publicKey),
        claimant: claimant.publicKey,
        claimantTokenAccount: claimantAta,
        mint,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    let failed = false;
    try {
      await sendIxs(context, [claimant], ix);
    } catch {
      failed = true;
    }
    assert.isTrue(failed, "duplicate claim should fail");
  });

  it("rejects an invalid Merkle proof", async () => {
    const { context, program, mint, vault, entries, tree, claimants } = env;

    const claimant = claimants[1];
    const amount = entries[1].amount;
    // Wrong proof (belongs to index 2 instead of index 1)
    const badProof = tree.getProof(2).map((node) => [...node]);
    const claimantAta = getAssociatedTokenAddressSync(mint, claimant.publicKey);

    const ix = await program.methods
      .claim(amount, badProof)
      .accountsPartial({
        claimStatus: claimPda(claimant.publicKey),
        claimant: claimant.publicKey,
        claimantTokenAccount: claimantAta,
        mint,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    let failed = false;
    try {
      await sendIxs(context, [claimant], ix);
    } catch {
      failed = true;
    }
    assert.isTrue(failed, "invalid proof should fail");
  });

  it("claims an allocation with a valid trusted signature", async () => {
    const { context, program, mint, vault, entries, signer, claimants } = env;

    const claimant = claimants[1];
    const amount = entries[1].amount;
    const claimantAta = getAssociatedTokenAddressSync(mint, claimant.publicKey);

    const ed25519Ix = buildClaimSignatureIx(
      signer,
      claimant.publicKey,
      amount,
      program.programId,
    );

    const claimIx = await program.methods
      .claimWithSignature(amount)
      .accountsPartial({
        claimStatus: claimPda(claimant.publicKey),
        claimant: claimant.publicKey,
        claimantTokenAccount: claimantAta,
        mint,
        vault,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    await sendIxs(context, [claimant], ed25519Ix, claimIx);

    const acc = await context.banksClient.getAccount(claimantAta);
    const balance = AccountLayout.decode(acc!.data).amount;
    assert.equal(balance.toString(), amount.toString());
  });

  it("rejects claiming again via Merkle after claiming with signature", async () => {
    const { context, program, mint, vault, entries, tree, claimants } = env;

    const claimant = claimants[1];
    const amount = entries[1].amount;
    const proof = tree.getProof(1).map((node) => [...node]);
    const claimantAta = getAssociatedTokenAddressSync(mint, claimant.publicKey);

    const ix = await program.methods
      .claim(amount, proof)
      .accountsPartial({
        claimStatus: claimPda(claimant.publicKey),
        claimant: claimant.publicKey,
        claimantTokenAccount: claimantAta,
        mint,
        vault,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    let failed = false;
    try {
      await sendIxs(context, [claimant], ix);
    } catch {
      failed = true;
    }
    assert.isTrue(failed, "cross-path duplicate claim should fail");
  });

  it("rejects a signature meant for another recipient", async () => {
    const { context, program, mint, vault, entries, signer, claimants } = env;

    const intendedRecipient = claimants[2];
    const attacker = claimants[3];
    const amount = entries[3].amount;
    const attackerAta = getAssociatedTokenAddressSync(mint, attacker.publicKey);

    const ed25519Ix = buildClaimSignatureIx(
      signer,
      intendedRecipient.publicKey,
      amount,
      program.programId,
    );

    const claimIx = await program.methods
      .claimWithSignature(amount)
      .accountsPartial({
        claimStatus: claimPda(attacker.publicKey),
        claimant: attacker.publicKey,
        claimantTokenAccount: attackerAta,
        mint,
        vault,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    let failed = false;
    try {
      await sendIxs(context, [attacker], ed25519Ix, claimIx);
    } catch {
      failed = true;
    }
    assert.isTrue(failed, "signature for another recipient should fail");
  });
});