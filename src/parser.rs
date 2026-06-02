use crate::netlist::{Gate, GateType, Netlist};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// Parses a Verilog file and constructs a Netlist struct.
pub fn verilog_to_netlist(filename: &str) -> Netlist {
    let path = Path::new(filename);
    let file = File::open(path).expect("Impossibile aprire file");
    let reader = io::BufReader::new(file);
    let mut netlist = Netlist::new();

    // Read all lines to allow multi-line accumulation for inputs/outputs
    let lines: Vec<String> = reader.lines().filter_map(|r| r.ok()).collect();
    let mut i: usize = 0;
    while i < lines.len() {
        let line = &lines[i];
        let mut trimmed = line.trim().to_string();

        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("module")
            || trimmed.starts_with("endmodule")
        {
            i += 1;
            continue;
        }

        // Accumulate multi-line input declarations ending with ';'
        if trimmed.starts_with("input") {
            let mut acc = trimmed.trim_start_matches("input").trim().to_string();
            while !acc.contains(';') && i + 1 < lines.len() {
                i += 1;
                acc.push_str(lines[i].trim());
            }
            // remove trailing semicolon if present
            if let Some(pos) = acc.rfind(';') {
                acc.truncate(pos);
            }
            for tok in acc.split(',') {
                let s = tok.trim();
                if !s.is_empty() {
                    add_bus_or_scalar(&mut netlist.inputs, s);
                }
            }
            i += 1;
            continue;
        }

        // Accumulate multi-line output declarations
        if trimmed.starts_with("output") {
            let mut acc = trimmed.trim_start_matches("output").trim().to_string();
            while !acc.contains(';') && i + 1 < lines.len() {
                i += 1;
                acc.push_str(lines[i].trim());
            }
            if let Some(pos) = acc.rfind(';') {
                acc.truncate(pos);
            }
            for tok in acc.split(',') {
                let s = tok.trim();
                if !s.is_empty() {
                    add_bus_or_scalar(&mut netlist.outputs, s);
                }
            }
            i += 1;
            continue;
        }
        // Handle simple continuous assignments: assign out = in;
        if trimmed.starts_with("assign") {
            if let Some(eq) = trimmed.find('=') {
                let left = trimmed[6..eq].trim().trim_end_matches(';').to_string();
                let right = trimmed[eq + 1..].trim().trim_end_matches(';').to_string();
                // treat as a buffer (wire connection)
                let gate = Gate {
                    name: left.clone(),
                    gate_type: GateType::BUF,
                    outputs: vec![left.clone()],
                    inputs: vec![right.clone()],
                };
                netlist.gates.push(gate);
            }
            i += 1;
            continue;
        }

        // Parse gate/module instantiations (positional or named-port)
        if let (Some(start), Some(end)) = (trimmed.find('('), trimmed.rfind(')')) {
            if start >= end {
                i += 1;
                continue;
            }
            // extract the token(s) before the '(' -> typically "TYPE INSTNAME"
            let pre = trimmed[..start].trim();
            let mut pre_parts = pre.split_whitespace();
            let type_token = pre_parts.next().unwrap_or("");
            let inst_name = pre_parts.next().map(|s| s.to_string());

            let gate_type = map_inst_to_gatetype(type_token);

            let inside = &trimmed[start + 1..end];
            let mut positional: Vec<String> = Vec::new();
            let mut named_outputs: Vec<String> = Vec::new();
            let mut named_inputs: Vec<String> = Vec::new();

            for tok in split_top_level(inside) {
                let t = tok.trim().trim_end_matches(';').trim();
                if t.is_empty() {
                    continue;
                }
                if t.starts_with('.') {
                    // named port: .PIN(net)
                    if let Some(popen) = t.find('(') {
                        if let Some(pclose) = t.rfind(')') {
                            if popen < pclose && popen > 0 {
                                let pin = t[1..popen].trim();
                                let net = t[popen + 1..pclose].trim();
                                if is_output_pin(pin) {
                                    named_outputs.push(net.to_string());
                                } else {
                                    named_inputs.push(net.to_string());
                                }
                            } else {
                                // malformed token, skip
                                continue;
                            }
                        }
                    }
                } else {
                    // positional connection
                    // may be like n310 or rs1_to_sc[0]
                    positional.push(t.to_string());
                }
            }

            let (outputs, inputs) = if !named_outputs.is_empty() || !named_inputs.is_empty() {
                (named_outputs, named_inputs)
            } else if !positional.is_empty() {
                let out = vec![positional[0].clone()];
                let ins = if positional.len() > 1 {
                    positional[1..].to_vec()
                } else {
                    Vec::new()
                };
                (out, ins)
            } else {
                (Vec::new(), Vec::new())
            };

            if !outputs.is_empty() || !inputs.is_empty() {
                let gate_name = if let Some(n) = inst_name { n } else { outputs.get(0).cloned().unwrap_or_else(|| type_token.to_string()) };
                let gate = Gate {
                    name: gate_name,
                    gate_type,
                    outputs,
                    inputs,
                };
                netlist.gates.push(gate);
            }
        }

        i += 1;
    }

    netlist
}

