# lau-calm-crdt

**CALM theorem and CRDT join-semilattice theory** — formal convergence guarantees for distributed agent state.

This crate provides a rigorous mathematical framework for Conflict-free Replicated Data Types (CRDTs). Every CRDT is modeled as a bounded join-semilattice, and convergence is guaranteed by the algebraic properties of the lattice. The CALM theorem connects monotonicity to coordination-free distributed computation.

---

## What This Does

- **Bounded join-semilattices**: a trait `BoundedJoinSemilattice` with verified commutativity, associativity, idempotency, and identity laws. Concrete instances: `NatMax`, `BoolOr`, `Product`, `VectorMax`, `MapSemilattice`.
- **State-based CRDTs (CvRDTs)**: `GCounter`, `PNCounter`, `GSet`, `ORSet`, `LWWRegister`, `LWWElementSet` — all implement `BoundedJoinSemilattice` for automatic merge.
- **Operation-based CRDTs (CmRDTs)**: `OpCounter`, `OpSet`, `OpLWWRegister`, `OpMap` — commutative operations verified via `OpCrdt::verify_commutative`.
- **Delta-state CRDTs**: `DeltaGCounter`, `DeltaGSet`, `DeltaLWWRegister` — ship only changes since last sync, not full state.
- **CALM theorem verification**: check whether a function is monotone on the lattice, and therefore whether it admits a coordination-free implementation.
- **Kleene fixpoints**: compute least fixpoints by iterating from bottom; build and inspect ascending chains.
- **Scott continuity**: verify that functions preserve directed suprema on DCPOs.
- **Anti-entropy protocols**: Merkle tree diffing, gossip-based convergence, replica sync.
- **CRDT composition**: products, coproducts, function spaces, and sequences.
- **CUDAclaw SmartCRDT**: a real-world composite CRDT tracking agent heartbeats, tasks, and versions — with full formal verification.

---

## Key Idea

> **CRDTs converge because merge is a join in a join-semilattice.**
>
> The merge operation satisfies: `a ⊔ b = b ⊔ a` (commutative), `(a ⊔ b) ⊔ c = a ⊔ (b ⊔ c)` (associative), `a ⊔ a = a` (idempotent). This means replicas can merge in any order, any number of times, and always reach the same state.
>
> The **CALM theorem** says: a distributed program has a coordination-free implementation **if and only if** it is monotone on a lattice. This crate lets you check that.

---

## Install

```toml
[dependencies]
lau-calm-crdt = "0.1"
```

Or via git:

```toml
[dependencies]
lau-calm-crdt = { git = "https://github.com/SuperInstance/lau-calm-crdt" }
```

Requires Rust 2021 edition. Dependencies: `serde` (serialization), `nalgebra` (linear algebra).

---

## Quick Start

### State-based CRDT

```rust
use lau_calm_crdt::*;

// Two replicas of a G-Counter (3 nodes)
let mut replica_a = GCounter::new(3);
let mut replica_b = GCounter::new(3);

replica_a.inc(0);  // Node 0 increments
replica_a.inc(0);
replica_b.inc(1);  // Node 1 increments
replica_b.inc(1);
replica_b.inc(1);

// Merge: just join the semilattices
let merged = replica_a.join(&replica_b);
assert_eq!(merged.value(), 5);  // Total count

// Idempotent: merging again changes nothing
assert_eq!(merged.join(&merged), merged);
```

### CALM Analysis

```rust
use lau_calm_crdt::*;

let samples = vec![NatMax(0), NatMax(1), NatMax(5), NatMax(10)];

// This function is monotone → coordination-free per CALM
let analysis = calm_analysis(&samples, &|x: &NatMax| NatMax(x.0 + 1), "increment");
assert!(analysis.is_monotone);
assert!(analysis.is_coordination_free);

// This function is NOT monotone → requires coordination
let analysis = calm_analysis(
    &[BoolOr(false), BoolOr(true)],
    &|x: &BoolOr| BoolOr(!x.0),
    "negate",
);
assert!(!analysis.is_monotone);
```

### Delta-state CRDT

```rust
use lau_calm_crdt::*;

let mut a = DeltaGCounter::new(2, 0);
let mut b = DeltaGCounter::new(2, 1);

a.inc(); a.inc();
b.inc();

// Ship only the delta, not the full state
let delta = a.extract_delta();
b.merge_delta(&delta);
assert_eq!(b.value(), 3);  // 1 (b's own) + 2 (from a's delta)
```

