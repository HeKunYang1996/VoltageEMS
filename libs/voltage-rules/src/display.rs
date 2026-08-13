//! Execution Display Builder
//!
//! Generates a human-readable `display` layer from a raw `RuleExecutionResult`
//! for storage in `rule_history.execution_result`.  The enrichment is
//! best-effort: if a piece of metadata (instance name, point name, flow label)
//! is unavailable we fall back to the raw numeric ID so callers always receive
//! a complete object.
//!
//! # Data sources
//! | Need               | Source                                     |
//! |--------------------|---------------------------------------------|
//! | Node label / type  | `flow_json` column in `rules` table         |
//! | Branch name        | `nodes_json` → `RuleSwitchBranch.name`      |
//! | Variable bindings  | `nodes_json` → `RuleVariable`              |
//! | Instance name      | `instances` table (SQLite, async query)     |
//! | Point name / unit  | `voltage-model` compile-time product lib    |
//! | Node execution detail (condition results, snapshots) | `RuleExecutionResult.node_details` |
//!
//! # Design
//! Every variable that appears anywhere in the flow (Switch condition inputs,
//! ChangeValue targets/sources, Calculation inputs/outputs, PeriodDelta
//! input/output) is resolved once into a `VarInfo` (device instance name +
//! point name + unit + value snapshot). That single lookup table is reused
//! everywhere text is generated (`trigger_reason`, per-step condition/formula
//! text, action descriptions) so the "which device/point was this" story is
//! always consistent and always includes the owning instance — never a bare
//! point name.

use crate::executor::{ActionResult, ConditionResult, NodeExecutionDetail, RuleExecutionResult};
use crate::types::{RuleFlow, RuleNode, RuleVariable};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use voltage_model::product_lib::get_builtin_product;

// ─────────────────────────────────────────────────────────────────────────────
// Internal types
// ─────────────────────────────────────────────────────────────────────────────

struct FlowNodeInfo {
    /// User-visible label (e.g. "SOC Branch")
    label: String,
    /// Business type (e.g. "function-switch")
    node_type: String,
}

struct InstanceInfo {
    name: String,
    product_name: String,
}

/// Fully-resolved description of one rule variable: which device/point it
/// binds to, and the value snapshot captured during this execution.
#[derive(Clone)]
struct VarInfo {
    /// Point name if resolved, else falls back to the raw variable name (X1, Y1, ...)
    label: String,
    instance_id: Option<u32>,
    instance_name: String,
    point_id: Option<u32>,
    point_name: String,
    unit: String,
    value: Option<f64>,
    /// For "combined" variables (`RuleVariable.formula` non-empty, e.g.
    /// `X3 = X1 + X2`): the formula with every operand already substituted
    /// for its own "instance·point (value)" text, so a condition on X3
    /// doesn't collapse to a bare "X3 (301)" with no way to see where 301
    /// came from.
    formula_resolved: Option<String>,
}

impl VarInfo {
    /// "{instance}·{point} (value unit)" — falls back gracefully when any
    /// piece of metadata is missing. For combined variables, expands to
    /// "{name}[{formula with operands resolved}] (value unit)" instead of
    /// hiding the computation behind the bare variable name.
    fn describe(&self) -> String {
        let target = match &self.formula_resolved {
            Some(formula) => format!("{}[{}]", self.label, formula),
            None if self.instance_name.is_empty() => self.label.clone(),
            None => format!("{}·{}", self.instance_name, self.label),
        };
        match self.value {
            Some(v) => {
                let val_str = if self.unit.is_empty() {
                    format!("{}", v)
                } else {
                    format!("{} {}", v, self.unit)
                };
                format!("{} ({})", target, val_str)
            },
            None => target,
        }
    }

    fn to_json(&self, key: &str) -> Value {
        json!({
            "key": key,
            "label": self.label,
            "value": self.value,
            "unit": self.unit,
            "instance_id": self.instance_id,
            "instance_name": self.instance_name,
            "point_id": self.point_id,
            "point_name": self.point_name,
            "formula_resolved": self.formula_resolved,
        })
    }
}

