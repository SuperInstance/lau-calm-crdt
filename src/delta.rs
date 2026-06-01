//! Delta-state CRDTs.

use crate::semilattice::BoundedJoinSemilattice;
use crate::crdt_state::{GCounter, GSet, LWWRegister};

/// A delta-state CRDT.
pub trait DeltaCrdt: BoundedJoinSemilattice {
    type Delta: BoundedJoinSemilattice + Into<Self> + Clone;
    fn extract_delta(&mut self) -> Self::Delta;
    fn merge_delta(&mut self, delta: &Self::Delta) {
        let merged = self.join(&delta.clone().into());
        *self = merged;
    }
}

// ---------------------------------------------------------------------------
// Delta-GCounter
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeltaGCounter {
    pub inner: GCounter,
    pub replica_id: usize,
    delta_counts: Vec<u64>,
}

impl DeltaGCounter {
    pub fn new(n: usize, replica_id: usize) -> Self {
        DeltaGCounter { inner: GCounter::new(n), replica_id, delta_counts: vec![0u64; n] }
    }
    pub fn inc(&mut self) { self.inner.inc(self.replica_id); self.delta_counts[self.replica_id] += 1; }
    pub fn value(&self) -> u64 { self.inner.value() }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GCounterDelta { pub counts: Vec<u64> }

impl BoundedJoinSemilattice for GCounterDelta {
    fn bottom() -> Self { GCounterDelta { counts: vec![] } }
    fn join(&self, other: &Self) -> Self {
        let len = self.counts.len().max(other.counts.len());
        let mut result = vec![0u64; len];
        for i in 0..self.counts.len() { result[i] = self.counts[i]; }
        for i in 0..other.counts.len() { result[i] = result[i].max(other.counts[i]); }
        GCounterDelta { counts: result }
    }
}

impl From<GCounterDelta> for DeltaGCounter {
    fn from(delta: GCounterDelta) -> Self {
        let n = delta.counts.len();
        DeltaGCounter {
            inner: GCounter { n, counts: crate::semilattice::VectorMax(delta.counts) },
            replica_id: 0,
            delta_counts: vec![0; n],
        }
    }
}

impl BoundedJoinSemilattice for DeltaGCounter {
    fn bottom() -> Self { DeltaGCounter { inner: GCounter::bottom(), replica_id: 0, delta_counts: vec![] } }
    fn join(&self, other: &Self) -> Self {
        DeltaGCounter {
            inner: self.inner.join(&other.inner),
            replica_id: self.replica_id,
            delta_counts: vec![0; self.inner.n.max(other.inner.n)],
        }
    }
}

impl DeltaCrdt for DeltaGCounter {
    type Delta = GCounterDelta;
    fn extract_delta(&mut self) -> Self::Delta {
        let delta = GCounterDelta { counts: std::mem::take(&mut self.delta_counts) };
        self.delta_counts = vec![0; self.inner.n];
        delta
    }
}

// ---------------------------------------------------------------------------
// Delta-GSet
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeltaGSet<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> {
    pub inner: GSet<T>,
    delta: GSet<T>,
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> DeltaGSet<T> {
    pub fn new() -> Self { DeltaGSet { inner: GSet::new(), delta: GSet::new() } }
    pub fn add(&mut self, elem: T) { self.inner.add(elem.clone()); self.delta.add(elem); }
    pub fn contains(&self, elem: &T) -> bool { self.inner.contains(elem) }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> Default for DeltaGSet<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> BoundedJoinSemilattice for DeltaGSet<T> {
    fn bottom() -> Self { DeltaGSet { inner: GSet::new(), delta: GSet::new() } }
    fn join(&self, other: &Self) -> Self { DeltaGSet { inner: self.inner.join(&other.inner), delta: GSet::new() } }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> From<GSet<T>> for DeltaGSet<T> {
    fn from(inner: GSet<T>) -> Self { DeltaGSet { inner, delta: GSet::new() } }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> DeltaCrdt for DeltaGSet<T> {
    type Delta = GSet<T>;
    fn extract_delta(&mut self) -> Self::Delta { std::mem::replace(&mut self.delta, GSet::new()) }
}

// ---------------------------------------------------------------------------
// Delta-LWW-Register
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeltaLWWRegister<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> {
    pub inner: LWWRegister<T>,
    pending_delta: Option<LWWRegister<T>>,
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> DeltaLWWRegister<T> {
    pub fn new() -> Self { DeltaLWWRegister { inner: LWWRegister::new(), pending_delta: None } }
    pub fn set(&mut self, value: T, timestamp: u64) {
        self.inner.set(value.clone(), timestamp);
        let mut delta = LWWRegister::new(); delta.set(value, timestamp);
        self.pending_delta = Some(delta);
    }
    pub fn get(&self) -> Option<&T> { self.inner.get() }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> Default for DeltaLWWRegister<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> BoundedJoinSemilattice for DeltaLWWRegister<T> {
    fn bottom() -> Self { DeltaLWWRegister { inner: LWWRegister::bottom(), pending_delta: None } }
    fn join(&self, other: &Self) -> Self { DeltaLWWRegister { inner: self.inner.join(&other.inner), pending_delta: None } }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> From<LWWRegister<T>> for DeltaLWWRegister<T> {
    fn from(inner: LWWRegister<T>) -> Self { DeltaLWWRegister { inner, pending_delta: None } }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> DeltaCrdt for DeltaLWWRegister<T> {
    type Delta = LWWRegister<T>;
    fn extract_delta(&mut self) -> Self::Delta { self.pending_delta.take().unwrap_or_else(LWWRegister::bottom) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_gcounter_propagation() {
        let mut a = DeltaGCounter::new(2, 0); let mut b = DeltaGCounter::new(2, 1);
        a.inc(); a.inc(); b.inc();
        let delta = a.extract_delta(); b.merge_delta(&delta);
        assert_eq!(b.value(), 3);
    }

    #[test]
    fn delta_gcounter_empty_delta() {
        let mut a = DeltaGCounter::new(2, 0);
        let delta = a.extract_delta();
        assert!(delta.counts.iter().all(|&c| c == 0));
    }

    #[test]
    fn delta_gset_propagation() {
        let mut a: DeltaGSet<&str> = DeltaGSet::new(); let mut b: DeltaGSet<&str> = DeltaGSet::new();
        a.add("x"); a.add("y");
        let delta = a.extract_delta(); b.merge_delta(&delta);
        assert!(b.contains(&"x")); assert!(b.contains(&"y"));
    }

    #[test]
    fn delta_gset_second_delta_empty() {
        let mut a: DeltaGSet<i32> = DeltaGSet::new(); a.add(1);
        let _d1 = a.extract_delta();
        let d2 = a.extract_delta();
        assert!(d2.is_empty());
    }

    #[test]
    fn delta_lww_register_propagation() {
        let mut a = DeltaLWWRegister::new(); let mut b = DeltaLWWRegister::new();
        a.set("hello", 10);
        let delta = a.extract_delta(); b.merge_delta(&delta);
        assert_eq!(b.get(), Some(&"hello"));
    }

    #[test]
    fn delta_lww_register_older_ignored() {
        let mut a = DeltaLWWRegister::new();
        a.set("new", 10); a.set("old", 5);
        assert_eq!(a.get(), Some(&"new"));
    }
}
