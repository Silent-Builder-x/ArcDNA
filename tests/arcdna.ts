import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Arcdna } from "../target/types/arcdna";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgramId,
  getArciumProgram,
  uploadCircuit,
  RescueCipher,
  deserializeLE,
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

describe("ArcDNA Genomic Matching - Devnet Cloud", () => {
  // --- 1. 核心环境配置 ---
  const useDevnet = true; 
  const clusterOffset = 456; // Devnet 推荐 Offset
  const rpcUrl = "https://api.devnet.solana.com"; 

  let provider: anchor.AnchorProvider;
  let program: Program<Arcdna>;
  let arciumProgram: any;
  let clusterAccount: PublicKey;
  const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

  before(async () => {
    if (useDevnet) {
      console.log("🌐 Connecting to Devnet Cluster...");
      const connection = new anchor.web3.Connection(rpcUrl, "confirmed");
      const wallet = new anchor.Wallet(owner);
      provider = new anchor.AnchorProvider(connection, wallet, { commitment: "confirmed" });
      const idl = JSON.parse(fs.readFileSync("./target/idl/arcdna.json", "utf8"));
      program = new anchor.Program(idl, provider) as Program<Arcdna>;
    } else {
      anchor.setProvider(anchor.AnchorProvider.env());
      provider = anchor.getProvider() as anchor.AnchorProvider;
      program = anchor.workspace.Arcdna as Program<Arcdna>;
    }
    arciumProgram = getArciumProgram(provider);
    clusterAccount = getClusterAccAddress(clusterOffset);
  });

  // --- 2. 官方风格的事件监听助手 ---
  type Event = anchor.IdlEvents<Arcdna>;
  const awaitEvent = async <E extends keyof Event>(eventName: E): Promise<Event[E]> => {
    let listenerId: number;
    const event = await new Promise<Event[E]>((res) => {
      listenerId = program.addEventListener(eventName, (e) => res(e));
    });
    await program.removeEventListener(listenerId);
    return event;
  };

  it("Performs an end-to-end DNA match on Arcium Devnet", async () => {
    // A. 初始化计算定义 (带“已存在”检查)
    console.log(`🚀 Initializing with Cluster Offset: ${clusterOffset}`);
    await safeInitDnaCompDef(program, owner);

    // B. 获取 MXE 公钥 (带手动 Buffer 切片 Fallback)
    const mxePublicKey = await getMXEPublicKeyRobust(provider, program.programId);
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    // C. 注册 Profile
    const targetData = Array.from({ length: 8 }, (_, i) => BigInt((i + 1) * 100));
    const targetNonce = randomBytes(16);
    const targetCiphertext = cipher.encrypt(targetData, targetNonce);

    const [profilePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("profile"), owner.publicKey.toBuffer()],
      program.programId
    );

    console.log("📝 Registering target DNA profile...");
    await program.methods
      .registerProfile(targetCiphertext.map(c => Array.from(c)))
      .rpc();

    // D. 发起比对 (构造 6 个匹配点)
    const userData = [...targetData.slice(0, 6), BigInt(999), BigInt(999)];
    const userNonce = randomBytes(16);
    const userCiphertext = cipher.encrypt(userData, userNonce);

    const matchEventPromise = awaitEvent("dnaMatchEvent");
    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("🧬 Queueing encrypted match computation...");
    await program.methods
      .computeMatchWithProfile(
        computationOffset,
        userCiphertext.map(c => Array.from(c)),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(userNonce).toString()),
      )
      .accountsPartial({
        targetProfile: profilePDA,
        computation: {
            computationAccount: getComputationAccAddress(clusterOffset, computationOffset),
            clusterAccount,
            mxeAccount: getMXEAccAddress(program.programId),
            mempoolAccount: getMempoolAccAddress(clusterOffset),
            executingPool: getExecutingPoolAccAddress(clusterOffset),
            compDefAccount: getCompDefAccAddress(
              program.programId,
              Buffer.from(getCompDefAccOffset("compute_dna_similarity")).readUInt32LE(),
            ),
        }
      })
      .rpc({ commitment: "confirmed" });

    // E. 等待并验证结果
    console.log("⏳ Waiting for MPC nodes to finalize...");
    await awaitComputationFinalization(provider, computationOffset, program.programId, "confirmed");

    const event = await matchEventPromise;
    const [score, isRelative] = cipher.decrypt([event.score, event.is_relative], event.nonce);
    
    console.log(`✅ Success! Score: ${score}/8, Relative: ${isRelative === BigInt(1)}`);
    expect(score).to.equal(BigInt(6));
    expect(isRelative).to.equal(BigInt(1));
  });

  // --- 3. 稳健的初始化函数 ---
  async function safeInitDnaCompDef(program: Program<Arcdna>, owner: anchor.web3.Keypair) {
    const compDefOffset = getCompDefAccOffset("compute_dna_similarity");
    const [compDefPDA] = PublicKey.findProgramAddressSync(
      [getArciumAccountBaseSeed("ComputationDefinitionAccount"), program.programId.toBuffer(), compDefOffset],
      getArciumProgramId(),
    );

    const mxeAccount = getMXEAccAddress(program.programId);
    
    // Fallback: 如果 IDL 过大导致 fetch 失败，使用默认偏移
    let lutOffset = 0;
    try {
        const mxeAcc = await arciumProgram.account.mxeAccount.fetch(mxeAccount);
        lutOffset = mxeAcc.lutOffsetSlot;
    } catch(e) { console.warn("⚠️ Using fallback LUT offset 0"); }

    const lutAddress = getLookupTableAddress(program.programId, lutOffset);
    const info = await provider.connection.getAccountInfo(compDefPDA);

    if (!info) {
        console.log("Initializing Comp Def on-chain...");
        await program.methods.initDnaCompDef().accounts({
          compDefAccount: compDefPDA, payer: owner.publicKey, mxeAccount, addressLookupTable: lutAddress,
        }).signers([owner]).rpc({ commitment: "confirmed" });
    } else {
        console.log("✅ Comp Def already exists.");
    }

    console.log("Uploading circuit...");
    const rawCircuit = fs.readFileSync("build/compute_dna_similarity.arcis");
    await uploadCircuit(provider, "compute_dna_similarity", program.programId, rawCircuit, true);
  }
});

// --- 4. 终极工具函数：绕过 SDK 解析直接读取数据 ---
async function getMXEPublicKeyRobust(provider: anchor.AnchorProvider, programId: PublicKey): Promise<Uint8Array> {
  const mxeAddress = getMXEAccAddress(programId);
  for (let i = 0; i < 15; i++) {
    try {
      const info = await provider.connection.getAccountInfo(mxeAddress);
      if (info && info.data.length >= 73) {
        return new Uint8Array(info.data.slice(41, 73)); // 物理切片获取 x25519 公钥
      }
    } catch (e) {}
    await new Promise(r => setTimeout(r, 3000));
  }
  throw new Error("Could not fetch MXE public key from Devnet");
}

function readKpJson(path: string): anchor.web3.Keypair {
  return anchor.web3.Keypair.fromSecretKey(new Uint8Array(JSON.parse(fs.readFileSync(path, "utf-8"))));
}