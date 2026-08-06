//! Expand SunSpec model JSON into Modbus point definitions.

use std::collections::HashMap;

use serde::Serialize;

use crate::sunspec::types::{SunSpecGroup, SunSpecModel, SunSpecPoint};

/// A model block discovered on a SunSpec device.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredModel {
    pub model_id: u16,
    pub length: u16,
    /// Register address of the model ID field (0-based Modbus address).
    pub start_register: u16,
}

/// Filter controlling which SunSpec points become Modbus mappings.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExpandFilter {
    /// Include static/nameplate points (`static: "S"`).
    pub include_static: bool,
    /// Include scale-factor registers (`type: sunssf`).
    pub include_scale_factors: bool,
    /// Include optional (`mandatory: "O"`) points.
    pub include_optional: bool,
}

/// Configuration for expanding a single model to Modbus points.
#[derive(Debug, Clone)]
pub struct ExpandConfig {
    pub model_id: u16,
    /// Register address where the model ID field lives.
    pub start_register: u16,
    pub slave_id: u8,
    pub function_code: u8,
    pub filter: ExpandFilter,
    /// Scale-factor values read from the device: SF point name → int16 value.
    /// When present, `scale = 10^sf_value` is applied to each point referencing that SF.
    pub sf_values: HashMap<String, i16>,
}

/// One expanded point ready for SQLite insertion.
#[derive(Debug, Clone, Serialize)]
pub struct ExpandedPoint {
    pub signal_name: String,
    pub register_address: u16,
    pub data_type: String,
    pub unit: String,
    pub description: String,
    pub scale: f64,
    pub offset: f64,
    pub protocol_mappings: String,
}

/// Collect all `sunssf` points in a model with their computed register addresses.
///
/// Returns `(name, register_address)` pairs in model-walk order.
/// Used to identify which registers to read from the device before expansion.
pub fn collect_sf_points(model: &SunSpecModel, start_register: u16) -> Vec<(String, u16)> {
    let mut out = Vec::new();
    let mut offset = 0u16;
    walk_sf_group(&model.group, start_register, &mut offset, &mut out);
    out
}

fn walk_sf_group(
    group: &SunSpecGroup,
    start_register: u16,
    offset: &mut u16,
    out: &mut Vec<(String, u16)>,
) {
    let repeat = group.count.as_fixed().unwrap_or(1);
    for _ in 0..repeat {
        for point in &group.points {
            let register = start_register.saturating_add(*offset);
            *offset = offset.saturating_add(point.size);
            if point.point_type == "sunssf" {
                out.push((point.name.clone(), register));
            }
        }
        for nested in &group.groups {
            walk_sf_group(nested, start_register, offset, out);
        }
    }
}

/// Expand a SunSpec model JSON into Modbus telemetry points.
pub fn expand_model(model: &SunSpecModel, config: &ExpandConfig) -> Vec<ExpandedPoint> {
    let mut points = Vec::new();
    let mut offset = 0u16;
    walk_group(
        &model.group,
        config,
        &model.group.name,
        &mut offset,
        &mut points,
    );
    points
}

fn walk_group(
    group: &SunSpecGroup,
    config: &ExpandConfig,
    group_prefix: &str,
    offset: &mut u16,
    out: &mut Vec<ExpandedPoint>,
) {
    let repeat = group.count.as_fixed().unwrap_or(1);
    if group.count.as_fixed().is_none() {
        tracing::warn!(
            "SunSpec model {} group '{}' has dynamic count; expanding once",
            config.model_id,
            group.name
        );
    }

    for instance in 0..repeat {
        let prefix = if repeat > 1 {
            format!("{group_prefix}_{instance}")
        } else {
            group_prefix.to_string()
        };

        for point in &group.points {
            emit_point(point, config, &prefix, offset, out);
        }

        for nested in &group.groups {
            let nested_prefix = format!("{prefix}_{}", nested.name);
            walk_group(nested, config, &nested_prefix, offset, out);
        }
    }
}

