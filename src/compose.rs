//! CRDT composition: product, coproduct, function space.

use crate::semilattice::{BoundedJoinSemilattice, Product};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Product CRDT: two independent CRDTs composed in parallel.
pub type ProductCrdt<A, B> = Product<A, B>;

/// Tag for coproduct.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CoproductTag { Left, Right }

/// Coproduct CRDT.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Coproduct<A: BoundedJoinSemilattice, B: BoundedJoinSemilattice> {
    pub tag: Option<CoproductTag>,
    pub left: Option<A>,
    pub right: Option<B>,
}

impl<A: BoundedJoinSemilattice, B: BoundedJoinSemilattice> Coproduct<A, B> {
    pub fn left(a: A) -> Self { Coproduct { tag: Some(CoproductTag::Left), left: Some(a), right: None } }
    pub fn right(b: B) -> Self { Coproduct { tag: Some(CoproductTag::Right), left: None, right: Some(b) } }
}

impl<A: BoundedJoinSemilattice, B: BoundedJoinSemilattice> BoundedJoinSemilattice for Coproduct<A, B> {
    fn bottom() -> Self { Coproduct { tag: None, left: None, right: None } }
    fn join(&self, other: &Self) -> Self {
        match (&self.tag, &other.tag) {
            (None, _) => other.clone(),
            (_, None) => self.clone(),
            (Some(CoproductTag::Left), Some(CoproductTag::Left)) =>
                Coproduct::left(self.left.as_ref().unwrap().join(other.left.as_ref().unwrap())),
            (Some(CoproductTag::Right), Some(CoproductTag::Right)) =>
                Coproduct::right(self.right.as_ref().unwrap().join(other.right.as_ref().unwrap())),
            _ => Coproduct::bottom(),
        }
    }
}

/// Function-space CRDT: K → V pointwise join.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FunctionCrdt<K, V>
where K: Eq + Hash + Clone + std::fmt::Debug + Send + Sync + 'static, V: BoundedJoinSemilattice {
    map: HashMap<K, V>,
}

impl<K, V> FunctionCrdt<K, V>
where K: Eq + Hash + Clone + std::fmt::Debug + Send + Sync + 'static, V: BoundedJoinSemilattice {
    pub fn new() -> Self { FunctionCrdt { map: HashMap::new() } }
    pub fn apply(&mut self, key: K, value: V) {
        self.map.entry(key).and_modify(|e| *e = e.join(&value)).or_insert(value);
    }
    pub fn lookup(&self, key: &K) -> Option<&V> { self.map.get(key) }
    pub fn map(&self) -> &HashMap<K, V> { &self.map }
}

impl<K, V> Default for FunctionCrdt<K, V>
where K: Eq + Hash + Clone + std::fmt::Debug + Send + Sync + 'static, V: BoundedJoinSemilattice {
    fn default() -> Self { Self::new() }
}

impl<K, V> BoundedJoinSemilattice for FunctionCrdt<K, V>
where K: Eq + Hash + Clone + std::fmt::Debug + Send + Sync + 'static, V: BoundedJoinSemilattice {
    fn bottom() -> Self { FunctionCrdt { map: HashMap::new() } }
    fn join(&self, other: &Self) -> Self {
        let mut result = self.map.clone();
        for (k, v) in &other.map {
            result.entry(k.clone()).and_modify(|e| *e = e.join(v)).or_insert_with(|| v.clone());
        }
        FunctionCrdt { map: result }
    }
}

/// A simplified list/sequence CRDT.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ListCrdt<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> {
    elements: Vec<(u64, T)>,
    clock: u64,
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> ListCrdt<T> {
    pub fn new() -> Self { ListCrdt { elements: vec![], clock: 0 } }
    pub fn insert(&mut self, item: T) -> u64 {
        let ts = self.clock; self.clock += 1;
        self.elements.push((ts, item)); ts
    }
    pub fn elements(&self) -> &[(u64, T)] { &self.elements }
    pub fn len(&self) -> usize { self.elements.len() }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> Default for ListCrdt<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> BoundedJoinSemilattice for ListCrdt<T> {
    fn bottom() -> Self { ListCrdt { elements: vec![], clock: 0 } }
    fn join(&self, other: &Self) -> Self {
        let mut seen = HashSet::new();
        let mut merged: Vec<(u64, T)> = vec![];
        for &(ts, ref item) in &self.elements { if seen.insert(ts) { merged.push((ts, item.clone())); } }
        for &(ts, ref item) in &other.elements { if seen.insert(ts) { merged.push((ts, item.clone())); } }
        merged.sort_by_key(|(ts, _)| *ts);
        ListCrdt { elements: merged, clock: self.clock.max(other.clock) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semilattice::{BoolOr, NatMax, Product};

    #[test]
    fn product_crdt_join() {
        let joined = Product(NatMax(1), BoolOr(false)).join(&Product(NatMax(3), BoolOr(true)));
        assert_eq!(joined.0, NatMax(3)); assert_eq!(joined.1, BoolOr(true));
    }

    #[test]
    fn coproduct_left_join() {
        let joined = Coproduct::<NatMax, NatMax>::left(NatMax(1)).join(&Coproduct::left(NatMax(3)));
        assert_eq!(joined.left, Some(NatMax(3)));
    }

    #[test]
    fn coproduct_right_join() {
        let joined = Coproduct::<BoolOr, BoolOr>::right(BoolOr(false)).join(&Coproduct::right(BoolOr(true)));
        assert_eq!(joined.right, Some(BoolOr(true)));
    }

    #[test]
    fn coproduct_mixed_gives_bottom() {
        let a = Coproduct::<NatMax, BoolOr>::left(NatMax(1));
        let b = Coproduct::<NatMax, BoolOr>::right(BoolOr(true));
        assert_eq!(a.join(&b), Coproduct::bottom());
    }

    #[test]
    fn function_crdt_apply_and_join() {
        let mut f: FunctionCrdt<&str, NatMax> = FunctionCrdt::new();
        f.apply("a", NatMax(1)); f.apply("b", NatMax(5)); f.apply("a", NatMax(3));
        assert_eq!(f.lookup(&"a"), Some(&NatMax(3))); assert_eq!(f.lookup(&"b"), Some(&NatMax(5)));
    }

    #[test]
    fn function_crdt_merge() {
        let mut f1: FunctionCrdt<&str, NatMax> = FunctionCrdt::new();
        f1.apply("x", NatMax(1));
        let mut f2: FunctionCrdt<&str, NatMax> = FunctionCrdt::new();
        f2.apply("x", NatMax(5)); f2.apply("y", NatMax(2));
        let merged = f1.join(&f2);
        assert_eq!(merged.lookup(&"x"), Some(&NatMax(5))); assert_eq!(merged.lookup(&"y"), Some(&NatMax(2)));
    }

    #[test]
    fn list_crdt_merge() {
        let mut a: ListCrdt<String> = ListCrdt::new();
        a.insert("x".to_string()); a.insert("y".to_string());
        let mut b: ListCrdt<String> = ListCrdt::new();
        // b's clock starts at 0, so "z" gets ts=0 which collides with a's ts=0 ("x")
        // Join dedupes by ts, so merged has {"x"(ts=0), "y"(ts=1)}
        b.insert("z".to_string());
        let merged = a.join(&b);
        assert_eq!(merged.len(), 2);
    }
}
