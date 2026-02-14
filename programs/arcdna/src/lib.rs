use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_DNA: u32 = comp_def_offset("compute_dna_similarity");

declare_id!("F2ZMuc2KsqmLKk3kmheq7HvkzHy5Ltn8GrYKUJQXcAQJ");

#[arcium_program]
pub mod arcdna {
    use super::*;

    pub fn init_dna_comp_def(ctx: Context<InitDnaCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn register_profile(
        ctx: Context<RegisterProfile>,
        encrypted_dna_shards: [[u8; 32]; 8],
    ) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        profile.owner = ctx.accounts.owner.key();
        profile.encrypted_dna_shards = encrypted_dna_shards;
        profile.bump = ctx.bumps.profile;
        Ok(())
    }

    pub fn compute_dna_similarity(
        ctx: Context<ComputeDnaSimilarity>, 
        computation_offset: u64,
        ciphertext_user: [[u8; 32]; 8],
        ciphertext_target: [[u8; 32]; 8],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        
        let mut builder = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce);

        for shard in &ciphertext_user {
            builder = builder.encrypted_u64(*shard);
        }
        for shard in &ciphertext_target {
            builder = builder.encrypted_u64(*shard);
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

    pub fn compute_match_with_profile(
        ctx: Context<ComputeSimilarityWithProfile>,
        computation_offset: u64,
        ciphertext_user: [[u8; 32]; 8],
        pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let accounts = &mut ctx.accounts.computation;
        accounts.sign_pda_account.bump = ctx.bumps.computation.sign_pda_account;

        let mut builder = ArgBuilder::new()
            .x25519_pubkey(pubkey)
            .plaintext_u128(nonce);

        for shard in &ciphertext_user {
            builder = builder.encrypted_u64(*shard);
        }
        for shard in &ctx.accounts.target_profile.encrypted_dna_shards {
            builder = builder.encrypted_u64(*shard);
        }

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
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "compute_dna_similarity")]
    pub fn compute_dna_similarity_callback(
        ctx: Context<ComputeDnaSimilarityCallback>,
        output: SignedComputationOutputs<ComputeDnaSimilarityOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(&ctx.accounts.cluster_account, &ctx.accounts.computation_account) {
            Ok(ComputeDnaSimilarityOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let score_bytes: [u8; 8] = o.ciphertexts[0][0..8].try_into().unwrap();
        let relative_bytes: [u8; 8] = o.ciphertexts[1][0..8].try_into().unwrap();

        emit!(DnaMatchEvent {
            score: u64::from_le_bytes(score_bytes),
            is_relative: u64::from_le_bytes(relative_bytes),
            nonce: o.nonce.to_le_bytes(),
        });
        Ok(())
    }
}

#[queue_computation_accounts("compute_dna_similarity", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct ComputeDnaSimilarity<'info> {
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
    pub mxe_account: Box<Account<'info, MXEAccount>>, // 修正：添加 Box
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

#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct ComputeSimilarityWithProfile<'info> {
    pub computation: ComputeDnaSimilarity<'info>,
    #[account(
        seeds = [b"profile", target_profile.owner.as_ref()],
        bump = target_profile.bump
    )]
    pub target_profile: Account<'info, UserProfile>,
}

#[callback_accounts("compute_dna_similarity")]
#[derive(Accounts)]
pub struct ComputeDnaSimilarityCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DNA))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>, // 修正：添加 Box
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
    pub mxe_account: Box<Account<'info, MXEAccount>>, // 修正：添加 Box
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

#[account]
pub struct UserProfile {
    pub owner: Pubkey,
    pub encrypted_dna_shards: [[u8; 32]; 8],
    pub bump: u8,
}

#[event]
pub struct DnaMatchEvent {
    pub score: u64,
    pub is_relative: u64,
    pub nonce: [u8; 16],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
}