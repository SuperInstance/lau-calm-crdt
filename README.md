# lau-calm-crdt

> CALM theorem and CRDT join-semilattice theory — formal convergence guarantees for distributed agent state

## What This Does

CALM theorem and CRDT join-semilattice theory — formal convergence guarantees for distributed agent state. Part of the PLATO/LAU ecosystem — a mathematically rigorous framework for building educational agents that learn, teach, and evolve.

## The Key Idea

This crate implements the core abstractions needed for its domain, with a focus on correctness, composability, and conservation guarantees. Every public type is serializable (serde), every algorithm is tested, and every invariant is verified.

## Install

```bash
cargo add lau-calm-crdt
```

## Quick Start

See the API Reference below for complete usage. Key entry points:

```rust
use lau_calm_crdt::*;
// See types and methods below for complete usage
```

## API Reference

```rust
pub struct DirectedSet<L: BoundedJoinSemilattice> 
    pub fn new(elements: Vec<L>) -> Self  DirectedSet { elements } }
    pub fn is_directed(&self) -> bool 
    pub fn supremum(&self) -> L 
pub fn verify_scott_continuous<L, F>(directed: &DirectedSet<L>, f: &F) -> bool
pub fn verify_scott_continuous_multi<L, F>(directed_sets: &[DirectedSet<L>], f: &F) -> bool
pub struct Dcpo<L: BoundedJoinSemilattice> 
    pub fn new(elements: Vec<L>) -> Self  Dcpo { elements } }
    pub fn bottom(&self) -> L  L::bottom() }
    pub fn contains(&self, elem: &L) -> bool  self.elements.contains(elem) }
    pub fn supremum_of(&self, indices: &[usize]) -> L 
pub struct AgentNode<L: BoundedJoinSemilattice> 
    pub fn new(id: u64) -> Self  AgentNode { id, state: L::bottom(), op_count: 0 } }
    pub fn apply_update(&mut self, f: impl FnOnce(&L) -> L)  self.state = f(&self.state); self.op_count += 1; }
    pub fn merge_from(&mut self, other: &AgentNode<L>)  self.state = self.state.join(&other.state); self.op_count += 1; }
pub struct VerificationResult 
pub fn verify_crdt<L, F>(crdt_name: &str, samples: &[L], update_fn: &F) -> VerificationResult
pub struct SmartCRDT 
    pub fn new(n_agents: usize) -> Self 
    pub fn tick(&mut self, agent_id: usize)  self.heartbeat[agent_id] += 1; self.version[agent_id] += 1; }
    pub fn assign_task(&mut self, task: String)  self.tasks.insert(task); }
    pub fn active_agents(&self) -> usize  self.heartbeat.iter().filter(|&&h| h > 0).count() }
pub trait OpCrdt: Clone + PartialEq + Eq + std::fmt::Debug + Send + Sync 
pub enum CounterOp  Inc(i64) }
pub struct OpCounter  pub value: i64 }
    pub fn new() -> Self  OpCounter { value: 0 } }
pub enum SetOp<T: Clone + Send + Sync>  Add(T), Remove(T) }
pub struct OpSet<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync> 
    pub fn new() -> Self  OpSet { elements: std::collections::HashSet::new() } }
    pub fn contains(&self, elem: &T) -> bool  self.elements.contains(elem) }
    pub fn elements(&self) -> &std::collections::HashSet<T>  &self.elements }
pub struct RegisterOp<T: Clone + Send + Sync>  pub value: T, pub timestamp: u64 }
pub struct OpLWWRegister<T: Clone + Eq + std::fmt::Debug + Send + Sync> 
    pub fn new() -> Self  OpLWWRegister { value: None, timestamp: 0 } }
pub enum MapOp<K: Clone + std::fmt::Debug + Send + Sync, V: Clone + Send + Sync>  Put(K, V), Remove(K) }
pub struct OpMap<K: Eq + std::hash::Hash + Clone + std::fmt::Debug, V: Clone + Eq + std::fmt::Debug> 
    pub fn new() -> Self  OpMap { data: HashMap::new() } }
    pub fn get(&self, k: &K) -> Option<&V>  self.data.get(k).map(|(v, _)| v) }
pub enum CoproductTag  Left, Right }
pub struct Coproduct<A: BoundedJoinSemilattice, B: BoundedJoinSemilattice> 
    pub fn left(a: A) -> Self  Coproduct { tag: Some(CoproductTag::Left), left: Some(a), right: None } }
    pub fn right(b: B) -> Self  Coproduct { tag: Some(CoproductTag::Right), left: None, right: Some(b) } }
pub struct FunctionCrdt<K, V>
    pub fn new() -> Self  FunctionCrdt { map: HashMap::new() } }
    pub fn apply(&mut self, key: K, value: V) 
    pub fn lookup(&self, key: &K) -> Option<&V>  self.map.get(key) }
    pub fn map(&self) -> &HashMap<K, V>  &self.map }
pub struct ListCrdt<T: Clone + Eq + std::fmt::Debug + Send + Sync + 'static> 
    pub fn new() -> Self  ListCrdt { elements: vec![], clock: 0 } }
    pub fn insert(&mut self, item: T) -> u64 
    pub fn elements(&self) -> &[(u64, T)]  &self.elements }
    pub fn len(&self) -> usize  self.elements.len() }
pub trait DeltaCrdt: BoundedJoinSemilattice 
pub struct DeltaGCounter 
    pub fn new(n: usize, replica_id: usize) -> Self 
    pub fn inc(&mut self)  self.inner.inc(self.replica_id); self.delta_counts[self.replica_id] += 1; }
    pub fn value(&self) -> u64  self.inner.value() }
pub struct GCounterDelta  pub counts: Vec<u64> }
pub struct DeltaGSet<T: Eq + std::hash::Hash + Clone + std::fmt::Debug + Send + Sync + 'static> 
    pub fn new() -> Self  DeltaGSet { inner: GSet::new(), delta: GSet::new() } }
```

## How It Works

Read the source in `src/` for full implementation details. All algorithms are documented with inline comments explaining the mathematical foundations.

## The Math

This crate implements formal mathematical constructs. See the source documentation for theorem statements and proofs of correctness.

## Testing

**79 tests** covering construction, serialization, correctness properties, edge cases, and composability with other lau-* crates.

## License

MIT