fn emit_point(
    point: &SunSpecPoint,
    config: &ExpandConfig,
    group_prefix: &str,
    offset: &mut u16,
    out: &mut Vec<ExpandedPoint>,
) {
    let register = config.start_register.saturating_add(*offset);
    *offset = offset.saturating_add(point.size);

    if !should_include(point, &config.filter) {
        return;
    }

    let Some(data_type) = sunspec_type_to_data_type(&point.point_type, point.size) else {
        return;
    };

    let signal_name = format!("M{}_{}_{}", config.model_id, group_prefix, point.name);
    let description = point
        .desc
        .clone()
        .or_else(|| point.label.clone())
        .unwrap_or_else(|| point.name.clone());

    let scale = point
        .sf
        .as_deref()
        .and_then(|sf_name| config.sf_values.get(sf_name))
        .map(|&sf_val| 10f64.powi(sf_val as i32))
        .unwrap_or(1.0);

    let mapping = serde_json::json!({
        "slave_id": config.slave_id,
        "function_code": config.function_code,
        "register_address": register,
        "data_type": data_type,
        "byte_order": "ABCD",
    });

    out.push(ExpandedPoint {
        signal_name,
        register_address: register,
        data_type: data_type.to_string(),
        unit: point.units.clone().unwrap_or_default(),
        description,
        scale,
        offset: 0.0,
        protocol_mappings: mapping.to_string(),
    });
}

fn should_include(point: &SunSpecPoint, filter: &ExpandFilter) -> bool {
    if matches!(point.point_type.as_str(), "pad" | "string" | "count") {
        return false;
    }

    if point.r#static.as_deref() == Some("S") && !filter.include_static {
        return false;
    }

    if point.point_type == "sunssf" && !filter.include_scale_factors {
        return false;
    }

    if point.mandatory.as_deref() == Some("O") && !filter.include_optional {
        return false;
    }

    if point.access.as_deref() == Some("RW") {
        // v1: read-only telemetry only
        return false;
    }

    sunspec_type_to_data_type(&point.point_type, point.size).is_some()
}

fn sunspec_type_to_data_type(point_type: &str, size: u16) -> Option<&'static str> {
    match point_type {
        "uint16" | "enum16" | "acc16" | "raw16" | "bitfield16" => Some("uint16"),
        "int16" | "sunssf" => Some("int16"),
        "uint32" | "enum32" | "acc32" => Some("uint32"),
        "int32" => Some("int32"),
        "uint64" | "acc64" => Some("uint64"),
        "int64" => Some("int64"),
        "float32" => Some("float32"),
        "float64" => Some("float64"),
        "bitfield32" => {
            if size >= 2 {
                Some("uint32")
            } else {
                Some("uint16")
            }
        },
        "bitfield64" => Some("uint64"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sunspec::model::load_model;

    #[test]
    fn expand_model_103_skips_header_and_sf_by_default() {
        let model = load_model(103).unwrap();
        let config = ExpandConfig {
            model_id: 103,
            start_register: 40_002,
            slave_id: 1,
            function_code: 3,
            filter: ExpandFilter::default(),
            sf_values: HashMap::new(),
        };

        let points = expand_model(&model, &config);
        assert!(!points.is_empty());
        assert!(points.iter().all(|p| !p.signal_name.ends_with("_ID")));
        assert!(points.iter().all(|p| !p.signal_name.ends_with("_A_SF")));

        let a = points
            .iter()
            .find(|p| p.signal_name.ends_with("_A"))
            .expect("A point");
        assert_eq!(a.register_address, 40_004);
        assert_eq!(a.data_type, "uint16");
    }

    #[test]
    fn expand_model_103_with_sf() {
        let model = load_model(103).unwrap();
        let config = ExpandConfig {
            model_id: 103,
            start_register: 40_002,
            slave_id: 1,
            function_code: 3,
            filter: ExpandFilter {
                include_scale_factors: true,
                ..Default::default()
            },
            sf_values: HashMap::new(),
        };

        let points = expand_model(&model, &config);
        assert!(points.iter().any(|p| p.signal_name.ends_with("_A_SF")));
    }

    #[test]
    fn sf_values_applied_to_scale() {
        let model = load_model(103).unwrap();
        let sf_points = collect_sf_points(&model, 40_002);
        assert!(!sf_points.is_empty(), "Model 103 should have SF points");

        // Simulate device returning A_SF = -2 (raw value = 0.01 multiplier)
        let mut sf_values = HashMap::new();
        for (name, _reg) in &sf_points {
            sf_values.insert(name.clone(), -2i16);
        }

        let config = ExpandConfig {
            model_id: 103,
            start_register: 40_002,
            slave_id: 1,
            function_code: 3,
            filter: ExpandFilter::default(),
            sf_values,
        };

        let points = expand_model(&model, &config);
        let a = points
            .iter()
            .find(|p| p.signal_name.ends_with("_A"))
            .expect("A point");
        let expected = 10f64.powi(-2);
        assert!(
            (a.scale - expected).abs() < 1e-10,
            "Expected scale {expected} for SF=-2, got {}",
            a.scale
        );
    }
}
