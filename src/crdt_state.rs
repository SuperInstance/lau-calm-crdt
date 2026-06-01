//! State-based CRDTs (CvRDTs).

use std::collections::{HashMap, HashSet};
use crate::semilattice::{BoundedJoinSemilattice, VectorMax};

// ---------------------------------------------------------------------------
// G-Counter
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GCounter {
    pub n: usize,
    pub counts: VectorMax,
}

impl GCounter {
    pub fn new(n: usize) -> Self { GCounter { n, counts: VectorMax::new(n) } }
    pub fn inc(&mut self, i: usize) { assert!(i < self.n); self.counts.inc(i); }
    pub fn value(&self) -> u64 { self.counts.0.iter().sum() }
    pub fn read(&self, i: usize) -> u64 { self.counts.0[i] }
}

impl BoundedJoinSemilattice for GCounter {
    fn bottom() -> Self { GCounter { n: 0, counts: VectorMax::new(0) } }
    fn join(&self, other: &Self) -> Self {
        GCounter { n: self.n.max(other.n), counts: self.counts.join(&other.counts) }
    }
}

// ---------------------------------------------------------------------------
// PN-Counter
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PNCounter {
    pub n: usize,
    pub pos: GCounter,
    pub neg: GCounter,
}

impl PNCounter {
    pub fn new(n: usize) -> Self { PNCounter { n, pos: GCounter::new(n), neg: GCounter::new(n) } }
    pub fn inc(&mut self, i: usize) { self.pos.inc(i); }
    pub fn dec(&mut self, i: usize) { self.neg.inc(i); }
    pub fn value(&self) -> i64 { self.pos.value() as i64 - self.neg.value() as i64 }
}

impl BoundedJoinSemilattice for PNCounter {
    fn bottom() -> Self { PNCounter { n: 0, pos: GCounter::bottom(), neg: GCounter::bottom() } }
    fn join(&self, other: &Self) -> Self {
        PNCounter { n: self.n.max(other.n), pos: self.pos.join(&other.pos), neg: self.neg.join(&other.neg) }
    }
}

