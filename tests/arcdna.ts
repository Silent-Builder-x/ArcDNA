import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Arcdna } from "../target/types/arcdna";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getArciumEnv,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgramId,
  getArciumProgram,
  uploadCircuit,
  RescueCipher,
  deserializeLE,
  getMXEPublicKey,
  getMXEAccAddress,
  getMempoolAccAddress,
  getCompDefAccAddress,
  getExecutingPoolAccAddress,
  getComputationAccAddress,
  getClusterAccAddress,
  getLookupTableAddress,
  x25519,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("ArcDNA: Confidential Genomic Matching", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  
  const program = anchor.workspace.Arcdna as Program<Arcdna>;
  const arciumProgram = getArciumProgram(provider as anchor.AnchorProvider);

  // Helper to await specific events
  const awaitEvent = async (eventName: string): Promise<any> => {
    return new Promise((resolve) => {
      const listener = program.addEventListener(eventName, (event, slot) => {
        program.removeEventListener(listener);
        resolve(event);
      });
    });
  };

  const arciumEnv = getArciumEnv();

  // Test Data
  const userDna = [100n, 200n, 300n, 400n];
  const targetDna = [100n, 200n, 300n, 999n];

  it("Performs End-to-End Encrypted Match", async () => {
    // 1. Initialize Computation Definition
    // Note: In a real deploy, this is done once. We attempt it here for the test flow.
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);
    try {
        console.log("Initializing Computation Definition...");
        await initDnaCompDef(program, owner);
    } catch (e) {
        console.log("Initialization skipped (likely already initialized)");
    }

    // 2. Setup Encryption Keys (Client Side)
    const ephemeralPrivKey = x25519.utils.randomSecretKey();
    const ephemeralPubKey = x25519.getPublicKey(ephemeralPrivKey);

    const mxePubKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );
    console.log("MXE Public Key:", Buffer.from(mxePubKey).toString("hex"));

    const sharedSecret = x25519.getSharedSecret(ephemeralPrivKey, mxePubKey);
    const cipher = new RescueCipher(sharedSecret);

    // 3. Encrypt Inputs
    // We encrypt all 8 inputs (4 user + 4 target)
    console.log("Encrypting 8 DNA segments...");
    const combinedInputs = [...userDna, ...targetDna];
    const nonce = randomBytes(16);
    
    // Cipher.encrypt takes array of BigInts
    const encryptedOutputs = cipher.encrypt(combinedInputs, nonce);
    
    // Split back into user and target arrays (32 bytes per ciphertext)
    const userEncrypted = encryptedOutputs.slice(0, 4).map(u => Array.from(u));
    const targetEncrypted = encryptedOutputs.slice(4, 8).map(u => Array.from(u));

    // 4. Send Transaction
    const computationOffset = new anchor.BN(randomBytes(8), "hex");
    const nonceBN = new anchor.BN(deserializeLE(nonce).toString());

    console.log("Sending 'request_genomic_match' transaction...");
    
    // Start listening for event
    const eventPromise = awaitEvent("DnaMatchEvent");

    const tx = await program.methods
      .requestGenomicMatch(
        computationOffset,
        userEncrypted,
        targetEncrypted,
        Array.from(ephemeralPubKey),
        nonceBN
      )
      .accounts({
        // Standard Arcium Accounts
        mxeAccount: getMXEAccAddress(program.programId),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            computationOffset
        ),
        compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("compute_dna_similarity")).readUInt32LE()
        ),
        // System
        payer: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        arciumProgram: arciumProgram.programId, 
      } as any) // Cast to any to avoid strict type checks on complex account struct
      .rpc({ skipPreflight: true, commitment: "confirmed" });
    
    console.log("Transaction Signature:", tx);

    // 5. Wait for Off-Chain MPC Computation
    console.log("Waiting for Arcium MPC Execution...");
    await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        computationOffset,
        program.programId,
        "confirmed"
    );

    // 6. Decrypt Results
    console.log("Decrypting results...");
    const event: any = await eventPromise;

    // Structure of event: { encryptedScore: [], encryptedIsRelative: [], nonce: [] }
    const resultCiphertexts = [
        new Uint8Array(event.encryptedScore),
        new Uint8Array(event.encryptedIsRelative)
    ];
    const resultNonce = new Uint8Array(event.nonce);

    const decryptedResults = cipher.decrypt(resultCiphertexts, resultNonce);

    const score = Number(decryptedResults[0]);
    const isRelative = Number(decryptedResults[1]);

    console.log(`\n=== DNA MATCH RESULTS ===`);
    console.log(`Similarity Score: ${score} / 4`);
    console.log(`Is Relative?    : ${isRelative === 1 ? "YES" : "NO"}`);
    console.log(`=========================\n`);

    expect(score).to.equal(3);
    expect(isRelative).to.equal(1);
  });

  // --- Helper Functions ---

  async function initDnaCompDef(
    program: Program<Arcdna>,
    owner: anchor.web3.Keypair,
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount",
    );
    const offset = getCompDefAccOffset("compute_dna_similarity");

    // Note: Since we updated the program to use OffChainCircuitSource, 
    // the circuit file doesn't necessarily need to be uploaded to Solana via Buffer accounts.
    // However, we still need to initialize the CompDef account.

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId(),
    )[0];

    const mxeAccount = getMXEAccAddress(program.programId);
    const mxeAcc = await arciumProgram.account.mxeAccount.fetch(mxeAccount);
    const lutAddress = getLookupTableAddress(program.programId, mxeAcc.lutOffsetSlot);

    console.log("Calling init_dna_config...");
    const sig = await program.methods
      .initDnaConfig()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount,
        addressLookupTable: lutAddress,
        lutProgram: new PublicKey("AddressLookupTab1e1111111111111111111111111"),
      } as any)
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
      
    return sig;
  }
});

async function getMXEPublicKeyWithRetry(
  provider: anchor.AnchorProvider,
  programId: PublicKey,
  maxRetries: number = 20,
  retryDelayMs: number = 500,
): Promise<Uint8Array> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const mxePublicKey = await getMXEPublicKey(provider, programId);
      if (mxePublicKey) {
        return mxePublicKey;
      }
    } catch (error) {
       // ignore
    }
    await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
  }
  throw new Error("Failed to fetch MXE public key");
}

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString())),
  );
}
