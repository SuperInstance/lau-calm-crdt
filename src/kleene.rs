//! Kleene fixpoints and ascending chains.

use crate::semilattice::BoundedJoinSemilattice;

/// Compute the Kleene fixpoint: iterate f from bottom until stabilization.
pub fn kleene_fixpoint<L, F>(f: &F, max_iterations: usize) -> L
where L: BoundedJoinSemilattice, F: Fn(&L) -> L {
    let mut current = L::bottom();
    for _ in 0..max_iterations {
        let next = f(&current);
        if next == current { return current; }
        current = next;
    }
    current
}

/// An ascending chain in a lattice.
#[derive(Debug, Clone)]
pub struct AscendingChain<L: BoundedJoinSemilattice> {
    pub elements: Vec<L>,
}

impl<L: BoundedJoinSemilattice> AscendingChain<L> {
    pub fn new() -> Self { AscendingChain { elements: vec![] } }
    pub fn from_bottom() -> Self { AscendingChain { elements: vec![L::bottom()] } }

    pub fn push(&mut self, elem: L) -> bool {
        if let Some(last) = self.elements.last() {
            if !last.leq(&elem) { return false; }
        }
        self.elements.push(elem);
        true
    }

    pub fn supremum(&self) -> L {
        self.elements.iter().fold(L::bottom(), |acc, e| acc.join(e))
    }

    pub fn is_stabilized(&self) -> bool {
        if self.elements.len() < 2 { return false; }
        let n = self.elements.len();
        self.elements[n - 1] == self.elements[n - 2]
    }

    pub fn iterate<F>(f: &F, max_steps: usize) -> Self
    where F: Fn(&L) -> L {
        let mut chain = Self::from_bottom();
        for _ in 0..max_steps {
            let next = f(chain.elements.last().unwrap());
            let stabilized = next == *chain.elements.last().unwrap();
            chain.push(next);
            if stabilized { break; }
        }
        chain
    }
}

impl<L: BoundedJoinSemilattice> Default for AscendingChain<L> {
    fn default() -> Self { Self::new() }
}

/// Verify that a function has a fixpoint by iteration.
pub fn verify_fixpoint<L, F>(f: &F, max_iterations: usize) -> Option<L>
where L: BoundedJoinSemilattice, F: Fn(&L) -> L {
    let mut current = L::bottom();
    for _ in 0..max_iterations {
        let next = f(&current);
        if next == current { return Some(current); }
        current = next;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semilattice::NatMax;
    use crate::crdt_state::GCounter;

    #[test]
    fn kleene_fixpoint_constant() {
        let fp = kleene_fixpoint(&|x: &NatMax| x.clone(), 100);
        assert_eq!(fp, NatMax::bottom());
    }

    #[test]
    fn kleene_fixpoint_ascending() {
        let fp = kleene_fixpoint(&|x: &NatMax| NatMax(x.0.min(4) + 1), 100);
        assert_eq!(fp, NatMax(5));
    }

    #[test]
    fn ascending_chain_valid() {
        let mut chain = AscendingChain::from_bottom();
        chain.push(NatMax(1)); chain.push(NatMax(3)); chain.push(NatMax(5));
        assert_eq!(chain.supremum(), NatMax(5)); assert!(!chain.is_stabilized());
    }

    #[test]
    fn ascending_chain_invalid_push() {
        let mut chain = AscendingChain::from_bottom();
        chain.push(NatMax(5));
        assert!(!chain.push(NatMax(3)));
    }

    #[test]
    fn ascending_chain_stabilized() {
        let mut chain = AscendingChain::from_bottom();
        chain.push(NatMax(5)); chain.push(NatMax(5));
        assert!(chain.is_stabilized());
    }

    #[test]
    fn ascending_chain_iterate() {
        let chain = AscendingChain::iterate(&|x: &NatMax| NatMax(x.0.min(9) + 1), 100);
        assert!(chain.is_stabilized()); assert_eq!(chain.supremum(), NatMax(10));
    }

    #[test]
    fn verify_fixpoint_found() {
        let fp = verify_fixpoint(&|x: &NatMax| NatMax(x.0.saturating_add(1).min(3)), 100);
        assert_eq!(fp, Some(NatMax(3)));
    }

    #[test]
    fn gcounter_fixpoint() {
        let state = GCounter::new(2);
        let fp = kleene_fixpoint(&|x: &GCounter| x.join(&state), 100);
        assert_eq!(fp, state);
    }
}
