//! Experimental admission of one blindly sampled mature producer per actual birth.
use super::*;
pub(crate) struct ParentAdmission {
    capture: Compute,
    claim: Compute,
    update: Compute,
    advance: Compute,
}
impl ParentAdmission {
    pub fn new(d: &wgpu::Device, s: &Simulation) -> Self {
        let c0 = buffer(
            d,
            "parent genome cache 0",
            MAX_AGENTS as u64 * GENOME_BANK_STRIDE as u64 * 4,
        );
        let c1 = buffer(
            d,
            "parent genome cache 1",
            MAX_AGENTS as u64 * GENOME_BANK_STRIDE as u64 * 4,
        );
        let ct = buffer(
            d,
            "parent traits cache",
            MAX_AGENTS as u64 * std::mem::size_of::<CognitiveTraits>() as u64,
        );
        let make = |entry| {
            Compute::new(
                d,
                "parent admission",
                include_str!("parent_admission.wgsl"),
                entry,
                "rurrrwwwwwwww",
                pair(|i| {
                    vec![
                        &s.agent_buffers[i],
                        &s.params_buffer,
                        &s.claims_buffer,
                        &s.genome_buffers[0],
                        &s.genome_buffers[1],
                        &c0,
                        &c1,
                        &ct,
                        &s.reservoir_genome_buffers[0],
                        &s.reservoir_genome_buffers[1],
                        &s.reservoir_traits_buffer,
                        &s.reservoir_rng_buffer,
                        &s.reservoir_claims_buffer,
                    ]
                }),
            )
        };
        Self {
            capture: make("capture"),
            claim: make("claim"),
            update: make("main"),
            advance: make("advance"),
        }
    }
    pub fn before_fusion(&self, e: &mut wgpu::CommandEncoder, i: usize) {
        self.capture.dispatch(e, i, MAX_AGENTS.div_ceil(64), 1);
    }
    pub fn after_fusion(&self, e: &mut wgpu::CommandEncoder, i: usize, claims: &wgpu::Buffer) {
        e.clear_buffer(claims, 0, None);
        self.claim.dispatch(e, i, MAX_AGENTS.div_ceil(64), 1);
        self.update.dispatch(e, i, MAX_AGENTS.div_ceil(64), 1);
        self.advance.dispatch(e, i, 1, 1);
    }
}
