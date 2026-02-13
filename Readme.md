# ArcDNA: Confidential Genomic Matching Protocol via MPC

## 🧬 Overview

**ArcDNA** is a privacy-preserving infrastructure for genomic similarity analysis built on **Arcium** and **Solana**.

Genomic data is the most sensitive information a human possesses. Traditional DNA matching services require users to upload their raw genetic sequences to centralized databases, creating permanent privacy risks. **ArcDNA** utilizes **Secure Multi-Party Computation (MPC)** to compute genetic similarity entirely on encrypted data. Platforms and nodes never see the raw sequences—only the authorized matching results are revealed.

## 🚀 Live Deployment Status (Devnet)

The protocol is fully operational and verified on the Arcium Devnet.

- **MXE Address:** `H6ri1pKhvGiqapvabZwtThmWNCKcCjw3sw1w17iN8kmy`
- **MXE Program ID:** `CamjN5ifgeAB7mLrpW59rfTHte6eVSRRu6E3K1vQsXqb`
- **Computation Definition:** `8j22c52iXewM516KwjTwpZYDmmkoef6m8coDW14MjVth`
- **Status:** `Active`

## 🧠 Core Innovation: The "Blind" Bio-Lab

ArcDNA implements a secure genomic primitive based on the **Hamming Distance** algorithm:

- **Shielded Sequences:** DNA feature vectors are encrypted locally using ephemeral session keys. The Solana ledger only receives ciphertext shards.
- **MPC Comparison:** The Arcis circuit iterates through encrypted genome fragments using constant-time multiplexers (`if-else` mux) to calculate similarity without leaking intermediate data.
- **Verifiable Proofs:** Final match results are computed by the Arcium Multi-Party Execution (MXE) Network and committed to the Solana ledger via verified callbacks.

## 🛠 Build & Implementation

```
# Compile Arcis circuits and Solana program
arcium build

# Deploy to Cluster
arcium deploy --cluster-offset 456 --recovery-set-size 4 --keypair-path ~/.config/solana/id.json -u d

```

## 📄 Technical Specification (v0.8.3)

- **Engine:** `compute_dna_similarity` (Arcis-MPC Circuit)
- **Security:** Supported by Arcium MPC threshold signatures.
- **Protocol Version:** `v0.8.3`

## ⚙️ Development Environment

- **Arcium CLI:** `0.8.3`
- **Anchor Framework:** `0.30.1`
- **Cluster:** Arcium Devnet (`-u d`)
- **Compliance:** Built following Arcium Standards with verified `/// CHECK:` safety comments.