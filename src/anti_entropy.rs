//! Anti-entropy protocol.

use crate::semilattice::BoundedJoinSemilattice;

/// A Merkle tree hash for state comparison.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MerkleHash(pub [u8; 32]);

impl MerkleHash { pub fn zero() -> Self { MerkleHash([0u8; 32]) } }

/// A simple Merkle tree node for state diffing.
#[derive(Clone, Debug)]
pub struct MerkleNode {
    pub hash: MerkleHash,
    pub children: Vec<MerkleNode>,
    pub key: Option<u64>,
}

impl MerkleNode {
    pub fn leaf(key: u64, hash: [u8; 32]) -> Self {
        MerkleNode { hash: MerkleHash(hash), children: vec![], key: Some(key) }
    }
    pub fn empty() -> Self { MerkleNode { hash: MerkleHash::zero(), children: vec![], key: None } }

    pub fn diff(&self, other: &MerkleNode) -> Vec<u64> {
        if self.hash == other.hash { return vec![]; }
        if self.children.is_empty() && other.children.is_empty() {
            let mut keys = vec![];
            if let Some(k) = self.key { keys.push(k); }
            if let Some(k) = other.key { if !keys.contains(&k) { keys.push(k); } }
            return keys;
        }
        let mut diffs = vec![];
        let max_len = self.children.len().max(other.children.len());
        let empty = MerkleNode::empty();
        for i in 0..max_len {
            let left = self.children.get(i).unwrap_or(&empty);
            let right = other.children.get(i).unwrap_or(&empty);
            diffs.extend(left.diff(right));
        }
        diffs
    }
}

/// A replica in the anti-entropy protocol.
#[derive(Clone, Debug)]
pub struct Replica<L: BoundedJoinSemilattice> {
    pub id: usize,
    pub state: L,
    pub version: u64,
}

impl<L: BoundedJoinSemilattice> Replica<L> {
    pub fn new(id: usize) -> Self { Replica { id, state: L::bottom(), version: 0 } }
    pub fn sync_from(&mut self, other: &Replica<L>) -> bool {
        let merged = self.state.join(&other.state);
        if merged != self.state { self.state = merged; self.version += 1; true } else { false }
    }
}

/// Gossip-based anti-entropy simulation.
#[derive(Debug, Clone)]
pub struct GossipNetwork<L: BoundedJoinSemilattice> {
    pub replicas: Vec<Replica<L>>,
}

impl<L: BoundedJoinSemilattice> GossipNetwork<L> {
    pub fn new(n: usize) -> Self {
        GossipNetwork { replicas: (0..n).map(|i| Replica::new(i)).collect() }
    }

    pub fn gossip_round(&mut self) -> usize {
        let n = self.replicas.len();
        let states: Vec<L> = self.replicas.iter().map(|r| r.state.clone()).collect();
        let mut changes = 0;
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let merged = self.replicas[i].state.join(&states[j]);
                    if merged != self.replicas[i].state {
                        self.replicas[i].state = merged;
                        self.replicas[i].version += 1;
                        changes += 1;
                    }
                }
            }
        }
        changes
    }

    pub fn converge(&mut self, max_rounds: usize) -> bool {
        for _ in 0..max_rounds {
            if self.is_converged() { return true; }
            self.gossip_round();
        }
        self.is_converged()
    }

    pub fn is_converged(&self) -> bool {
        if self.replicas.len() <= 1 { return true; }
        let first = &self.replicas[0].state;
        self.replicas.iter().all(|r| r.state == *first)
    }
}

/// Merkle-based state sync.
#[derive(Debug, Clone)]
pub struct MerkleSync {
    pub local_tree: MerkleNode,
    pub remote_tree: MerkleNode,
}

impl MerkleSync {
    pub fn new(local: MerkleNode, remote: MerkleNode) -> Self { MerkleSync { local_tree: local, remote_tree: remote } }
    pub fn diff_keys(&self) -> Vec<u64> { self.local_tree.diff(&self.remote_tree) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semilattice::NatMax;
    use crate::crdt_state::GSet;

    #[test]
    fn merkle_diff_same() {
        let a = MerkleNode::leaf(1, [1u8; 32]);
        assert!(a.diff(&MerkleNode::leaf(1, [1u8; 32])).is_empty());
    }

    #[test]
    fn merkle_diff_different() {
        let a = MerkleNode::leaf(1, [1u8; 32]);
        assert!(!a.diff(&MerkleNode::leaf(1, [2u8; 32])).is_empty());
    }

    #[test]
    fn replica_sync() {
        let mut r1 = Replica::new(0); let mut r2 = Replica::new(1);
        r1.state = NatMax(5); r2.state = NatMax(3);
        r2.sync_from(&r1); assert_eq!(r2.state, NatMax(5));
    }

    #[test]
    fn gossip_convergence() {
        let mut net = GossipNetwork::new(3);
        net.replicas[0].state = NatMax(10);
        net.replicas[1].state = NatMax(5);
        net.replicas[2].state = NatMax(3);
        assert!(net.converge(10));
        assert!(net.replicas.iter().all(|r| r.state == NatMax(10)));
    }

    #[test]
    fn gossip_gset_convergence() {
        let mut net: GossipNetwork<GSet<u64>> = GossipNetwork::new(3);
        net.replicas[0].state.add(1); net.replicas[0].state.add(2);
        net.replicas[1].state.add(3); net.replicas[2].state.add(4);
        assert!(net.converge(10));
        for r in &net.replicas {
            assert!(r.state.contains(&1) && r.state.contains(&2) && r.state.contains(&3) && r.state.contains(&4));
        }
    }

    #[test]
    fn merkle_sync_diff_keys() {
        let sync = MerkleSync::new(MerkleNode::leaf(1, [1u8; 32]), MerkleNode::leaf(2, [2u8; 32]));
        assert!(!sync.diff_keys().is_empty());
    }
}
