//! CUDAclaw SmartCRDT formal verification.

use crate::semilattice::BoundedJoinSemilattice;
use crate::calm::{calm_analysis, verify_monotone, verify_semilattice_axioms};
use crate::anti_entropy::GossipNetwork;

/// A CUDAclaw agent node with a local CRDT state.
#[derive(Clone, Debug)]
pub struct AgentNode<L: BoundedJoinSemilattice> {
    pub id: u64,
    pub state: L,
    pub op_count: u64,
}

impl<L: BoundedJoinSemilattice> AgentNode<L> {
    pub fn new(id: u64) -> Self { AgentNode { id, state: L::bottom(), op_count: 0 } }
    pub fn apply_update(&mut self, f: impl FnOnce(&L) -> L) { self.state = f(&self.state); self.op_count += 1; }
    pub fn merge_from(&mut self, other: &AgentNode<L>) { self.state = self.state.join(&other.state); self.op_count += 1; }
}

/// Verification result for a CUDAclaw SmartCRDT.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub crdt_name: String,
    pub is_monotone: bool,
    pub is_coordination_free: bool,
    pub converges: bool,
    pub semilattice_axioms_hold: bool,
    pub explanation: String,
}

/// Verify that a CRDT satisfies all convergence properties.
pub fn verify_crdt<L, F>(crdt_name: &str, samples: &[L], update_fn: &F) -> VerificationResult
where L: BoundedJoinSemilattice, F: Fn(&L) -> L {
    let is_monotone = verify_monotone(samples, update_fn);
    let analysis = calm_analysis(samples, update_fn, crdt_name);
    let axioms_hold = verify_semilattice_axioms(samples);
    let converges = verify_convergence(samples);
    VerificationResult {
        crdt_name: crdt_name.to_string(),
        is_monotone,
        is_coordination_free: analysis.is_coordination_free,
        converges,
        semilattice_axioms_hold: axioms_hold,
        explanation: if is_monotone && axioms_hold && converges {
            format!("CUDAclaw SmartCRDT '{}' VERIFIED: operations are monotone, semilattice axioms hold, convergence guaranteed without coordination.", crdt_name)
        } else {
            format!("CUDAclaw SmartCRDT '{}' WARNING: monotone={}, axioms={}, converges={}", crdt_name, is_monotone, axioms_hold, converges)
        },
    }
}

fn verify_convergence<L: BoundedJoinSemilattice>(samples: &[L]) -> bool {
    if samples.len() < 2 { return true; }
    let forward = samples.iter().fold(L::bottom(), |acc, s| acc.join(s));
    let reverse = samples.iter().rev().fold(L::bottom(), |acc, s| acc.join(s));
    if forward != reverse { return false; }
    if samples.len() >= 3 {
        let mut pairs: Vec<L> = vec![];
        for i in (0..samples.len()).step_by(2) {
            if i + 1 < samples.len() { pairs.push(samples[i].join(&samples[i + 1])); }
            else { pairs.push(samples[i].clone()); }
        }
        let pairwise = pairs.into_iter().fold(L::bottom(), |acc, s| acc.join(&s));
        if forward != pairwise { return false; }
    }
    true
}

/// SmartCRDT: CUDAclaw's combined CRDT for agent state.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SmartCRDT {
    pub heartbeat: Vec<u64>,
    pub tasks: std::collections::HashSet<String>,
    pub version: Vec<u64>,
}

impl SmartCRDT {
    pub fn new(n_agents: usize) -> Self {
        SmartCRDT { heartbeat: vec![0u64; n_agents], tasks: std::collections::HashSet::new(), version: vec![0u64; n_agents] }
    }
    pub fn tick(&mut self, agent_id: usize) { self.heartbeat[agent_id] += 1; self.version[agent_id] += 1; }
    pub fn assign_task(&mut self, task: String) { self.tasks.insert(task); }
    pub fn active_agents(&self) -> usize { self.heartbeat.iter().filter(|&&h| h > 0).count() }
}