// ---------------------------------------------------------------------------
// G-Set
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GSet<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> {
    elements: HashSet<T>,
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> GSet<T> {
    pub fn new() -> Self { GSet { elements: HashSet::new() } }
    pub fn add(&mut self, elem: T) { self.elements.insert(elem); }
    pub fn contains(&self, elem: &T) -> bool { self.elements.contains(elem) }
    pub fn elements(&self) -> &HashSet<T> { &self.elements }
    pub fn len(&self) -> usize { self.elements.len() }
    pub fn is_empty(&self) -> bool { self.elements.is_empty() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> Default for GSet<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> BoundedJoinSemilattice for GSet<T> {
    fn bottom() -> Self { GSet { elements: HashSet::new() } }
    fn join(&self, other: &Self) -> Self { GSet { elements: &self.elements | &other.elements } }
}

// ---------------------------------------------------------------------------
// OR-Set
// ---------------------------------------------------------------------------

pub type Tag = u64;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ORSet<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> {
    elements: HashMap<T, HashSet<Tag>>,
    tombstones: HashSet<Tag>,
    next_tag: Tag,
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> ORSet<T> {
    pub fn new() -> Self { ORSet { elements: HashMap::new(), tombstones: HashSet::new(), next_tag: 0 } }
    pub fn add(&mut self, elem: T) -> Tag {
        let tag = self.next_tag; self.next_tag += 1;
        self.elements.entry(elem).or_default().insert(tag); tag
    }
    pub fn remove(&mut self, elem: &T) {
        if let Some(tags) = self.elements.get(elem) {
            for t in tags.iter().copied() { self.tombstones.insert(t); }
        }
        self.elements.remove(elem);
    }
    pub fn contains(&self, elem: &T) -> bool { self.elements.contains_key(elem) }
    pub fn elements(&self) -> Vec<&T> { self.elements.keys().collect() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> Default for ORSet<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> BoundedJoinSemilattice for ORSet<T> {
    fn bottom() -> Self { ORSet { elements: HashMap::new(), tombstones: HashSet::new(), next_tag: 0 } }
    fn join(&self, other: &Self) -> Self {
        let mut result = ORSet {
            elements: HashMap::new(),
            tombstones: &self.tombstones | &other.tombstones,
            next_tag: self.next_tag.max(other.next_tag),
        };
        for (elem, tags) in &self.elements {
            let filtered: HashSet<Tag> = tags - &result.tombstones;
            if !filtered.is_empty() { result.elements.insert(elem.clone(), filtered); }
        }
        for (elem, tags) in &other.elements {
            let filtered: HashSet<Tag> = tags - &result.tombstones;
            if filtered.is_empty() { continue; }
            result.elements.entry(elem.clone()).or_default().extend(filtered);
        }
        result
    }
}

// ---------------------------------------------------------------------------
// LWW-Register
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LWWRegister<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> {
    pub value: Option<T>,
    pub timestamp: u64,
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> LWWRegister<T> {
    pub fn new() -> Self { LWWRegister { value: None, timestamp: 0 } }
    pub fn set(&mut self, value: T, timestamp: u64) {
        if timestamp >= self.timestamp { self.value = Some(value); self.timestamp = timestamp; }
    }
    pub fn get(&self) -> Option<&T> { self.value.as_ref() }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> Default for LWWRegister<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> BoundedJoinSemilattice for LWWRegister<T> {
    fn bottom() -> Self { LWWRegister { value: None, timestamp: 0 } }
    fn join(&self, other: &Self) -> Self {
        if other.timestamp > self.timestamp { other.clone() } else { self.clone() }
    }
}

// ---------------------------------------------------------------------------
// LWW-Element-Set
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LWWElementSet<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> {
    add_set: HashMap<T, u64>,
    remove_set: HashMap<T, u64>,
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> LWWElementSet<T> {
    pub fn new() -> Self { LWWElementSet { add_set: HashMap::new(), remove_set: HashMap::new() } }
    pub fn add(&mut self, elem: T, ts: u64) { self.add_set.insert(elem, ts); }
    pub fn remove(&mut self, elem: T, ts: u64) { self.remove_set.insert(elem, ts); }
    pub fn contains(&self, elem: &T) -> bool {
        match (self.add_set.get(elem), self.remove_set.get(elem)) {
            (Some(a), Some(r)) => a > r, (Some(_), None) => true, _ => false,
        }
    }
    pub fn elements(&self) -> Vec<&T> { self.add_set.keys().filter(|e| self.contains(e)).collect() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> Default for LWWElementSet<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> BoundedJoinSemilattice for LWWElementSet<T> {
    fn bottom() -> Self { LWWElementSet { add_set: HashMap::new(), remove_set: HashMap::new() } }
    fn join(&self, other: &Self) -> Self {
        let mut add_set = self.add_set.clone();
        for (k, v) in &other.add_set { add_set.entry(k.clone()).and_modify(|e| { if v > e { *e = *v; } }).or_insert(*v); }
        let mut remove_set = self.remove_set.clone();
        for (k, v) in &other.remove_set { remove_set.entry(k.clone()).and_modify(|e| { if v > e { *e = *v; } }).or_insert(*v); }
        LWWElementSet { add_set, remove_set }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcounter_increment_and_merge() {
        let mut a = GCounter::new(3); let mut b = GCounter::new(3);
        a.inc(0); a.inc(0); b.inc(1); b.inc(1); b.inc(1);
        assert_eq!(a.join(&b).value(), 5);
    }

    #[test]
    fn gcounter_idempotent_merge() {
        let mut a = GCounter::new(2); a.inc(0); a.inc(1);
        assert_eq!(a.join(&a), a);
    }

    #[test]
    fn pncounter_inc_dec() {
        let mut c = PNCounter::new(2); c.inc(0); c.inc(0); c.dec(1);
        assert_eq!(c.value(), 1);
    }

    #[test]
    fn pncounter_merge() {
        let mut a = PNCounter::new(2); let mut b = PNCounter::new(2);
        a.inc(0); b.dec(1); assert_eq!(a.join(&b).value(), 0);
    }

    #[test]
    fn gset_add_and_merge() {
        let mut a: GSet<&str> = GSet::new(); let mut b: GSet<&str> = GSet::new();
        a.add("hello"); b.add("world");
        let m = a.join(&b); assert!(m.contains(&"hello")); assert!(m.contains(&"world"));
    }

    #[test]
    fn gset_idempotent() {
        let mut a: GSet<i32> = GSet::new(); a.add(1); a.add(1); assert_eq!(a.len(), 1);
    }

    #[test]
    fn orset_add_remove() {
        let mut s: ORSet<&str> = ORSet::new(); s.add("x");
        assert!(s.contains(&"x")); s.remove(&"x"); assert!(!s.contains(&"x"));
    }

    #[test]
    fn orset_merge_concurrent_add() {
        let mut a: ORSet<&str> = ORSet::new(); let mut b: ORSet<&str> = ORSet::new();
        a.add("x"); b.add("x"); assert!(a.join(&b).contains(&"x"));
    }

    #[test]
    fn lww_register_latest_wins() {
        let mut r: LWWRegister<&str> = LWWRegister::new();
        r.set("alice", 1); r.set("bob", 5); assert_eq!(r.get(), Some(&"bob"));
        r.set("charlie", 3); assert_eq!(r.get(), Some(&"bob"));
    }

    #[test]
    fn lww_register_merge() {
        let mut a: LWWRegister<&str> = LWWRegister::new(); let mut b: LWWRegister<&str> = LWWRegister::new();
        a.set("alice", 10); b.set("bob", 20); assert_eq!(a.join(&b).get(), Some(&"bob"));
    }

    #[test]
    fn lww_element_set_add_remove() {
        let mut s: LWWElementSet<&str> = LWWElementSet::new();
        s.add("x", 1); assert!(s.contains(&"x")); s.remove("x", 5); assert!(!s.contains(&"x"));
    }

    #[test]
    fn lww_element_set_merge() {
        let mut a: LWWElementSet<String> = LWWElementSet::new(); let mut b: LWWElementSet<String> = LWWElementSet::new();
        a.add("x".to_string(), 1); b.remove("x".to_string(), 5);
        assert!(!a.join(&b).contains(&"x".to_string()));
    }
}
