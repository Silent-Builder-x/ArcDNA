# ArcDNA: The "Blind" Genomic Matching Protocol 🧬

## 📖 Overview

**ArcDNA** is a privacy-first genomic infrastructure built on **Arcium** and **Solana**. It solves the "Genomic Privacy Paradox": *To find genetic relatives or cure diseases, we must expose our most sensitive biological data to centralized databases.*

ArcDNA implements a **"Blind Bio-Lab"**:

1. Users encrypt their DNA locally into **Secret Shares**.
2. The **Arcium MXE Network** computes similarity (Hamming Distance) on these shares.
3. **No raw DNA sequence is ever revealed**—not to the platform, not to the counterparty, and not to the blockchain.

> *"Compute on the data, without seeing the data."*

## 🚀 Live Demo & Deployment

The protocol logic is verified on the Arcium Devnet v0.8.3.

### 🖥️ Try the Experience

We have built a fully interactive visualization of the MPC process.
[Launch Blind Bio-Lab Demo](https://silent-builder-x.github.io/ArcDNA/)

## 🧠 Core Innovation

### 1. Client-Side Sharding (The "Glass Break" Technique)

Before a genome leaves the user's device, it is mathematically split into `n` shards using Shamir's Secret Sharing (or additive sharing for this circuit).

- **User A's DNA:** `AGCT...` -> `[Shard 1, Shard 2, Shard 3]`
- **Network View:** Random noise.

### 2. Homomorphic Hamming Circuit

The Arcis circuit (`src/lib.rs`) iterates through encrypted genome vectors using constant-time operations.

- **Input:** `Encrypted<User_Vector>`, `Encrypted<Target_Vector>`
- **Logic:** `Mux(User[i] == Target[i], Score + 1, Score)`
- **Output:** `Encrypted<Similarity_Score>`

### 3. Solana Verification Layer

The Anchor program orchestrates the workflow, verifying that the computation was performed by authorized Arcium nodes (via signature verification) before releasing the result event.

## 🛠 Architecture

```
graph LR
    A[User Client] -- 1. Encrypt & Shard --> B(Solana Program)
    B -- 2. Queue Computation --> C{Arcium MXE Cluster}
    C -- 3. MPC Execution --> C
    C -- 4. Callback with Proof --> B
    B -- 5. Emit Match Event --> A

```

## ⚙️ Build & Reproduction

### Prerequisites

- Rust `1.75+`
- Solana CLI `3.1.8+`
- Arcium CLI `0.8.3`

### 1. Build Circuit & Program

```
# Compile the Arcis circuit and Anchor program
arcium build

```

### 2. Deploy to Arcium Devnet

```
# Upload computation definition
arcium deploy --cluster-offset 456 --recovery-set-size 4 --keypair-path ~/.config/solana/id.json -u d

```

## 📄 Technical Specification (v1.1)

- **Engine:** `compute_dna_similarity` (Arcis-MPC)
- **Encryption Scheme:** Linear Secret Sharing (LSS)
- **Privacy Guarantee:** Information-Theoretic Security for inputs.
- **Compliance:** Built following Arcium Standards with verified `/// CHECK:` safety comments.