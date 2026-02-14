use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    pub struct GenomeData {
        pub sequences: [u64; 8],
    }

    /// 比对结果结构
    pub struct MatchResult {
        pub similarity_score: u64, // 匹配成功的片段数量
        pub is_relative: u64,      // 阈值判定标识
    }

    /// 核心指令：在不解密的情况下计算汉明相似度
    #[instruction]
    pub fn compute_dna_similarity(
        user_dna_ctxt: Enc<Shared, GenomeData>,
        target_dna_ctxt: Enc<Shared, GenomeData>
    ) -> Enc<Shared, MatchResult> {
        let user = user_dna_ctxt.to_arcis();
        let target = target_dna_ctxt.to_arcis();
        
        let mut score = 0u64;

        // 执行并行比对电路：计算两个序列中相同片段的数量
        // 汉明距离 = 总长度 - score
        for i in 0..8 {
            let is_match = user.sequences[i] == target.sequences[i];
            
            score = if is_match { score + 1 } else { score };
        }

        // 阈值判定：如果匹配片段 >= 6 (75% 相似度)，判定为亲属
        let relative_flag = if score >= 6 { 1u64 } else { 0u64 };

        let result = MatchResult {
            similarity_score: score,
            is_relative: relative_flag,
        };

        // 将结果重新加密并返回给请求者
        user_dna_ctxt.owner.from_arcis(result)
    }
}