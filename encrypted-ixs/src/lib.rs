use arcis::*;

#[encrypted]
mod dna_match_engine {
    use arcis::*;

    pub struct GenomeData {
        pub sequences: [u64; 4], // 模拟 4 段高度敏感的基因特征哈希
    }

    pub struct MatchResult {
        pub similarity_score: u64, // 匹配成功的片段数量
        pub is_relative: u64,      // 1 为亲属 (匹配 >= 3), 0 为非亲属
    }

    #[instruction]
    pub fn compute_dna_similarity(
        user_dna_ctxt: Enc<Shared, GenomeData>,
        target_dna_ctxt: Enc<Shared, GenomeData>
    ) -> Enc<Shared, MatchResult> {
        let user = user_dna_ctxt.to_arcis();
        let target = target_dna_ctxt.to_arcis();
        
        let mut score = 0u64;

        // 执行同态汉明距离计算 (Hamming Distance)
        for i in 0..4 {
            let is_match = user.sequences[i] == target.sequences[i];
            
            // 使用 V4 规范的 if-else Mux 逻辑进行密文累加
            score = if is_match { score + 1 } else { score };
        }

        // 阈值判定：如果匹配数 >= 3，判定为高度相似
        let relative_flag = if score >= 3 { 1u64 } else { 0u64 };

        let result = MatchResult {
            similarity_score: score,
            is_relative: relative_flag,
        };

        user_dna_ctxt.owner.from_arcis(result)
    }
}