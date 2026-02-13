use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    // A simplified representation of genomic data using 4x64-bit hashes.
    // In a real-world scenario, this could be a Bloom filter or MinHash of the genome.
    pub struct GenomeData {
        pub sequences: [u64; 4], // 4 segments of highly sensitive genomic feature hashes
    }

    pub struct MatchResult {
        pub similarity_score: u64, // Number of matching segments
        pub is_relative: u64,      // 1 if match >= threshold, 0 otherwise
    }

    #[instruction]
    pub fn compute_dna_similarity(
        user_dna_ctxt: Enc<Shared, GenomeData>,
        target_dna_ctxt: Enc<Shared, GenomeData>
    ) -> Enc<Shared, MatchResult> {
        let user = user_dna_ctxt.to_arcis();
        let target = target_dna_ctxt.to_arcis();
        
        let mut score = 0u64;

        // Execute secure Hamming Distance calculation in MPC
        for i in 0..4 {
            let is_match = user.sequences[i] == target.sequences[i];
            
            // Use Mux (Multiplexer) logic supported by Arcis for encrypted accumulation
            score = if is_match { score + 1 } else { score };
        }

        // Threshold check: if match count >= 3, flag as highly similar (relative)
        let relative_flag = if score >= 3 { 1u64 } else { 0u64 };

        let result = MatchResult {
            similarity_score: score,
            is_relative: relative_flag,
        };

        // Return the result encrypted with the Shared key (only accessible by the invoker)
        user_dna_ctxt.owner.from_arcis(result)
    }
}