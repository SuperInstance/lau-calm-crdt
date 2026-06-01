//! # lau-calm-crdt
//!
//! CALM theorem and CRDT join-semilattice theory — formal convergence guarantees
//! for distributed agent state.
//!
//! ## Core Concepts
//!
//! - **CALM Theorem**: A distributed computation has a coordination-free
//!   implementation iff it is monotone on a lattice.
//! - **CRDTs**: Converge because their merge operation is a join in a bounded
//!   join-semilattice.
//! - **Delta-state CRDTs**: Efficient delta propagation avoiding full-state shipping.
//! - **Kleene fixpoints**: Denotational semantics for convergent CRDT states.
//! - **Scott continuity**: Continuous functions on directed-complete partial orders.

pub mod semilattice;
pub mod calm;
pub mod crdt_state;
pub mod crdt_op;
pub mod delta;
pub mod kleene;
pub mod scott;
pub mod anti_entropy;
pub mod compose;
pub mod cudaclaw;
