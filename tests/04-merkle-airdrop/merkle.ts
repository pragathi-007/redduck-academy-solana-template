import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { createHash } from "crypto";

export interface AirdropEntry {
  account: PublicKey;
  amount: BN;
}

const sha256 = (chunks: Buffer[]): Buffer => {
  const h = createHash("sha256");
  for (const c of chunks) h.update(c);
  return h.digest();
};

const hashPair = (a: Buffer, b: Buffer): Buffer => {
  return Buffer.compare(a, b) <= 0 ? sha256([a, b]) : sha256([b, a]);
};

export const hashLeaf = (entry: AirdropEntry): Buffer => {
  const amountBuf = entry.amount.toArrayLike(Buffer, "le", 8);
  return sha256([entry.account.toBuffer(), amountBuf]);
};

export class MerkleTree {
  private layers: Buffer[][];

  constructor(entries: AirdropEntry[]) {
    if (entries.length === 0) {
      throw new Error("MerkleTree requires at least one entry");
    }
    const leaves = entries.map(hashLeaf);
    this.layers = [leaves];

    let current = leaves;
    while (current.length > 1) {
      const next: Buffer[] = [];
      for (let i = 0; i < current.length; i += 2) {
        if (i + 1 < current.length) {
          next.push(hashPair(current[i], current[i + 1]));
        } else {
          next.push(current[i]);
        }
      }
      this.layers.push(next);
      current = next;
    }
  }

  get root(): Buffer {
    return this.layers[this.layers.length - 1][0];
  }

  getProof(index: number): Buffer[] {
    if (index < 0 || index >= this.layers[0].length) {
      throw new Error("Leaf index out of bounds");
    }
    const proof: Buffer[] = [];
    let idx = index;
    for (let level = 0; level < this.layers.length - 1; level++) {
      const layer = this.layers[level];
      const siblingIdx = idx % 2 === 0 ? idx + 1 : idx - 1;
      if (siblingIdx < layer.length) {
        proof.push(layer[siblingIdx]);
      }
      idx = Math.floor(idx / 2);
    }
    return proof;
  }
}