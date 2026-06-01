//! Operation-based CRDTs (CmRDTs).

use std::collections::HashMap;

/// Trait for operation-based CRDTs.
pub trait OpCrdt: Clone + PartialEq + Eq + std::fmt::Debug + Send + Sync {
    type Op: Clone + std::fmt::Debug + Send + Sync;
    fn apply(&mut self, op: &Self::Op);
    fn verify_commutative(base: &Self, a: &Self::Op, b: &Self::Op) -> bool {
        let mut s1 = base.clone(); s1.apply(a); s1.apply(b);
        let mut s2 = base.clone(); s2.apply(b); s2.apply(a);
        s1 == s2
    }
}

// ---------------------------------------------------------------------------
// OpCounter
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum CounterOp { Inc(i64) }

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OpCounter { pub value: i64 }

impl OpCounter {
    pub fn new() -> Self { OpCounter { value: 0 } }
}

impl Default for OpCounter { fn default() -> Self { Self::new() } }

impl OpCrdt for OpCounter {
    type Op = CounterOp;
    fn apply(&mut self, op: &Self::Op) {
        match op { CounterOp::Inc(amount) => self.value += amount }
    }
}

// ---------------------------------------------------------------------------
// OpSet
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum SetOp<T: Clone + Send + Sync> { Add(T), Remove(T) }

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OpSet<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync> {
    elements: std::collections::HashSet<T>,
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync> OpSet<T> {
    pub fn new() -> Self { OpSet { elements: std::collections::HashSet::new() } }
    pub fn contains(&self, elem: &T) -> bool { self.elements.contains(elem) }
    pub fn elements(&self) -> &std::collections::HashSet<T> { &self.elements }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync> Default for OpSet<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync> OpCrdt for OpSet<T> {
    type Op = SetOp<T>;
    fn apply(&mut self, op: &Self::Op) {
        match op {
            SetOp::Add(elem) => { self.elements.insert(elem.clone()); }
            SetOp::Remove(elem) => { self.elements.remove(elem); }
        }
    }
}

// ---------------------------------------------------------------------------
// OpLWWRegister
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct RegisterOp<T: Clone + Send + Sync> { pub value: T, pub timestamp: u64 }

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OpLWWRegister<T: Clone + Eq + std::fmt::Debug + Send + Sync> {
    pub value: Option<T>, pub timestamp: u64,
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync> OpLWWRegister<T> {
    pub fn new() -> Self { OpLWWRegister { value: None, timestamp: 0 } }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync> Default for OpLWWRegister<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Clone + Eq + std::fmt::Debug + Send + Sync> OpCrdt for OpLWWRegister<T> {
    type Op = RegisterOp<T>;
    fn apply(&mut self, op: &Self::Op) {
        if op.timestamp >= self.timestamp { self.value = Some(op.value.clone()); self.timestamp = op.timestamp; }
    }
}

// ---------------------------------------------------------------------------
// OpMap
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum MapOp<K: Clone + std::fmt::Debug + Send + Sync, V: Clone + Send + Sync> { Put(K, V), Remove(K) }

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OpMap<K: Eq + std::hash::Hash + Clone + std::fmt::Debug, V: Clone + Eq + std::fmt::Debug> {
    data: HashMap<K, (V, u64)>,
}

impl<K: Eq + std::hash::Hash + Clone + std::fmt::Debug, V: Clone + Eq + std::fmt::Debug> OpMap<K, V> {
    pub fn new() -> Self { OpMap { data: HashMap::new() } }
    pub fn get(&self, k: &K) -> Option<&V> { self.data.get(k).map(|(v, _)| v) }
}

impl<K: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync, V: Clone + Eq + std::fmt::Debug + Send + Sync> Default for OpMap<K, V> {
    fn default() -> Self { Self::new() }
}

impl<K: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync, V: Clone + Eq + std::fmt::Debug + Send + Sync> OpCrdt for OpMap<K, V> {
    type Op = MapOp<K, V>;
    fn apply(&mut self, op: &Self::Op) {
        match op {
            MapOp::Put(k, v) => {
                let ts = self.data.get(k).map(|(_, ts)| ts + 1).unwrap_or(1);
                self.data.insert(k.clone(), (v.clone(), ts));
            }
            MapOp::Remove(k) => { self.data.remove(k); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_counter_commutativity() {
        let base = OpCounter::new();
        assert!(OpCounter::verify_commutative(&base, &CounterOp::Inc(5), &CounterOp::Inc(3)));
    }

    #[test]
    fn op_counter_convergence() {
        let ops = vec![CounterOp::Inc(1), CounterOp::Inc(2), CounterOp::Inc(3)];
        let mut s1 = OpCounter::new(); for op in &ops { s1.apply(op); }
        let mut s2 = OpCounter::new(); for op in ops.iter().rev() { s2.apply(op); }
        assert_eq!(s1, s2); assert_eq!(s1.value, 6);
    }

    #[test]
    fn op_set_commutativity() {
        let base = OpSet::<String>::new();
        assert!(OpSet::verify_commutative(&base, &SetOp::Add("x".into()), &SetOp::Add("y".into())));
    }

    #[test]
    fn op_lww_register_commutativity() {
        let base = OpLWWRegister::<String>::new();
        let a = RegisterOp { value: "alice".into(), timestamp: 1 };
        let b = RegisterOp { value: "bob".into(), timestamp: 2 };
        assert!(OpLWWRegister::verify_commutative(&base, &a, &b));
    }

    #[test]
    fn op_lww_register_order_independence() {
        let ops = vec![
            RegisterOp { value: "a".to_string(), timestamp: 1 },
            RegisterOp { value: "b".to_string(), timestamp: 3 },
            RegisterOp { value: "c".to_string(), timestamp: 2 },
        ];
        let mut s1 = OpLWWRegister::new(); for op in &ops { s1.apply(op); }
        let mut s2 = OpLWWRegister::new(); for op in ops.iter().rev() { s2.apply(op); }
        assert_eq!(s1, s2); assert_eq!(s1.value, Some("b".to_string()));
    }

    #[test]
    fn op_map_put_remove() {
        let mut m: OpMap<String, i32> = OpMap::new();
        m.apply(&MapOp::Put("key".to_string(), 42)); assert_eq!(m.get(&"key".to_string()), Some(&42));
        m.apply(&MapOp::Remove::<String, i32>("key".to_string())); assert_eq!(m.get(&"key".to_string()), None);
    }
}