impl BoundedJoinSemilattice for SmartCRDT {
    fn bottom() -> Self { SmartCRDT { heartbeat: vec![], tasks: std::collections::HashSet::new(), version: vec![] } }
    fn join(&self, other: &Self) -> Self {
        let n = self.heartbeat.len().max(other.heartbeat.len());
        let mut hb = vec![0u64; n];
        for i in 0..self.heartbeat.len() { hb[i] = self.heartbeat[i]; }
        for i in 0..other.heartbeat.len() { hb[i] = hb[i].max(other.heartbeat[i]); }
        let mut ver = vec![0u64; n];
        for i in 0..self.version.len() { ver[i] = self.version[i]; }
        for i in 0..other.version.len() { ver[i] = ver[i].max(other.version[i]); }
        SmartCRDT { heartbeat: hb, tasks: &self.tasks | &other.tasks, version: ver }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semilattice::NatMax;
    use crate::crdt_state::GCounter;

    #[test]
    fn agent_node_merge() {
        let mut a = AgentNode::new(0); let mut b = AgentNode::new(1);
        a.apply_update(|_| NatMax(5)); b.apply_update(|_| NatMax(3));
        a.merge_from(&b); assert_eq!(a.state, NatMax(5));
    }

    #[test]
    fn smart_crdt_heartbeat_and_merge() {
        let mut a = SmartCRDT::new(3); let mut b = SmartCRDT::new(3);
        a.tick(0); a.tick(0); b.tick(1);
        assert_eq!(a.join(&b).heartbeat, vec![2, 1, 0]);
    }

    #[test]
    fn smart_crdt_tasks_merge() {
        let mut a = SmartCRDT::new(2); let mut b = SmartCRDT::new(2);
        a.assign_task("render".to_string()); b.assign_task("compile".to_string());
        let merged = a.join(&b);
        assert!(merged.tasks.contains("render")); assert!(merged.tasks.contains("compile"));
    }

    #[test]
    fn verify_gcounter_crdt() {
        let samples = vec![
            GCounter::new(2),
            { let mut g = GCounter::new(2); g.inc(0); g },
            { let mut g = GCounter::new(2); g.inc(0); g.inc(1); g },
        ];
        let result = verify_crdt("G-Counter", &samples, &|x: &GCounter| { let mut c = x.clone(); c.inc(0); c });
        assert!(result.is_monotone); assert!(result.converges); assert!(result.semilattice_axioms_hold);
    }

    #[test]
    fn verify_smart_crdt() {
        let s0 = SmartCRDT::new(2);
        let mut s1 = SmartCRDT::new(2); s1.tick(0);
        let mut s2 = SmartCRDT::new(2); s2.tick(1); s2.assign_task("task_a".to_string());
        let result = verify_crdt("SmartCRDT", &[s0, s1, s2], &|x: &SmartCRDT| { let mut c = x.clone(); c.tick(0); c });
        assert!(result.is_monotone); assert!(result.converges);
    }

    #[test]
    fn smart_crdt_gossip_convergence() {
        let mut net: GossipNetwork<SmartCRDT> = GossipNetwork::new(3);
        net.replicas[0].state = SmartCRDT::new(3); net.replicas[0].state.tick(0);
        net.replicas[1].state = SmartCRDT::new(3); net.replicas[1].state.tick(1);
        net.replicas[1].state.tasks.insert("task_x".to_string());
        net.replicas[2].state = SmartCRDT::new(3); net.replicas[2].state.tick(2);
        assert!(net.converge(10));
        assert!(net.replicas.iter().all(|r| r.state.tasks.contains("task_x")));
    }

    #[test]
    fn smart_crdt_active_agents() {
        let mut s = SmartCRDT::new(4); s.tick(0); s.tick(2);
        assert_eq!(s.active_agents(), 2);
    }
}
