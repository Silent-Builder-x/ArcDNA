use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

// Arcium imports for advanced circuit handling (MPC-ready)
use arcium_client::idl::arcium::types::{CircuitSource, OffChainCircuitSource};
use arcium_macros::circuit_hash;

const COMP_DEF_OFFSET_DNA: u32 = comp_def_offset("compute_dna_similarity");

declare_id!("BQbwqV2LhBNcxLjFwQRXfF8UU1fULKdx87nMQ5m3nQLK");

#[arcium_program]
pub mod arcdna {
    use super::*;

    pub fn init_dna_config(ctx: Context<InitDnaCompDef>) -> Result<()> {
        // Advanced: Using Off-Chain source for larger genomic circuits to save Gas.
        // We use the raw GitHub URL so Arcium nodes can fetch the binary circuit file directly.
        init_comp_def(
            ctx.accounts, 
            Some(CircuitSource::OffChain(OffChainCircuitSource {
                source: "https://raw.githubusercontent.com/Silent-Builder-x/ArcDNA/main/build/compute_dna_similarity.arcis".to_string(),
                hash: circuit_hash!("compute_dna_similarity"),
            })), 
            None
        )?;
        Ok(())
    }

    pub fn request_genomic_match(
        ctx: Context<RequestDnaMatch>,
        computation_offset: u64,
        user_dna_shards: [[u8; 32]; 4],   // User sample
        target_dna_shards: [[u8; 32]; 4], // Target sample
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        
        let mut builder = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce);
        
        // Sequentially add encrypted shards to the MPC computation queue
        for s in user_dna_shards {
            builder = builder.encrypted_u64(s);
        }
        for s in target_dna_shards {
            builder = builder.encrypted_u64(s);
        }

        queue_computation(
            ctx.accounts,
            computation_offset,
            builder.build(),
            vec![ComputeDnaSimilarityCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[]
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "compute_dna_similarity")]
    pub fn compute_dna_similarity_callback(
        ctx: Context<ComputeDnaSimilarityCallback>,
        output: SignedComputationOutputs<ComputeDnaSimilarityOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(result) => result,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        // Emit privacy-preserving result for client-side decryption
        emit!(DnaMatchEvent {
            encrypted_score: o.field_0.ciphertexts[0],
            encrypted_is_relative: o.field_0.ciphertexts[1],
            nonce: o.field_0.nonce.to_le_bytes(),
        });
        
        msg!("Confidential DNA Matching Completed via MXE.");
        Ok(())
    }
}

// --- Events ---
#[event]
pub struct DnaMatchEvent {
    pub encrypted_score: [u8; 32],
    pub encrypted_is_relative: [u8; 32],
    pub nonce: [u8; 16],
}

#[queue_computation_accounts("compute_dna_similarity", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RequestDnaMatch<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init_if_needed, space = 9, payer = payer, seeds = [&SIGN_PDA_SEED], bump, address = derive_sign_pda!())]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: Internal Arcium mempool
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: Internal execution pool
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: Tracking current genomic match instance
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DNA))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    /// CHECK: Arcium Fee Pool
    pub pool_account: Account<'info, FeePool>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    /// CHECK: Arcium Clock Account
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("compute_dna_similarity")]
#[derive(Accounts)]
pub struct ComputeDnaSimilarityCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DNA))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: Validated result from MXE cluster
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: System instructions sysvar
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("compute_dna_similarity", payer)]
#[derive(Accounts)]
pub struct InitDnaCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: Initializing genomic definition
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: LUT for network routing
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: Official LUT Program
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Aborted")] AbortedComputation,
    #[msg("No Cluster")] ClusterNotSet,
}