//! Polynomial W2 search experiment over independently supplied exact costs.
//! No production wrapper or Unicode boundary implementation is enabled.
#![cfg(feature = "native")]
#[derive(Clone, Debug, PartialEq)]
struct Plan {
    widths: Vec<f64>,
    ends: Vec<usize>,
    cost: f64,
}
fn better(a: &Plan, b: &Plan, mode: u8) -> bool {
    a.ends
        .len()
        .cmp(&b.ends.len())
        .then_with(|| a.cost.total_cmp(&b.cost))
        .then_with(|| {
            let order = a
                .widths
                .iter()
                .zip(&b.widths)
                .map(|(a, b)| a.total_cmp(b))
                .find(|c| !c.is_eq())
                .unwrap_or(std::cmp::Ordering::Equal);
            if mode == 0 { order.reverse() } else { order }
        })
        .then_with(|| a.ends.cmp(&b.ends))
        .is_lt()
}
fn search(costs: &[Vec<f64>], width: f64, mode: u8) -> Plan {
    let n = costs.len();
    let mut suffix: Vec<Option<Plan>> = vec![None; n + 1];
    suffix[n] = Some(Plan {
        widths: Vec::new(),
        ends: Vec::new(),
        cost: 0.0,
    });
    for start in (0..n).rev() {
        for end in start + 1..=n {
            let w = costs[start][end];
            if w > width && end != start + 1 {
                continue;
            }
            let tail = suffix[end].as_ref().unwrap();
            let mut widths = vec![w];
            widths.extend_from_slice(&tail.widths);
            let mut ends = vec![end];
            ends.extend_from_slice(&tail.ends);
            let candidate = Plan {
                widths,
                ends,
                cost: (width - w).max(0.0).powi(2) + tail.cost,
            };
            if suffix[start]
                .as_ref()
                .is_none_or(|old| better(&candidate, old, mode))
            {
                suffix[start] = Some(candidate);
            }
        }
    }
    suffix[0].take().unwrap()
}
#[test]
fn dynamic_programming_matches_exhaustive_small_nonmonotonic_costs() {
    let mut seed = 0x12345678_u64;
    for n in 1..=8 {
        for _ in 0..100 {
            let mut costs = vec![vec![0.0; n + 1]; n];
            for row in &mut costs {
                for w in row {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    *w = ((seed >> 32) % 40) as f64 * 0.25;
                }
            }
            for mode in [0, 3] {
                let mut best = None;
                for mask in 0..1_usize << (n - 1) {
                    let mut ends: Vec<_> = (1..n).filter(|i| mask & (1 << (i - 1)) != 0).collect();
                    ends.push(n);
                    let mut start = 0;
                    let mut widths = Vec::new();
                    let mut admissible = true;
                    for &end in &ends {
                        let w = costs[start][end];
                        admissible &= w <= 5.0 || end == start + 1;
                        widths.push(w);
                        start = end;
                    }
                    if !admissible {
                        continue;
                    }
                    let cost = widths
                        .iter()
                        .rev()
                        .fold(0.0, |sum, w| (5.0 - w).max(0.0).powi(2) + sum);
                    let plan = Plan { widths, ends, cost };
                    if best.as_ref().is_none_or(|old| better(&plan, old, mode)) {
                        best = Some(plan);
                    }
                }
                assert_eq!(search(&costs, 5.0, mode), best.unwrap());
            }
        }
    }
}
#[test]
fn mode_one_must_scan_past_nonfitting_intermediate_widths() {
    let widths = [3.0, 8.0, 4.0, 9.0];
    assert_eq!(widths.iter().rposition(|&w| w <= 5.0), Some(2));
}

#[test]
fn recursive_suffix_choice_is_not_reopened_when_prefix_rounding_erases_costs() {
    let width = 16_384.0_f64;
    let delta = 1.0 / 32_768.0;
    let prefix = width * width;
    let small = delta * delta;
    let large = 4.0 * small;
    assert!(small < large);
    assert_eq!((prefix + small).to_bits(), (prefix + large).to_bits());
    for (mode, a, b) in [
        (0, [width - delta, width], [width, width - 2.0 * delta]),
        (3, [width, width - delta], [width - 2.0 * delta, width]),
    ] {
        let mut costs = vec![vec![width + 1.0; 5]; 4];
        costs[0][1] = 0.0;
        costs[1][2] = a[0];
        costs[2][4] = a[1];
        costs[1][3] = b[0];
        costs[3][4] = b[1];
        let chosen = search(&costs, width, mode);
        assert_eq!(chosen.ends, [1, 2, 4]);
        assert_eq!(chosen.widths, [0.0, a[0], a[1]]);
        assert_eq!(chosen.cost, prefix);
        let discarded_suffix = Plan {
            widths: vec![0.0, b[0], b[1]],
            ends: vec![1, 3, 4],
            cost: prefix + large,
        };
        // A global comparison of all rounded complete plans would reopen the
        // discarded suffix and prefer it lexicographically. Recursive DP must
        // retain its locally lower-cost suffix, for either opposite tie rule.
        assert!(better(&discarded_suffix, &chosen, mode));
    }
}

#[test]
fn recursive_binary64_score_does_not_claim_exact_global_squared_slack() {
    let width = 16_384.0_f64;
    let delta = 1.0 / 32_768.0;
    let prefix = width * width;
    assert_eq!((prefix + delta * delta).to_bits(), prefix.to_bits());
    let mut costs = vec![vec![width + 1.0; 4]; 3];
    costs[0][1] = 0.0;
    costs[1][3] = width - delta;
    costs[0][2] = 0.0;
    costs[2][3] = width;
    // Exact dyadic scores are prefix+2^-30 and prefix. Rounded scores tie,
    // so mode3 selects the smaller second width despite its larger exact cost.
    let mode3 = search(&costs, width, 3);
    assert_eq!(mode3.ends, [1, 3]);
    assert_eq!(mode3.cost, prefix);
    assert_eq!(search(&costs, width, 0).ends, [2, 3]);
}
