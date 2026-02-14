use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    pub struct GenomeData {
        // Treat DNA sequences as 8 segments of 64-bit integers
        // In real applications, this could be millions of u64 values
        pub sequences: [u64; 8],
    }

    pub struct MatchParams {
        pub threshold: u64, // Dynamically allowed matching threshold
    }

    /// Structure for comparison results
    pub struct MatchResult {
        pub matching_segments: u64, // Number of successfully matched segments
        pub is_relative: bool,      // Whether determined as a relative (1=true, 0=false)
    }

    /// Core instruction: Privacy-preserving DNA similarity computation
    /// 
    /// Principle (MPC):
    /// 1. Data enters the computation nodes in the form of Secret Shares.
    /// 2. Nodes compute equality through communication protocols without revealing any party's raw data.
    /// 3. The output is also in an encrypted state, and only users with the private key can decrypt the result.
    #[instruction]
    pub fn compute_dna_similarity(
        user_dna: Enc<Shared, GenomeData>,
        target_dna: Enc<Shared, GenomeData>,
        params: Enc<Shared, MatchParams> // New: Support dynamic parameters
    ) -> Enc<Shared, MatchResult> {
        let user = user_dna.to_arcis();
        let target = target_dna.to_arcis();
        let p = params.to_arcis();
        
        let mut score = 0u64;

        // Parallel comparison circuit
        // In MPC arithmetic circuits, equality checks (a == b) are usually compiled as subtraction and zero checks
        for i in 0..8 {
            // Compare whether two gene segments are completely identical
            let is_match = user.sequences[i] == target.sequences[i];
            
            // Accumulate matching score
            score = if is_match { score + 1 } else { score };
        }

        // Threshold determination logic
        // If matching segments >= threshold, it is considered a relative
        let is_rel = score >= p.threshold;

        let result = MatchResult {
            matching_segments: score,
            is_relative: is_rel,
        };

        // The result is only visible to the user who initiated the computation (re-encrypted for the caller)
        user_dna.owner.from_arcis(result)
    }
}