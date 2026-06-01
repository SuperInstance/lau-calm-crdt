//! Bounded join-semilattices.
//!
//! A join-semilattice is a partially ordered set where any two elements have a
//! least upper bound (join / merge). Bounded means there exists a bottom element ⊥.

use std::cmp::Ordering;

/// Trait for bounded join-semilattices.
pub trait BoundedJoinSemilattice: Clone + PartialEq + Eq + std::fmt::Debug + Send + Sync + 'static {
    /// The bottom element ⊥ — the identity for join.
    fn bottom() -> Self;

    /// Join (least upper bound) of two elements. Must be:
    /// - Commutative: a.join(b) == b.join(a)
    /// - Associative: a.join(b).join(c) == a.join(b.join(c))
    /// - Idempotent: a.join(a) == a
    /// - Identity: a.join(bottom()) == a
    fn join(&self, other: &Self) -> Self;

    /// Partial order: self ⊑ other.
    fn leq(&self, other: &Self) -> bool {
        self.join(other) == *other
    }

    /// Check if this is the bottom element.
    fn is_bottom(&self) -> bool {
        *self == Self::bottom()
    }
}

// ---------------------------------------------------------------------------
// Natural numbers as a join-semilattice (max)
// ---------------------------------------------------------------------------

/// A natural number join-semilattice where join = max.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NatMax(pub u64);

impl BoundedJoinSemilattice for NatMax {
    fn bottom() -> Self { NatMax(0) }
    fn join(&self, other: &Self) -> Self { NatMax(self.0.max(other.0)) }
}

impl PartialOrd for NatMax {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for NatMax {
    fn cmp(&self, other: &Self) -> Ordering { self.0.cmp(&other.0) }
}

// ---------------------------------------------------------------------------
// Boolean join-semilattice (OR)
// ---------------------------------------------------------------------------

/// Boolean join-semilattice where join = OR.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BoolOr(pub bool);

impl BoundedJoinSemilattice for BoolOr {
    fn bottom() -> Self { BoolOr(false) }
    fn join(&self, other: &Self) -> Self { BoolOr(self.0 || other.0) }
}

// ---------------------------------------------------------------------------
// Product (pair) of semilattices
// ---------------------------------------------------------------------------

/// Product of two semilattices (pointwise join).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Product<A, B>(pub A, pub B);

impl<A: BoundedJoinSemilattice, B: BoundedJoinSemilattice> BoundedJoinSemilattice for Product<A, B> {
    fn bottom() -> Self { Product(A::bottom(), B::bottom()) }
    fn join(&self, other: &Self) -> Self {
        Product(self.0.join(&other.0), self.1.join(&other.1))
    }
}

// ---------------------------------------------------------------------------
// Map semilattice (pointwise join on values)
// ---------------------------------------------------------------------------

/// A map from keys to semilattice values — pointwise join.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MapSemilattice<K, V>
where
    K: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static,
    V: BoundedJoinSemilattice,
{
    entries: std::collections::HashMap<K, V>,
}

impl<K, V> MapSemilattice<K, V>
where
    K: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static,
    V: BoundedJoinSemilattice,
{
    pub fn new() -> Self { MapSemilattice { entries: std::collections::HashMap::new() } }
    pub fn insert(&mut self, key: K, value: V) { self.entries.insert(key, value); }
    pub fn get(&self, key: &K) -> Option<&V> { self.entries.get(key) }
    pub fn entries(&self) -> &std::collections::HashMap<K, V> { &self.entries }
}

impl<K, V> Default for MapSemilattice<K, V>
where
    K: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static,
    V: BoundedJoinSemilattice,
{
    fn default() -> Self { Self::new() }
}

impl<K, V> BoundedJoinSemilattice for MapSemilattice<K, V>
where
    K: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static,
    V: BoundedJoinSemilattice,
{
    fn bottom() -> Self { MapSemilattice { entries: std::collections::HashMap::new() } }
    fn join(&self, other: &Self) -> Self {
        let mut result = self.entries.clone();
        for (k, v) in &other.entries {
            result.entry(k.clone())
                .and_modify(|e| *e = e.join(v))
                .or_insert_with(|| v.clone());
        }
        MapSemilattice { entries: result }
    }
}

// ---------------------------------------------------------------------------
// Vector semilattice (pointwise max, fixed dimension)
// ---------------------------------------------------------------------------

/// A vector of u64 with pointwise-max join — used by G-Counter.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VectorMax(pub Vec<u64>);

impl VectorMax {
    pub fn new(dim: usize) -> Self { VectorMax(vec![0u64; dim]) }
    pub fn dim(&self) -> usize { self.0.len() }
    pub fn inc(&mut self, index: usize) { self.0[index] += 1; }
}

impl BoundedJoinSemilattice for VectorMax {
    fn bottom() -> Self { VectorMax(vec![]) }
    fn join(&self, other: &Self) -> Self {
        let len = self.0.len().max(other.0.len());
        let mut result = vec![0u64; len];
        for i in 0..self.0.len() { result[i] = self.0[i]; }
        for i in 0..other.0.len() { result[i] = result[i].max(other.0[i]); }
        VectorMax(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nat_max_bottom_is_zero() { assert_eq!(NatMax::bottom(), NatMax(0)); }

    #[test]
    fn nat_max_join_commutative() {
        let a = NatMax(3); let b = NatMax(7);
        assert_eq!(a.join(&b), b.join(&a));
    }

    #[test]
    fn nat_max_join_associative() {
        let a = NatMax(1); let b = NatMax(5); let c = NatMax(3);
        assert_eq!(a.join(&b).join(&c), a.join(&b.join(&c)));
    }

    #[test]
    fn nat_max_join_idempotent() {
        let a = NatMax(42);
        assert_eq!(a.join(&a), a);
    }

    #[test]
    fn nat_max_join_identity() {
        let a = NatMax(10);
        assert_eq!(a.join(&NatMax::bottom()), a);
    }

    #[test]
    fn nat_max_leq_consistent() {
        let a = NatMax(3); let b = NatMax(5);
        assert!(a.leq(&b));
        assert!(!b.leq(&a));
    }

    #[test]
    fn bool_or_laws() {
        let t = BoolOr(true); let f = BoolOr(false);
        assert_eq!(f.join(&f), f);
        assert_eq!(f.join(&t), t);
        assert_eq!(t.join(&t), t);
        assert!(f.leq(&t));
    }

    #[test]
    fn product_laws() {
        let a = Product(NatMax(1), BoolOr(false));
        let b = Product(NatMax(3), BoolOr(true));
        assert_eq!(a.join(&b), Product(NatMax(3), BoolOr(true)));
    }

    #[test]
    fn vector_max_join() {
        let a = VectorMax(vec![1, 0, 3]); let b = VectorMax(vec![0, 2, 1]);
        assert_eq!(a.join(&b), VectorMax(vec![1, 2, 3]));
    }

    #[test]
    fn vector_max_idempotent() {
        let a = VectorMax(vec![1, 2, 3]);
        assert_eq!(a.join(&a), a);
    }

    #[test]
    fn map_semilattice_join() {
        let mut a = MapSemilattice::new();
        a.insert("x", NatMax(1)); a.insert("y", NatMax(5));
        let mut b = MapSemilattice::new();
        b.insert("x", NatMax(3)); b.insert("z", NatMax(7));
        let joined = a.join(&b);
        assert_eq!(joined.get(&"x"), Some(&NatMax(3)));
        assert_eq!(joined.get(&"y"), Some(&NatMax(5)));
        assert_eq!(joined.get(&"z"), Some(&NatMax(7)));
    }
}
