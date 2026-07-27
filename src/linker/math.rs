use super::Candidate;
use crate::SemanticVector;

pub(crate) fn ordered_vectors(vectors: &[SemanticVector]) -> Vec<&SemanticVector> {
    let mut ordered = vectors.iter().collect::<Vec<_>>();
    ordered.sort_unstable_by(|left, right| left.node_id().cmp(right.node_id()));
    ordered
}

pub(crate) fn cosine(left: &SemanticVector, right: &SemanticVector) -> f64 {
    let dot = left
        .values()
        .iter()
        .zip(right.values())
        .map(|(&left, &right)| f64::from(left) * f64::from(right))
        .sum::<f64>();
    (dot / (left.norm() * right.norm())).clamp(-1.0, 1.0)
}

pub(crate) fn retain_top_k(candidates: &mut Vec<Candidate>, candidate: Candidate, top_k: usize) {
    if candidates.len() == top_k
        && !candidate_is_better(&candidate, candidates.last().expect("top_k is positive"))
    {
        return;
    }
    let position = candidates
        .binary_search_by(|existing| compare_candidates(existing, &candidate))
        .unwrap_or_else(|position| position);
    candidates.insert(position, candidate);
    if candidates.len() > top_k {
        candidates.pop();
    }
}

fn compare_candidates(left: &Candidate, right: &Candidate) -> std::cmp::Ordering {
    right
        .score
        .total_cmp(&left.score)
        .then_with(|| left.target.cmp(&right.target))
}

fn candidate_is_better(candidate: &Candidate, existing: &Candidate) -> bool {
    match candidate.score.total_cmp(&existing.score) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => candidate.target < existing.target,
        std::cmp::Ordering::Less => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{Candidate, compare_candidates, retain_top_k};

    #[test]
    fn bounded_candidate_retention_matches_full_sort() {
        let input = [
            Candidate {
                target: 7,
                score: 0.80,
            },
            Candidate {
                target: 4,
                score: 0.95,
            },
            Candidate {
                target: 2,
                score: 0.95,
            },
            Candidate {
                target: 8,
                score: 0.75,
            },
            Candidate {
                target: 1,
                score: 0.90,
            },
            Candidate {
                target: 3,
                score: 0.90,
            },
        ];
        for top_k in 1..=input.len() {
            let mut expected = input.to_vec();
            expected.sort_unstable_by(compare_candidates);
            expected.truncate(top_k);
            let mut actual = Vec::new();
            for candidate in input {
                retain_top_k(&mut actual, candidate, top_k);
            }
            assert_eq!(actual, expected);
            let mut reversed = Vec::new();
            for candidate in input.into_iter().rev() {
                retain_top_k(&mut reversed, candidate, top_k);
            }
            assert_eq!(reversed, expected);
        }
    }
}