// Add either a scalar net or expand a bus declaration like "[3:0] name" into
// individual indexed nets `name[3]..name[0]`.
fn add_bus_or_scalar(vec: &mut Vec<String>, s: &str) {
    let token = s.trim();
    if token.is_empty() {
        return;
    }

    // Case: "[MSB:LSB] name"
    if token.starts_with('[') {
        if let Some(rb) = token.find(']') {
            let range = &token[1..rb];
            let rest = token[rb + 1..].trim();
            if let Some(colon) = range.find(':') {
                if let (Ok(msb), Ok(lsb)) = (range[..colon].trim().parse::<i32>(), range[colon + 1..].trim().parse::<i32>()) {
                    let mut hi = msb;
                    let lo = lsb;
                    while hi >= lo {
                        vec.push(format!("{}[{}]", rest, hi));
                        hi -= 1;
                    }
                    return;
                }
            }
            // fallback: treat as scalar name
            if !rest.is_empty() {
                vec.push(rest.to_string());
                return;
            }
        }
    }

    // Case: "name[MSB:LSB]"
    if let Some(lb) = token.find('[') {
        if let Some(rb) = token.rfind(']') {
            let name = token[..lb].trim();
            let range = &token[lb + 1..rb];
            if let Some(colon) = range.find(':') {
                if let (Ok(msb), Ok(lsb)) = (range[..colon].trim().parse::<i32>(), range[colon + 1..].trim().parse::<i32>()) {
                    let mut hi = msb;
                    let lo = lsb;
                    while hi >= lo {
                        vec.push(format!("{}[{}]", name, hi));
                        hi -= 1;
                    }
                    return;
                }
            }
        }
    }

    // Simple scalar or already-indexed net
    vec.push(token.to_string());
}

// Heuristic mapping from instance cell name to basic GateType
fn map_inst_to_gatetype(s: &str) -> GateType {
    let up = s.to_uppercase();
    if up.contains("XNOR") {
        GateType::XNOR
    } else if up.contains("XOR") {
        GateType::XOR
    } else if up.contains("NAND") {
        GateType::NAND
    } else if up.contains("NOR") {
        GateType::NOR
    } else if up.contains("AND") {
        GateType::AND
    } else if up.contains("OR") {
        GateType::OR
    } else if up.contains("INV") || up == "NOT" {
        GateType::NOT
    } else if up.contains("BUF") || up.contains("CLKBUF") || up.contains("BUFF") {
        GateType::BUF
    } else {
        // try direct mapping of simple names
        GateType::from_str(s)
    }
}

// Heuristic to determine if a named pin is an output pin
fn is_output_pin(pin: &str) -> bool {
    let p = pin.to_uppercase();
    p.starts_with('Z') || p == "Q" || p.starts_with('Y') || p.contains("OUT") || p.starts_with("CO") || p.starts_with('S')
}

// Split a comma-separated list at top level, ignoring commas inside (), {}, []
fn split_top_level(s: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut depth_paren = 0i32;
    let mut depth_brace = 0i32;
    let mut depth_brack = 0i32;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => if depth_paren > 0 { depth_paren -= 1 },
            '{' => depth_brace += 1,
            '}' => if depth_brace > 0 { depth_brace -= 1 },
            '[' => depth_brack += 1,
            ']' => if depth_brack > 0 { depth_brack -= 1 },
            ',' if depth_paren == 0 && depth_brace == 0 && depth_brack == 0 => {
                parts.push(s[start..i].to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(s[start..].to_string());
    }
    parts
}