/// Reconstruct a readable expression string from formula tokens
/// (`RuleVariable.formula`, e.g. `["X1", "+", "X2"]` → `"X1 + X2"`).
/// Mirrors the token-joining half of `executor::evaluate_token_formula`
/// (evaluation itself stays in the executor; this is display-only).
fn formula_token_text(tokens: &[Value]) -> String {
    tokens
        .iter()
        .map(|t| match t {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Build a human-readable `display` JSON block for a rule execution.
///
/// Returns `Value::Null` when the `rules` row cannot be found.
pub async fn build_execution_display(
    pool: &SqlitePool,
    rule_id: i64,
    result: &RuleExecutionResult,
) -> Value {
    // ── 1. Load flow_json + nodes_json ──────────────────────────────────────
    let row: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT flow_json, nodes_json FROM rules WHERE id = ?")
            .bind(rule_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    let (flow_json_str, nodes_json_str) = row.unwrap_or((None, None));

    // ── 2. Parse flow_json → node label/type map ────────────────────────────
    let node_info_map = parse_node_info_map(flow_json_str.as_deref());

    // ── 3. Parse nodes_json → RuleFlow (variables + branch names) ──────────
    let rule_flow: Option<RuleFlow> = nodes_json_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    // Variable *definitions* (instance/point/formula binding) are collected
    // once across the whole flow — a variable's binding doesn't change
    // between nodes. Variable *values*, however, are read per-node from
    // `NodeExecutionDetail.input_values` (see step 7): the executor shares
    // one flat value map across the whole run, so by the time a later node
    // finishes, an earlier node's condition variable may already have been
    // overwritten. Using the final `result.variable_values` for text like
    // `trigger_reason` showed the *post-execution* value instead of the
    // value that was actually true when the condition fired.
    let var_defs: HashMap<String, RuleVariable> = collect_all_variables(rule_flow.as_ref())
        .into_iter()
        .map(|v| (v.name.clone(), v))
        .collect();

    // ── 4. Collect instance IDs needed for lookup ───────────────────────────
    let instance_ids: Vec<u32> = {
        let mut ids = HashSet::new();
        for v in var_defs.values() {
            if let Some(id) = v.instance {
                ids.insert(id);
            }
        }
        for a in &result.actions_executed {
            if a.target_type == "instance" {
                ids.insert(a.target_id);
            }
        }
        ids.into_iter().collect()
    };

    // ── 5. Query instances table ────────────────────────────────────────────
    let instance_map = query_instance_map(pool, &instance_ids).await;

    // ── 6. Top-level "variables" overview: final snapshot across the whole
    // run (debug aid, not used for trigger/condition text — see step 7).
    let final_var_map = build_var_info_map(&var_defs, &result.variable_values, &instance_map);
    let variable_display = build_variable_display(&final_var_map);

    // ── 7. Build sub-sections ───────────────────────────────────────────────
    let execution_steps = build_execution_steps(
        result,
        rule_flow.as_ref(),
        &node_info_map,
        &var_defs,
        &instance_map,
    );
    let execution_graph = build_execution_graph(result, &execution_steps);
    let action_display = build_action_display(&result.actions_executed, &instance_map);

    // ── 8. Assemble top-level display object ────────────────────────────────
    let summary = build_summary(result, &action_display);
    let trigger_reason = build_trigger_reason(
        result,
        rule_flow.as_ref(),
        &node_info_map,
        &var_defs,
        &instance_map,
    );

    json!({
        "summary": summary,
        "trigger_reason": trigger_reason,
        "variables": variable_display,
        "execution_steps": execution_steps,
        "execution_graph": execution_graph,
        "actions": action_display,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse Vue Flow `flow_json` into a map of `node_id → (label, type)`.
fn parse_node_info_map(flow_json_str: Option<&str>) -> HashMap<String, FlowNodeInfo> {
    let mut map = HashMap::new();

    let flow: Value = match flow_json_str.and_then(|s| serde_json::from_str(s).ok()) {
        Some(v) => v,
        None => return map,
    };
    let nodes = match flow.get("nodes").and_then(|n| n.as_array()) {
        Some(n) => n,
        None => return map,
    };

    for node in nodes {
        let id = match node.get("id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        // business type: data.type preferred, fall back to node-level type
        let node_type = node
            .get("data")
            .and_then(|d| d.get("type"))
            .and_then(|t| t.as_str())
            .or_else(|| node.get("type").and_then(|t| t.as_str()))
            .unwrap_or("")
            .to_string();

        // label: data.label if set, otherwise derive from type
        let label = node
            .get("data")
            .and_then(|d| d.get("label"))
            .and_then(|l| l.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| humanize_node_type(&node_type));

        map.insert(id, FlowNodeInfo { label, node_type });
    }
    map
}

fn humanize_node_type(t: &str) -> String {
    match t {
        "start" => "Start",
        "end" => "End",
        "function-switch" => "Condition",
        "action-changeValue" => "Set Value",
        "action-calculation" => "Calculation",
        "action-periodDelta" => "Period Delta",
        other => other,
    }
    .to_string()
}

/// Collect unique `RuleVariable` definitions from all nodes in a `RuleFlow`.
///
/// Covers every node type that carries variable bindings, including
/// `PeriodDelta`'s `input`/`output` (previously missing, which meant period
/// rules never got instance/point-qualified snapshots in `display`).
fn collect_all_variables(rule_flow: Option<&RuleFlow>) -> Vec<RuleVariable> {
    let mut vars: Vec<RuleVariable> = Vec::new();
    if let Some(flow) = rule_flow {
        for node in flow.nodes.values() {
            match node {
                RuleNode::Switch { variables, .. }
                | RuleNode::ChangeValue { variables, .. }
                | RuleNode::Calculation { variables, .. } => {
                    vars.extend_from_slice(variables);
                },
                RuleNode::PeriodDelta { input, output, .. } => {
                    vars.push(input.clone());
                    vars.push(output.clone());
                },
                RuleNode::Start { .. } | RuleNode::End => {},
            }
        }
        // Deduplicate by name
        vars.sort_by(|a, b| a.name.cmp(&b.name));
        vars.dedup_by(|a, b| a.name == b.name);
    }
    vars
}

/// Query `instance_id`, `instance_name`, `product_name` for each given ID.
async fn query_instance_map(pool: &SqlitePool, ids: &[u32]) -> HashMap<u32, InstanceInfo> {
    let mut map = HashMap::new();
    for &id in ids {
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT instance_name, product_name FROM instances WHERE instance_id = ?",
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((name, product_name)) = row {
            map.insert(id, InstanceInfo { name, product_name });
        }
    }
    map
}

/// Look up a point's name and unit from the product library.
///
/// `point_type` is the rule-engine convention: `"measurement"` / `"M"` / `"action"` / `"A"`.
fn lookup_point(
    product_name: &str,
    point_type: Option<&str>,
    point_id: u32,
) -> Option<(&'static str, &'static str)> {
    let product = get_builtin_product(product_name)?;
    let is_action = matches!(point_type, Some("action") | Some("A"));
    let points = if is_action {
        &product.actions
    } else {
        &product.measurements
    };
    let def = points.iter().find(|p| p.id == point_id)?;
    Some((def.name.as_str(), def.unit.as_str()))
}

/// Resolve every rule variable to its device/point/value against a specific
/// value snapshot. Callers pass either the final `result.variable_values`
/// (for the top-level overview) or a single node's
/// `NodeExecutionDetail.input_values` (for anything that describes "what
/// this node actually saw") — see call sites for which is correct where.
///
/// Two passes: plain point-bound variables first (their value comes
/// straight from `snapshot`), then "combined" variables (`formula`
/// non-empty, e.g. `X3 = X1 + X2`) whose `formula_resolved` text is built by
/// substituting the already-resolved base variables from pass one.
fn build_var_info_map(
    var_defs: &HashMap<String, RuleVariable>,
    snapshot: &HashMap<String, f64>,
    instance_map: &HashMap<u32, InstanceInfo>,
) -> HashMap<String, VarInfo> {
    let mut map = HashMap::new();

    for var in var_defs.values().filter(|v| v.formula.is_empty()) {
        let value = snapshot.get(&var.name).copied();

        let (instance_name, point_name, unit) = if let (Some(iid), Some(pid)) =
            (var.instance, var.point)
        {
            let info = instance_map.get(&iid);
            let (pname, u) = info
                .and_then(|inst| lookup_point(&inst.product_name, var.point_type.as_deref(), pid))
                .unwrap_or(("", ""));
            (info.map(|i| i.name.as_str()).unwrap_or(""), pname, u)
        } else {
            ("", "", "")
        };

        let label = if !point_name.is_empty() {
            point_name.to_string()
        } else {
            var.name.clone()
        };

        map.insert(
            var.name.clone(),
            VarInfo {
                label,
                instance_id: var.instance,
                instance_name: instance_name.to_string(),
                point_id: var.point,
                point_name: point_name.to_string(),
                unit: unit.to_string(),
                value,
                formula_resolved: None,
            },
        );
    }

    // Second pass: combined variables reference base variables resolved above.
    let base_substitution = build_substitution_map(&map);
    for var in var_defs.values().filter(|v| !v.formula.is_empty()) {
        let formula_text = formula_token_text(&var.formula);
        let formula_resolved = substitute_variable_tokens(&formula_text, &base_substitution);
        map.insert(
            var.name.clone(),
            VarInfo {
                label: var.name.clone(),
                instance_id: None,
                instance_name: String::new(),
                point_id: None,
                point_name: String::new(),
                unit: String::new(),
                value: snapshot.get(&var.name).copied(),
                formula_resolved: Some(formula_resolved),
            },
        );
    }

    map
}

/// Build a `variable_name → "instance·point (value unit)"` substitution map
/// for expression rendering, only for variables whose value is known.
fn build_substitution_map(var_map: &HashMap<String, VarInfo>) -> HashMap<String, String> {
    var_map
        .iter()
        .filter(|(_, info)| info.value.is_some())
        .map(|(name, info)| (name.clone(), info.describe()))
        .collect()
}

/// Replace bare variable tokens (`X1`, `Y1`, ...) in an expression with their
/// resolved "instance·point (value unit)" text, matching on whole
/// identifiers only (word-boundary safe — `X1` will not also match inside
/// `X11`, unlike a naive `str::replace`).
fn substitute_variable_tokens(expr: &str, substitution: &HashMap<String, String>) -> String {
    let chars: Vec<char> = expr.chars().collect();
    let mut out = String::with_capacity(expr.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let token: String = chars[start..i].iter().collect();
            match substitution.get(&token) {
                Some(replacement) => out.push_str(replacement),
                None => out.push_str(&token),
            }
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Section builders
// ─────────────────────────────────────────────────────────────────────────────

fn build_variable_display(var_map: &HashMap<String, VarInfo>) -> Vec<Value> {
    let mut keys: Vec<&String> = var_map.keys().collect();
    keys.sort();
    keys.into_iter()
        .filter(|k| var_map[*k].value.is_some())
        .map(|k| var_map[k].to_json(k))
        .collect()
}

fn build_execution_steps(
    result: &RuleExecutionResult,
    rule_flow: Option<&RuleFlow>,
    node_info_map: &HashMap<String, FlowNodeInfo>,
    var_defs: &HashMap<String, RuleVariable>,
    instance_map: &HashMap<u32, InstanceInfo>,
) -> Vec<Value> {
    result
        .execution_path
        .iter()
        .map(|node_id| {
            let info = node_info_map.get(node_id);
            let label = info
                .map(|n| n.label.as_str())
                .unwrap_or(node_id.as_str())
                .to_string();
            let node_type = info.map(|n| n.node_type.as_str()).unwrap_or("").to_string();

            let detail = result.node_details.get(node_id);
            let flow_node = rule_flow.and_then(|f| f.nodes.get(node_id));

            let mut step = json!({
                "node_id": node_id,
                "label": label,
                "type": node_type,
                "node_kind": detail.map(|d| d.node_type).unwrap_or(""),
            });

            if let (Some(detail), Some(flow_node)) = (detail, flow_node) {
                // `terminal` = this node ended its branch with nowhere further
                // to go (no Switch branch matched, or an action node has no
                // output wire). Normal flow control, not an error — surfaced
                // explicitly so the UI can say "this path ends here" instead
                // of leaving the viewer to guess why the trace stops.
                step["terminal"] = json!(detail.terminal);
                if detail.terminal {
                    step["terminal_reason"] = json!(match detail.node_type {
                        "switch" => "No branch condition matched — this path ends here",
                        _ => "No downstream node connected — this path ends here",
                    });
                }

                // Values *as this node saw them* — not the run's final
                // snapshot, which may have been overwritten by later nodes
                // reusing the same variable name for a different point.
                let local_map = build_var_info_map(var_defs, &detail.input_values, instance_map);
                let local_substitution = build_substitution_map(&local_map);

                match detail.node_type {
                    "switch" => {
                        enrich_switch_step(&mut step, detail, flow_node, &local_substitution)
                    },
                    "change" => enrich_change_step(&mut step, detail, flow_node, &local_map),
                    "calculation" => {
                        enrich_calculation_step(&mut step, detail, flow_node, &local_substitution)
                    },
                    "periodDelta" => {
                        enrich_period_delta_step(&mut step, detail, flow_node, instance_map)
                    },
                    _ => {},
                }
            }

            step
        })
        .collect()
}

/// Build the final graph-shaped display contract while retaining
/// `execution_steps` during the frontend migration.
///
/// Node presentation details come from the enriched legacy steps; status and
/// edges come from the executor's machine-readable trace. Redundant
/// `matched_port` / `matched_label` fields are intentionally omitted here:
/// the activated edge owns that information.
fn build_execution_graph(result: &RuleExecutionResult, steps: &[Value]) -> Value {
    let trace_nodes: HashMap<&str, &crate::executor::ExecutionGraphNode> = result
        .execution_graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();

    let nodes: Vec<Value> = steps
        .iter()
        .map(|step| {
            let mut node = step.clone();
            let id = node
                .get("node_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if let Some(object) = node.as_object_mut() {
                object.remove("node_id");
                object.remove("node_kind");
                object.remove("matched_port");
                object.remove("matched_label");
                object.insert("id".to_string(), json!(id));
            }

            if let Some(trace) = trace_nodes.get(id.as_str()) {
                // Graph `type` is the stable execution kind (`switch`,
                // `change`, ...), not the Vue editor component type
                // (`function-switch`, `action-changeValue`, ...).
                node["type"] = json!(trace.node_type);
                node["status"] = json!(trace.status);
                node["terminal"] = json!(trace.terminal);
                if let Some(kind) = trace.terminal_kind {
                    node["terminal_kind"] = json!(kind);
                } else if let Some(object) = node.as_object_mut() {
                    object.remove("terminal_kind");
                }
                if let Some(reason) = &trace.terminal_reason {
                    node["terminal_reason"] = json!(reason);
                } else if let Some(object) = node.as_object_mut() {
                    object.remove("terminal_reason");
                }
            }
            node
        })
        .collect();

    json!({
        "nodes": nodes,
        "edges": result.execution_graph.edges,
    })
}

/// Switch step: expose every branch's raw + device/point-resolved
/// expression, its true/false result, and which port actually fired —
/// i.e. "complete data" for the condition that really triggered the rule,
/// not just the winning expression string.
fn enrich_switch_step(
    step: &mut Value,
    detail: &NodeExecutionDetail,
    flow_node: &RuleNode,
    substitution: &HashMap<String, String>,
) {
    let RuleNode::Switch { rule: branches, .. } = flow_node else {
        return;
    };

    let conditions: Vec<Value> = detail
        .condition_results
        .as_ref()
        .map(|results| {
            results
                .iter()
                .map(|cr: &ConditionResult| {
                    json!({
                        "port": cr.port,
                        "expression": cr.expression,
                        "expression_resolved": substitute_variable_tokens(&cr.expression, substitution),
                        "result": cr.result,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    step["conditions"] = json!(conditions);
    if let Some(port) = &detail.matched_port {
        step["matched_port"] = json!(port);
        // Branch name doubles as its display label today (no separate
        // human label is preserved past Vue Flow → nodes_json compaction).
        if let Some(branch) = branches.iter().find(|b| &b.name == port) {
            step["matched_label"] = json!(branch.name);
        }
    }
}

/// ChangeValue step: for each assignment, show the write target
/// (instance·point) and where the value came from — a literal, or another
/// variable (in which case its resolved device/point/value is embedded too,
/// covering "复合值" assignments like `Y1 = X1`).
fn enrich_change_step(
    step: &mut Value,
    detail: &NodeExecutionDetail,
    flow_node: &RuleNode,
    var_map: &HashMap<String, VarInfo>,
) {
    let RuleNode::ChangeValue {
        rule: assignments, ..
    } = flow_node
    else {
        return;
    };

    let actions = detail.actions.as_deref().unwrap_or(&[]);
    let rows: Vec<Value> = assignments
        .iter()
        .enumerate()
        .map(|(idx, assignment)| {
            let target = var_map.get(&assignment.variables);
            let target_json = target
                .map(|t| t.to_json(&assignment.variables))
                .unwrap_or(Value::Null);

            let (value_source, source_variable) = match assignment.value.as_str() {
                Some(s) if var_map.contains_key(s) => {
                    ("variable", var_map.get(s).map(|v| v.to_json(s)))
                },
                Some(_) => ("literal", None),
                None => ("literal", None),
            };

            let action = actions.get(idx);
            json!({
                "target": target_json,
                "raw_value": assignment.value,
                "value_source": value_source,
                "source_variable": source_variable,
                "written_value": action.map(|a| a.value),
                "success": action.map(|a| a.success).unwrap_or(false),
            })
        })
        .collect();

    step["assignments"] = json!(rows);
}

/// Calculation step: original device/point names for every input, the raw
/// formula text, and the same formula with each variable substituted for
/// "instance·point (value unit)" — so a rule like `output = a + b * 2` reads
/// as `output = PackVoltage (380 V) + PackCurrent (12 A) * 2` in history.
fn enrich_calculation_step(
    step: &mut Value,
    detail: &NodeExecutionDetail,
    flow_node: &RuleNode,
    substitution: &HashMap<String, String>,
) {
    let RuleNode::Calculation {
        rule: calculations, ..
    } = flow_node
    else {
        return;
    };

    let actions = detail.actions.as_deref().unwrap_or(&[]);
    let rows: Vec<Value> = calculations
        .iter()
        .enumerate()
        .map(|(idx, calc)| {
            let action = actions.get(idx);
            json!({
                "output_variable": calc.output,
                "formula": calc.formula,
                "formula_resolved": substitute_variable_tokens(&calc.formula, substitution),
                "result": action.map(|a| a.value),
                "success": action.map(|a| a.success).unwrap_or(false),
            })
        })
        .collect();

    step["calculations"] = json!(rows);
}

/// PeriodDelta step: period type + input snapshot (the cumulative reading
/// the delta was computed from) + output write target + the computed delta.
fn enrich_period_delta_step(
    step: &mut Value,
    detail: &NodeExecutionDetail,
    flow_node: &RuleNode,
    instance_map: &HashMap<u32, InstanceInfo>,
) {
    let RuleNode::PeriodDelta {
        input,
        output,
        period,
        ..
    } = flow_node
    else {
        return;
    };

    let describe = |var: &RuleVariable, value: Option<f64>| -> Value {
        let (instance_name, point_name, unit) =
            if let (Some(iid), Some(pid)) = (var.instance, var.point) {
                let inst = instance_map.get(&iid);
                let (pname, u) = inst
                    .and_then(|i| lookup_point(&i.product_name, var.point_type.as_deref(), pid))
                    .unwrap_or(("", ""));
                (inst.map(|i| i.name.as_str()).unwrap_or(""), pname, u)
            } else {
                ("", "", "")
            };
        json!({
            "instance_id": var.instance,
            "instance_name": instance_name,
            "point_id": var.point,
            "point_name": if point_name.is_empty() { var.name.as_str() } else { point_name },
            "unit": unit,
            "value": value,
        })
    };

    let input_value = detail.input_values.get(&input.name).copied();
    let action = detail.actions.as_deref().and_then(|a| a.first());

    step["period"] = json!(period);
    step["input"] = describe(input, input_value);
    step["output"] = describe(output, action.map(|a| a.value));
    step["delta"] = json!(action.map(|a| a.value));
    step["success"] = json!(action.map(|a| a.success).unwrap_or(false));
}

fn build_action_display(
    actions: &[ActionResult],
    instance_map: &HashMap<u32, InstanceInfo>,
) -> Vec<Value> {
    actions
        .iter()
        .map(|a| {
            let (target_name, point_name, unit) = if a.target_type == "instance" {
                let info = instance_map.get(&a.target_id);
                let (pname, u) = info
                    .and_then(|inst| {
                        lookup_point(&inst.product_name, Some(a.point_type), a.point_id)
                    })
                    .unwrap_or(("", ""));
                (
                    info.map(|i| i.name.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    pname.to_string(),
                    u.to_string(),
                )
            } else {
                (
                    format!("Channel#{}", a.target_id),
                    format!("{}:{}", a.point_type, a.point_id),
                    String::new(),
                )
            };

            let base = if unit.is_empty() {
                format!("Set {} · {} = {}", target_name, point_name, a.value)
            } else {
                format!(
                    "Set {} · {} = {} {}",
                    target_name, point_name, a.value, unit
                )
            };
            let description = if a.success {
                base
            } else {
                format!("{} (failed)", base)
            };

            json!({
                "description": description,
                "target_name": target_name,
                "point_name": point_name,
                "unit": unit,
                "value": a.value,
                "success": a.success,
            })
        })
        .collect()
}

fn build_summary(result: &RuleExecutionResult, actions: &[Value]) -> String {
    if !result.success {
        return match &result.error {
            Some(e) => format!("Failed: {}", e),
            None => "Execution failed".to_string(),
        };
    }
    if actions.is_empty() {
        return "Rule evaluated, no actions taken".to_string();
    }

    let failed_count = actions
        .iter()
        .filter(|a| !a.get("success").and_then(|s| s.as_bool()).unwrap_or(true))
        .count();

    let descs: Vec<&str> = actions
        .iter()
        .filter_map(|a| a.get("description").and_then(|d| d.as_str()))
        .collect();

    let base = match descs.len() {
        0 => "Rule executed successfully".to_string(),
        1 => descs[0].to_string(),
        n => format!("{}, and {} more action(s)", descs[0], n - 1),
    };

    if failed_count > 0 {
        format!("{} ({} action(s) failed to write)", base, failed_count)
    } else {
        base
    }
}

/// Build the trigger reason text.
///
/// Composed from whichever nodes actually ran, by role — not just "the
/// matched Switch expression" (which left ChangeValue/Calculation/PeriodDelta
/// -only rules with the generic "Rule triggered" fallback, and previously
/// used bare point names with no owning device):
/// - Switch nodes on the path: "matched condition" with each variable shown
///   as "instance·point (value unit)" — the full triggering condition, not
///   just the final written point.
/// - No Switch node on the path (a pure ChangeValue/Calculation/PeriodDelta
///   flow, typically driven by an OnChange/interval trigger outside the
///   flow itself): describe the first executed node instead, so the reason
///   always reflects what the flow actually did.
fn build_trigger_reason(
    result: &RuleExecutionResult,
    rule_flow: Option<&RuleFlow>,
    node_info_map: &HashMap<String, FlowNodeInfo>,
    var_defs: &HashMap<String, RuleVariable>,
    instance_map: &HashMap<u32, InstanceInfo>,
) -> String {
    let switch_reasons: Vec<String> = result
        .execution_path
        .iter()
        .filter_map(|node_id| {
            let detail = result.node_details.get(node_id)?;
            if detail.node_type != "switch" {
                return None;
            }
            let matched_port = detail.matched_port.as_ref()?;
            let expression = detail
                .condition_results
                .as_ref()?
                .iter()
                .find(|cr| &cr.port == matched_port && cr.result)?
                .expression
                .clone();
            // Resolve against *this node's own* input snapshot, not the
            // run's final values — a later node may have reused the same
            // variable name for a different point and overwritten it.
            let local_map = build_var_info_map(var_defs, &detail.input_values, instance_map);
            let local_substitution = build_substitution_map(&local_map);
            let resolved = substitute_variable_tokens(&expression, &local_substitution);
            let label = node_info_map.get(node_id).map(|n| n.label.as_str());
            Some(match label {
                Some(l) if !l.is_empty() => format!("{}: {}", l, resolved),
                _ => resolved,
            })
        })
        .collect();

    if !switch_reasons.is_empty() {
        return format!("Condition matched — {}", switch_reasons.join("; "));
    }

    // No Switch fired on this path — describe the first executed action node.
    for node_id in &result.execution_path {
        let Some(detail) = result.node_details.get(node_id) else {
            continue;
        };
        let flow_node = rule_flow.and_then(|f| f.nodes.get(node_id));
        match (detail.node_type, flow_node) {
            ("change", Some(RuleNode::ChangeValue { rule: assigns, .. })) => {
                let names: Vec<&str> = assigns.iter().map(|a| a.variables.as_str()).collect();
                return format!("Value change triggered: set {}", names.join(", "));
            },
            ("calculation", Some(RuleNode::Calculation { rule: calcs, .. })) => {
                let local_map = build_var_info_map(var_defs, &detail.input_values, instance_map);
                let local_substitution = build_substitution_map(&local_map);
                let parts: Vec<String> = calcs
                    .iter()
                    .map(|c| {
                        format!(
                            "{} = {}",
                            c.output,
                            substitute_variable_tokens(&c.formula, &local_substitution)
                        )
                    })
                    .collect();
                return format!("Calculation triggered: {}", parts.join(", "));
            },
            ("periodDelta", Some(RuleNode::PeriodDelta { period, .. })) => {
                return format!("Period data triggered ({} delta)", period);
            },
            _ => continue,
        }
    }

    "Rule triggered".to_string()
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::types::{RuleValueAssignment, RuleWires};
    use std::sync::Arc;

    fn var(name: &str, instance: u32, point: u32) -> RuleVariable {
        RuleVariable {
            name: name.to_string(),
            instance: Some(instance),
            point_type: Some("measurement".to_string()),
            point: Some(point),
            formula: vec![],
        }
    }

    fn base_result() -> RuleExecutionResult {
        RuleExecutionResult {
            rule_id: 1,
            success: true,
            actions_executed: vec![],
            error: None,
            execution_path: vec![],
            execution_graph: Default::default(),
            matched_condition: None,
            variable_values: Arc::new(HashMap::new()),
            point_values: Arc::new(HashMap::new()),
            node_details: HashMap::new(),
        }
    }

    #[test]
    fn execution_graph_uses_enriched_nodes_without_matched_port_duplication() {
        let mut result = base_result();
        result
            .execution_graph
            .nodes
            .push(crate::executor::ExecutionGraphNode {
                id: "sw1".to_string(),
                node_type: "switch",
                label: "Switch Function",
                status: crate::executor::ExecutionNodeStatus::Executed,
                terminal: false,
                terminal_kind: None,
                terminal_reason: None,
            });
        result
            .execution_graph
            .edges
            .push(crate::executor::ExecutionGraphEdge {
                source: "sw1".to_string(),
                target: "cv1".to_string(),
                port: Some("out001".to_string()),
                label: Some("out001".to_string()),
            });

        let steps = vec![json!({
            "node_id": "sw1",
            "label": "SOC Branch",
            "type": "function-switch",
            "matched_port": "out001",
            "matched_label": "out001",
            "conditions": [],
        })];
        let graph = build_execution_graph(&result, &steps);
        let node = &graph["nodes"][0];

        assert_eq!(node["id"], "sw1");
        assert_eq!(node["label"], "SOC Branch");
        assert_eq!(node["type"], "switch");
        assert_eq!(node["status"], "executed");
        assert!(node.get("node_kind").is_none());
        assert!(node.get("matched_port").is_none());
        assert!(node.get("matched_label").is_none());
        assert_eq!(graph["edges"][0]["port"], "out001");
    }

    #[test]
    fn substitute_respects_word_boundaries() {
        let mut sub = HashMap::new();
        sub.insert("X1".to_string(), "SOC (78 %)".to_string());
        // "X11" must NOT be affected by the "X1" substitution.
        assert_eq!(
            substitute_variable_tokens("X1>=70 && X11<5", &sub),
            "SOC (78 %)>=70 && X11<5"
        );
    }

    #[test]
    fn trigger_reason_uses_node_local_snapshot_not_final_value() {
        // X1 is read by the Switch node while it was 78.0; a later node in
        // the same run reuses the name "X1" for a different point and
        // overwrites it to 5.0 by the time the run ends. trigger_reason
        // must report 78 (what actually triggered), not 5 (the final value).
        let mut result = base_result();
        result.execution_path = vec!["sw1".to_string()];
        let mut var_defs = HashMap::new();
        var_defs.insert("X1".to_string(), var("X1", 1, 10));

        let mut input_values = HashMap::new();
        input_values.insert("X1".to_string(), 78.0);
        result.node_details.insert(
            "sw1".to_string(),
            NodeExecutionDetail {
                node_type: "switch",
                input_values: Arc::new(input_values),
                condition_results: Some(vec![ConditionResult {
                    expression: "X1>=70".to_string(),
                    result: true,
                    port: "out001".to_string(),
                }]),
                matched_port: Some("out001".to_string()),
                actions: None,
                terminal: false,
            },
        );
        // Final snapshot disagrees with what the Switch node actually saw.
        let mut final_values = HashMap::new();
        final_values.insert("X1".to_string(), 5.0);
        result.variable_values = Arc::new(final_values);

        let mut node_info_map = HashMap::new();
        node_info_map.insert(
            "sw1".to_string(),
            FlowNodeInfo {
                label: "SOC Branch".to_string(),
                node_type: "function-switch".to_string(),
            },
        );

        let reason =
            build_trigger_reason(&result, None, &node_info_map, &var_defs, &HashMap::new());
        assert_eq!(reason, "Condition matched — SOC Branch: X1 (78)>=70");
    }

    #[test]
    fn trigger_reason_expands_combined_variable_formula_chain() {
        // X3 = X1 + X2 (a Switch-local combined variable). The reason must
        // show the operand chain, not collapse to a bare "X3 (301)".
        let mut result = base_result();
        result.execution_path = vec!["sw1".to_string()];

        let mut var_defs = HashMap::new();
        var_defs.insert("X1".to_string(), var("X1", 1, 1));
        var_defs.insert("X2".to_string(), var("X2", 1, 2));
        var_defs.insert(
            "X3".to_string(),
            RuleVariable {
                name: "X3".to_string(),
                instance: None,
                point_type: None,
                point: None,
                formula: vec![
                    serde_json::json!("X1"),
                    serde_json::json!("+"),
                    serde_json::json!("X2"),
                ],
            },
        );

        let mut input_values = HashMap::new();
        input_values.insert("X1".to_string(), 100.0);
        input_values.insert("X2".to_string(), 201.0);
        input_values.insert("X3".to_string(), 301.0);
        result.node_details.insert(
            "sw1".to_string(),
            NodeExecutionDetail {
                node_type: "switch",
                input_values: Arc::new(input_values),
                condition_results: Some(vec![ConditionResult {
                    expression: "X3>=300".to_string(),
                    result: true,
                    port: "out001".to_string(),
                }]),
                matched_port: Some("out001".to_string()),
                actions: None,
                terminal: false,
            },
        );

        let reason =
            build_trigger_reason(&result, None, &HashMap::new(), &var_defs, &HashMap::new());
        assert_eq!(
            reason,
            "Condition matched — X3[X1 (100) + X2 (201)] (301)>=300"
        );
    }

    #[test]
    fn trigger_reason_falls_back_to_change_value_when_no_switch_fired() {
        let mut result = base_result();
        result.execution_path = vec!["cv1".to_string()];
        result.node_details.insert(
            "cv1".to_string(),
            NodeExecutionDetail {
                node_type: "change",
                input_values: Arc::new(HashMap::new()),
                condition_results: None,
                matched_port: None,
                actions: Some(vec![ActionResult {
                    target_type: "instance",
                    target_id: 1,
                    point_type: "A",
                    point_id: 5,
                    value: 1.0,
                    success: true,
                }]),
                terminal: true,
            },
        );

        let mut nodes = HashMap::new();
        nodes.insert(
            "cv1".to_string(),
            RuleNode::ChangeValue {
                variables: vec![var("Y1", 1, 5)],
                rule: vec![RuleValueAssignment {
                    variables: "Y1".to_string(),
                    value: serde_json::json!(1.0),
                }],
                wires: RuleWires::default(),
            },
        );
        let flow = RuleFlow {
            start_node: "cv1".to_string(),
            nodes,
        };

        let reason = build_trigger_reason(
            &result,
            Some(&flow),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        );
        assert_eq!(reason, "Value change triggered: set Y1");
    }

    #[test]
    fn collect_all_variables_includes_period_delta_input_and_output() {
        let mut nodes = HashMap::new();
        nodes.insert(
            "pd1".to_string(),
            RuleNode::PeriodDelta {
                input: var("X1", 1, 10),
                output: var("Y1", 1, 11),
                period: "daily".to_string(),
                wires: RuleWires::default(),
            },
        );
        let flow = RuleFlow {
            start_node: "pd1".to_string(),
            nodes,
        };

        let vars = collect_all_variables(Some(&flow));
        let names: Vec<&str> = vars.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"X1"));
        assert!(names.contains(&"Y1"));
    }
}
