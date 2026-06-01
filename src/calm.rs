//! CALM theorem verification.

use crate::semilattice::BoundedJoinSemilattice;

/// Result of a CALM analysis.
#[derive(Debug, Clone)]
pub struct CalmAnalysis {
    pub is_monotone: bool,
    pub is_coordination_free: bool,
    pub explanation: String,
}

/// Verify that a function f: L → L is monotone on sample points.
pub fn verify_monotone<L, F>(samples: &[L], f: &F) -> bool
where L: BoundedJoinSemilattice, F: Fn(&L) -> L {
    for a in samples {
        for b in samples {
            if a.leq(b) {
                if !f(a).leq(&f(b)) { return false; }
            }
        }
    }
    true
}

/// Perform a full CALM analysis on a function given sample points.
pub fn calm_analysis<L, F>(samples: &[L], f: &F, name: &str) -> CalmAnalysis
where L: BoundedJoinSemilattice, F: Fn(&L) -> L {
    let is_monotone = verify_monotone(samples, f);
    CalmAnalysis {
        is_monotone,
        is_coordination_free: is_monotone,
        explanation: if is_monotone {
            format!("Function '{}' IS monotone → coordination-free per CALM theorem.", name)
        } else {
            format!("Function '{}' is NOT monotone → requires coordination per CALM theorem.", name)
        },
    }
}

/// Check that a sequence is monotonically ascending.
pub fn verify_monotone_sequence<L>(sequence: &[L]) -> bool
where L: BoundedJoinSemilattice {
    for window in sequence.windows(2) {
        if !window[0].leq(&window[1]) { return false; }
    }
    true
}

/// Verify semilattice axioms on sample points.
pub fn verify_semilattice_axioms<L>(samples: &[L]) -> bool
where L: BoundedJoinSemilattice {
    for a in samples { if a.join(&L::bottom()) != *a { return false; } }
    for a in samples { if a.join(a) != *a { return false; } }
    for a in samples { for b in samples { if a.join(b) != b.join(a) { return false; } } }
    for a in samples {
        for b in samples {
            for c in samples { if a.join(&b.join(c)) != a.join(b).join(c) { return false; } }
        }
    }
    true
}

/// Decidability of monotonicity checking.
#[derive(Debug, Clone, Copy)]
pub enum Decidability { Decidable, SemiDecidable }

impl Decidability {
    pub fn is_decidable(&self) -> bool { matches!(self, Decidability::Decidable) }
}

pub fn decidability<L>(samples: &[L], claimed_total: Option<usize>) -> Decidability
where L: BoundedJoinSemilattice {
    if let Some(total) = claimed_total {
        if samples.len() >= total { return Decidability::Decidable; }
    }
    Decidability::SemiDecidable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semilattice::{BoolOr, NatMax};

    #[test]
    fn monotone_function_detected() {
        let samples = vec![NatMax(0), NatMax(1), NatMax(2), NatMax(3), NatMax(10)];
        assert!(verify_monotone(&samples, &|x: &NatMax| NatMax(x.0 + 1)));
    }

    #[test]
    fn non_monotone_function_detected() {
        let samples = vec![BoolOr(false), BoolOr(true)];
        assert!(!verify_monotone(&samples, &|x: &BoolOr| BoolOr(!x.0)));
    }

    #[test]
    fn calm_analysis_monotone() {
        let samples = vec![NatMax(0), NatMax(5), NatMax(10)];
        let a = calm_analysis(&samples, &|x: &NatMax| NatMax(x.0 * 2), "double");
        assert!(a.is_monotone); assert!(a.is_coordination_free);
    }

    #[test]
    fn calm_analysis_non_monotone() {
        let samples = vec![BoolOr(false), BoolOr(true)];
        let a = calm_analysis(&samples, &|x: &BoolOr| BoolOr(!x.0), "not");
        assert!(!a.is_monotone); assert!(!a.is_coordination_free);
    }

    #[test]
    fn monotone_sequence_valid() {
        assert!(verify_monotone_sequence(&[NatMax(0), NatMax(1), NatMax(3), NatMax(10)]));
    }

    #[test]
    fn monotone_sequence_invalid() {
        assert!(!verify_monotone_sequence(&[NatMax(5), NatMax(3)]));
    }

    #[test]
    fn semilattice_axioms_hold() {
        let samples = vec![NatMax(0), NatMax(1), NatMax(5), NatMax(10)];
        assert!(verify_semilattice_axioms(&samples));
    }

    #[test]
    fn decidability_finite() {
        let samples = vec![BoolOr(false), BoolOr(true)];
        assert!(decidability(&samples, Some(2)).is_decidable());
    }

    #[test]
    fn decidability_infinite() {
        let samples = vec![NatMax(0), NatMax(1)];
        assert!(!decidability(&samples, None).is_decidable());
    }
}
