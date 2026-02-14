use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

// Define the computation definition offset, which must correspond to the ID when uploading the circuit in the Arcium CLI
const COMP_DEF_OFFSET_DNA: u32 = comp_def_offset("compute_dna_similarity");

declare_id!("F2ZMuc2KsqmLKk3kmheq7HvkzHy5Ltn8GrYKUJQXcAQJ");

#[arcium_program]
pub mod arcdna {
    use super::*;

    /// Initialize the computation definition (Setup)
    pub fn init_dna_comp_def(ctx: Context<InitDnaCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    /// Register user genetic profile
    pub fn register_profile(
        ctx: Context<RegisterProfile>,
        encrypted_dna_shards: [[u8; 32]; 8], 
    ) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        profile.owner = ctx.accounts.owner.key();
        profile.encrypted_dna_shards = encrypted_dna_shards;
        profile.bump = ctx.bumps.profile;
        
        msg!("Profile registered for user: {}", profile.owner);
        Ok(())
    }

    /// Submit a matching request (Core logic)
    pub fn request_match(
        ctx: Context<RequestMatch>, 
        computation_offset: u64,
        ciphertext_user: [[u8; 32]; 8], 
        encrypted_threshold: [u8; 32], 
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let accounts = &mut ctx.accounts.computation;
        accounts.sign_pda_account.bump = ctx.bumps.computation.sign_pda_account;

        // 1. Build MPC parameters
        let mut builder = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce);

        // Parameter 1: User DNA
        for shard in &ciphertext_user {
            builder = builder.encrypted_u64(*shard);
        }

        // Parameter 2: Target DNA
        for shard in &ctx.accounts.target_profile.encrypted_dna_shards {
            builder = builder.encrypted_u64(*shard);
        }

        // Parameter 3: MatchParams
        builder = builder.encrypted_u64(encrypted_threshold);

        // 2. Queue Computation
        // Fix: The callback structure must be ComputeDnaSimilarityCallback
        queue_computation(
            accounts,
            computation_offset,
            builder.build(),
            vec![ComputeDnaSimilarityCallback::callback_ix(
                computation_offset,
                &accounts.mxe_account,
                &[]
            )?],
            1,
            0,
        )?;
        
        msg!("Computation queued. Target: {}", ctx.accounts.target_profile.owner);
        Ok(())
    }

    /// Callback function: Process MPC computation results
    /// Fix: The function name must be compute_dna_similarity_callback
    #[arcium_callback(encrypted_ix = "compute_dna_similarity")]
    pub fn compute_dna_similarity_callback(
        ctx: Context<ComputeDnaSimilarityCallback>, // Fix: Structure name
        output: SignedComputationOutputs<ComputeDnaSimilarityOutput>, // Fix: Output structure is usually generated based on the instruction name
    ) -> Result<()> {
        // Verify computation results
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(ComputeDnaSimilarityOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        // Parse output results
        // The Arcis circuit returns a MatchResult structure
        // The Arcium macro flattens it into a ciphertexts array
        let score_bytes: [u8; 8] = o.ciphertexts[0][0..8].try_into().unwrap();
        let is_relative_bytes: [u8; 8] = o.ciphertexts[1][0..8].try_into().unwrap();

        emit!(DnaMatchEvent {
            score: u64::from_le_bytes(score_bytes),
            is_relative: u64::from_le_bytes(is_relative_bytes) == 1,
            nonce: o.nonce.to_le_bytes(),
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

// --- Context Structs (Account Validation) ---

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct RequestMatch<'info> {
    pub computation: ComputeDnaSimilarityBase<'info>,
    
    #[account(
        seeds = [b"profile", target_profile.owner.as_ref()],
        bump = target_profile.bump
    )]
    pub target_profile: Account<'info, UserProfile>,
}

#[queue_computation_accounts("compute_dna_similarity", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct ComputeDnaSimilarityBase<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: mempool
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: execpool
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: computation
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DNA))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(mut, address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

// Fix: The structure name must be ComputeDnaSimilarityCallback
#[callback_accounts("compute_dna_similarity")]
#[derive(Accounts)]
pub struct ComputeDnaSimilarityCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DNA))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK: computation_account
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: sysvar
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
    /// CHECK: comp_def
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: lut
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_prog
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterProfile<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        init,
        payer = owner,
        space = 8 + 32 + (32 * 8) + 1, 
        seeds = [b"profile", owner.key().as_ref()],
        bump
    )]
    pub profile: Account<'info, UserProfile>,
    pub system_program: Program<'info, System>,
}

// --- Data Structures ---

#[account]
pub struct UserProfile {
    pub owner: Pubkey,
    pub encrypted_dna_shards: [[u8; 32]; 8],
    pub bump: u8,
}

#[event]
pub struct DnaMatchEvent {
    pub score: u64,
    pub is_relative: bool,
    pub nonce: [u8; 16],
    pub timestamp: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
}