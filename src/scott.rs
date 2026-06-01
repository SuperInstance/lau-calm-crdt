//! Scott continuity and DCPOs.

use crate::semilattice::BoundedJoinSemilattice;

/// A directed set.
#[derive(Debug, Clone)]
pub struct DirectedSet<L: BoundedJoinSemilattice> {
    pub elements: Vec<L>,
}

impl<L: BoundedJoinSemilattice> DirectedSet<L> {
    pub fn new(elements: Vec<L>) -> Self { DirectedSet { elements } }

    pub fn is_directed(&self) -> bool {
        for i in 0..self.elements.len() {
            for j in (i + 1)..self.elements.len() {
                let lub = self.elements[i].join(&self.elements[j]);
                if !self.elements.iter().any(|e| lub.leq(e)) { return false; }
            }
        }
        true
    }

    pub fn supremum(&self) -> L {
        self.elements.iter().fold(L::bottom(), |acc, e| acc.join(e))
    }
}

/// Verify Scott continuity: f(⊔D) == ⊔{f(d) | d ∈ D}.
pub fn verify_scott_continuous<L, F>(directed: &DirectedSet<L>, f: &F) -> bool
where L: BoundedJoinSemilattice, F: Fn(&L) -> L {
    let f_sup = f(&directed.supremum());
    let image_sup = directed.elements.iter().map(f).fold(L::bottom(), |acc, e| acc.join(&e));
    f_sup == image_sup
}

/// Verify Scott continuity on multiple directed sets.
pub fn verify_scott_continuous_multi<L, F>(directed_sets: &[DirectedSet<L>], f: &F) -> bool
where L: BoundedJoinSemilattice, F: Fn(&L) -> L {
    directed_sets.iter().all(|d| verify_scott_continuous(d, f))
}

/// A DCPO with bottom.
#[derive(Debug, Clone)]
pub struct Dcpo<L: BoundedJoinSemilattice> {
    pub elements: Vec<L>,
}

impl<L: BoundedJoinSemilattice> Dcpo<L> {
    pub fn new(elements: Vec<L>) -> Self { Dcpo { elements } }
    pub fn bottom(&self) -> L { L::bottom() }
    pub fn contains(&self, elem: &L) -> bool { self.elements.contains(elem) }
    pub fn supremum_of(&self, indices: &[usize]) -> L {
        indices.iter().map(|&i| self.elements[i].clone()).fold(L::bottom(), |acc, e| acc.join(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semilattice::NatMax;

    #[test]
    fn directed_set_valid() {
        assert!(DirectedSet::new(vec![NatMax(1), NatMax(2), NatMax(3)]).is_directed());
    }

    #[test]
    fn directed_set_supremum() {
        assert_eq!(DirectedSet::new(vec![NatMax(1), NatMax(2), NatMax(3)]).supremum(), NatMax(3));
    }

    #[test]
    fn scott_continuous_monotone() {
        let d = DirectedSet::new(vec![NatMax(0), NatMax(2), NatMax(4)]);
        assert!(verify_scott_continuous(&d, &|x: &NatMax| NatMax(x.0 + 1)));
    }

    #[test]
    fn scott_continuous_identity() {
        let d = DirectedSet::new(vec![NatMax(1), NatMax(5)]);
        assert!(verify_scott_continuous(&d, &|x: &NatMax| x.clone()));
    }

    #[test]
    fn scott_continuous_constant() {
        let d = DirectedSet::new(vec![NatMax(1), NatMax(3)]);
        assert!(verify_scott_continuous(&d, &|_: &NatMax| NatMax(42)));
    }

    #[test]
    fn dcpo_bottom_and_supremum() {
        let dcpo = Dcpo::new(vec![NatMax(1), NatMax(3), NatMax(5)]);
        assert_eq!(dcpo.bottom(), NatMax(0));
        assert_eq!(dcpo.supremum_of(&[0, 1, 2]), NatMax(5));
    }

    #[test]
    fn verify_multi_scott() {
        let sets = vec![
            DirectedSet::new(vec![NatMax(0), NatMax(2)]),
            DirectedSet::new(vec![NatMax(1), NatMax(5)]),
        ];
        assert!(verify_scott_continuous_multi(&sets, &|x: &NatMax| NatMax(x.0 * 2)));
    }
}
