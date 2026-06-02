use commutations_maximizer::dag::Dag;
use commutations_maximizer::netlist::{Gate, GateType, Netlist};
use commutations_maximizer::pattern_generator::InputPattern;
use commutations_maximizer::ppsfp::PPSFPSimulator;

fn brute_force_most_transitions(values: &std::collections::HashMap<String, u32>) -> ((usize, usize), u32) {
    let mut keys: Vec<&String> = values.keys().collect();
    keys.sort();
    let mut counts = vec![vec![0u32; 32]; 32];
    for name in keys.iter() {
        let val = *values.get(*name).unwrap_or(&0u32);
        for p in 0..32 {
            let bp = (val >> p) & 1;
            for q in (p + 1)..32 {
                let bq = (val >> q) & 1;
                if bp != bq {
                    counts[p][q] += 1;
                }
            }
        }
    }
    let mut best_p = 0usize;
    let mut best_q = 0usize;
    let mut best_count = 0u32;
    for p in 0..32 {
        for q in (p + 1)..32 {
            let c = counts[p][q];
            if c > best_count {
                best_count = c;
                best_p = p;
                best_q = q;
            }
        }
    }
    ((best_p, best_q), best_count)
}

fn build_simple_netlist_with_gate(num_inputs: usize) -> Netlist {
    let mut nl = Netlist::new();
    for i in 0..num_inputs {
        nl.inputs.push(format!("in{}", i));
    }
    // add a single gate to ensure Dag.topo is non-empty
    let g = Gate {
        name: "g0".to_string(),
        gate_type: GateType::BUF,
        outputs: vec!["out0".to_string()],
        inputs: vec!["in0".to_string()],
    };
    nl.gates.push(g);
    nl.outputs.push("out0".to_string());
    nl
}

#[test]
fn test_bruteforce_small() {
    let nl = build_simple_netlist_with_gate(10);
    let dag = Dag::from_netlist(&nl);
    let mut sim = PPSFPSimulator::new(&dag);

    let mut pat = InputPattern::from_strings((0..10).map(|i| format!("in{}", i)).collect());
    // set deterministic values
    for i in 0..10 {
        pat.set(&format!("in{}", i), (i as u32) * 0x11111111u32);
    }

    let ((p, q), count) = sim.most_transitions_for_compact_pattern(&pat);
    let expected = brute_force_most_transitions(&sim.good_map_clone());
    assert_eq!(((p, q), count), expected);
}

#[test]
fn test_bruteforce_large() {
    // large number of keys to hit the bitset path (s > 128)
    let num_inputs = 200usize;
    let nl = build_simple_netlist_with_gate(num_inputs);
    let dag = Dag::from_netlist(&nl);
    let mut sim = PPSFPSimulator::new(&dag);

    let mut keys: Vec<String> = (0..num_inputs).map(|i| format!("in{}", i)).collect();
    let mut pat = InputPattern::from_strings(keys.clone());
    // populate with pseudo-random but deterministic values
    for (i, k) in keys.iter().enumerate() {
        let val = (i as u32).wrapping_mul(0x9E3779B9u32).rotate_left((i % 31) as u32);
        pat.set(k, val);
    }

    let ((p, q), count) = sim.most_transitions_for_compact_pattern(&pat);
    let expected = brute_force_most_transitions(&sim.good_map_clone());
    assert_eq!(((p, q), count), expected);
}

#[test]
fn test_empty_topo_or_good() {
    // empty dag
    let nl = Netlist::new();
    let dag = Dag::from_netlist(&nl);
    let mut sim = PPSFPSimulator::new(&dag);
    let pat = InputPattern::new();
    let ((p, q), count) = sim.most_transitions_for_compact_pattern(&pat);
    assert_eq!(((p, q), count), ((0usize, 0usize), 0u32));
}
