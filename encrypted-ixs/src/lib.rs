use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    pub struct GenomeData {
        // 将 DNA 序列视为 8 个 64 位的片段
        // 实际应用中可以是数百万个 u64
        pub sequences: [u64; 8],
    }

    pub struct MatchParams {
        pub threshold: u64, // 动态允许的匹配阈值
    }

    /// 比对结果结构
    pub struct MatchResult {
        pub matching_segments: u64, // 匹配成功的片段数量
        pub is_relative: bool,      // 是否判定为亲属 (1=true, 0=false)
    }

    /// 核心指令：隐私保护下的 DNA 相似度计算
    /// 
    /// 原理 (MPC):
    /// 1. 数据以 Secret Shares 形式进入计算节点。
    /// 2. 节点间通过通信协议计算相等性，不泄露任何一方的原始数据。
    /// 3. 输出也是加密状态，只有拥有私钥的用户能解密结果。
    #[instruction]
    pub fn compute_dna_similarity(
        user_dna: Enc<Shared, GenomeData>,
        target_dna: Enc<Shared, GenomeData>,
        params: Enc<Shared, MatchParams> // 新增：支持动态传参
    ) -> Enc<Shared, MatchResult> {
        let user = user_dna.to_arcis();
        let target = target_dna.to_arcis();
        let p = params.to_arcis();
        
        let mut score = 0u64;

        // 并行比对电路
        // 在 MPC 算术电路中，相等性检查 (a == b) 通常会被编译为减法与零检查
        for i in 0..8 {
            // 比较两个基因片段是否完全一致
            let is_match = user.sequences[i] == target.sequences[i];
            
            // 累加匹配分数
            score = if is_match { score + 1 } else { score };
        }

        // 阈值判定逻辑
        // 如果匹配片段 >= 阈值，则认为是亲属
        let is_rel = score >= p.threshold;

        let result = MatchResult {
            matching_segments: score,
            is_relative: is_rel,
        };

        // 结果仅对发起计算的用户可见 (re-encrypted for the caller)
        user_dna.owner.from_arcis(result)
    }
}