### Gossip Convergence

```rust
use lau_calm_crdt::*;

let mut net: GossipNetwork<GSet<u64>> = GossipNetwork::new(3);
net.replicas[0].state.add(1);
net.replicas[0].state.add(2);
net.replicas[1].state.add(3);
net.replicas[2].state.add(4);

assert!(net.converge(10));  // All replicas see {1, 2, 3, 4}
```

### CUDAclaw SmartCRDT Verification

```rust
use lau_calm_crdt::*;

let samples = vec![
    GCounter::new(2),
    { let mut g = GCounter::new(2); g.inc(0); g },
    { let mut g = GCounter::new(2); g.inc(0); g.inc(1); g },
];

let result = verify_crdt("G-Counter", &samples, &|x: &GCounter| {
    let mut c = x.clone(); c.inc(0); c
});
assert!(result.is_monotone);
assert!(result.converges);
assert!(result.semilattice_axioms_hold);
```

---

## API Reference

### Semilattice Foundations (`semilattice`)

| Type / Trait | Description |
|---|---|
| `BoundedJoinSemilattice` | Core trait: `bottom()`, `join()`, `leq()`, `is_bottom()` |
| `NatMax(u64)` | Natural numbers with join = max |
| `BoolOr(bool)` | Booleans with join = OR |
| `Product<A, B>` | Pair of semilattices, pointwise join |
| `VectorMax(Vec<u64>)` | Vector with pointwise max |
| `MapSemilattice<K, V>` | Map with pointwise value join |

### State-based CRDTs (`crdt_state`)

| Type | Description |
|---|---|
| `GCounter` | Grow-only counter (vector of per-node counts, join = pointwise max) |
| `PNCounter` | Increment/decrement counter (pair of G-Counters) |
| `GSet<T>` | Grow-only set (join = union) |
| `ORSet<T>` | Observed-remove set (add wins over concurrent remove) |
| `LWWRegister<T>` | Last-writer-wins register (highest timestamp wins) |
| `LWWElementSet<T>` | LWW element set (add timestamp vs remove timestamp) |

### Operation-based CRDTs (`crdt_op`)

| Type | Operation type | Description |
|---|---|---|
| `OpCounter` | `CounterOp::Inc(i64)` | Commutative counter |
| `OpSet<T>` | `SetOp::Add / Remove` | Commutative set |
| `OpLWWRegister<T>` | `RegisterOp<T>` | Timestamp-ordered register |
| `OpMap<K, V>` | `MapOp::Put / Remove` | Key-value map |

### Delta-state CRDTs (`delta`)

| Type | Delta type | Description |
|---|---|---|
| `DeltaGCounter` | `GCounterDelta` | Ships only changed counter entries |
| `DeltaGSet<T>` | `GSet<T>` | Ships only new elements |
| `DeltaLWWRegister<T>` | `LWWRegister<T>` | Ships latest write |

### CALM Theorem (`calm`)

| Function | Description |
|---|---|
| `verify_monotone(samples, f)` | Check `a ⊑ b ⟹ f(a) ⊑ f(b)` on samples |
| `calm_analysis(samples, f, name)` | Full CALM verdict + explanation |
| `verify_monotone_sequence(seq)` | Check `[x₀, x₁, …]` is ascending |
| `verify_semilattice_axioms(samples)` | Check all 4 axioms on samples |

### Domain Theory (`kleene`, `scott`)

| Function / Type | Description |
|---|---|
| `kleene_fixpoint(f, max_iter)` | Iterate `f` from `⊥` until stable |
| `AscendingChain` | Build and verify ascending chains |
| `verify_fixpoint(f, max_iter)` | Returns `Some(x)` if fixpoint found |
| `DirectedSet` | Verify directedness, compute supremum |
| `verify_scott_continuous(directed, f)` | Check `f(⊔D) = ⊔{f(d) \| d ∈ D}` |
| `Dcpo` | Directed-complete partial order with bottom |

### Anti-Entropy (`anti_entropy`)

| Type | Description |
|---|---|
| `Replica<L>` | A replica with state and version |
| `GossipNetwork<L>` | N replicas, gossip rounds until convergence |
| `MerkleNode` | Merkle tree for efficient state diffing |
| `MerkleSync` | Diff local vs remote Merkle trees |

### Composition (`compose`)

| Type | Description |
|---|---|
| `ProductCrdt<A, B>` | Parallel composition (pointwise join) |
| `Coproduct<A, B>` | Tagged union (same-tag joins, mixed → ⊥) |
| `FunctionCrdt<K, V>` | K → V with pointwise value join |
| `ListCrdt<T>` | Timestamped sequence CRDT |

### CUDAclaw Application (`cudaclaw`)

| Type | Description |
|---|---|
| `SmartCRDT` | Heartbeats + tasks + versions for agent fleets |
| `AgentNode<L>` | Single agent with local semilattice state |
| `verify_crdt(name, samples, f)` | Full formal verification pipeline |

---

## How It Works

### The Convergence Guarantee

Every state-based CRDT in this crate implements `BoundedJoinSemilattice`, which requires:

1. **Commutativity**: `a.join(b) == b.join(a)`
2. **Associativity**: `a.join(b).join(c) == a.join(b.join(c))`
3. **Idempotency**: `a.join(a) == a`
4. **Identity**: `a.join(bottom()) == a`

These four properties guarantee that any number of replicas can merge in any order and always converge to the same state.

### CALM Theorem Pipeline

1. Define your state as a `BoundedJoinSemilattice`.
2. Define your update function `f: L → L`.
3. Call `calm_analysis(samples, &f, "name")` — it checks monotonicity on sample points.
4. If monotone: the computation is coordination-free. No consensus needed.

### Delta Propagation

Instead of shipping the full CRDT state, delta-state CRDTs track only changes since the last extraction:

```
replica.inc();           // Accumulates delta
let delta = replica.extract_delta();  // Returns only changes, resets internal delta
remote.merge_delta(&delta);           // Applies delta to remote
```

### Gossip Anti-Entropy

The `GossipNetwork` simulates epidemic-style state propagation:

1. Each round, every replica merges with all others.
2. After O(log N) rounds, all replicas converge.
3. Merkle trees enable efficient diffing — only divergent keys are exchanged.

---

## The Math

### Join-Semilattice

A **join-semilattice** `(S, ⊔, ⊥)` is a set with a binary operation satisfying:

| Axiom | Law |
|-------|-----|
| Commutativity | `a ⊔ b = b ⊔ a` |
| Associativity | `(a ⊔ b) ⊔ c = a ⊔ (b ⊔ c)` |
| Idempotency | `a ⊔ a = a` |
| Identity | `a ⊔ ⊥ = a` |

The partial order is defined by: `a ⊑ b ⟺ a ⊔ b = b`.

### CRDT Convergence

**Theorem**: If all replicas apply the same set of updates and merge via `join`, they converge to `⊔ {all states}` regardless of merge order.

*Proof*: By associativity and commutativity, `a ⊔ b ⊔ c` is well-defined regardless of parenthesization or order. By idempotency, merging the same state twice is a no-op. ∎

### CALM Theorem

**Theorem** (Alvaro et al., 2011): A distributed computation has a *coordination-free* implementation if and only if it is monotone with respect to the lattice of input facts.

Formally: `f` is coordination-free ⟺ `a ⊑ b ⟹ f(a) ⊑ f(b)`.

### Kleene Fixpoint Theorem

If `f` is Scott-continuous on a DCPO with bottom, then the least fixpoint is:

```
μf = ⊔ₙ fⁿ(⊥)
```

This crate computes this by iterating `f` from `⊥` until stabilization.

### Scott Continuity

A function `f` is **Scott-continuous** if it preserves directed suprema:

```
f(⊔ D) = ⊔ { f(d) | d ∈ D }
```

This is verified numerically by the `verify_scott_continuous` function on sample directed sets.

---

## Testing

**79 tests** covering:
- Semilattice axiom verification
- All CRDT types: construction, merge, idempotency, convergence
- CALM monotonicity (positive and negative cases)
- Kleene fixpoints and ascending chains
- Scott continuity verification
- Gossip convergence
- Delta propagation
- CRDT composition
- CUDAclaw SmartCRDT formal verification

---

## License

MIT
