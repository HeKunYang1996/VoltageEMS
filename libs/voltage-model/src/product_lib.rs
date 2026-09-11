//! Built-in Product Library
//!
//! This module provides the built-in product templates that are embedded
//! at compile time. Products define the structure for device instances
//! including their measurements, actions, and properties.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

/// Point definition for measurements, actions, and properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointDef {
    /// Point ID (unique within product)
    pub id: u32,
    /// Point name
    pub name: String,
    /// Unit of measurement (empty string if none)
    #[serde(default)]
    pub unit: String,
    /// Value type (number, string, etc.)
    #[serde(rename = "type", default)]
    pub value_type: String,
    /// Human-readable point description.
    #[serde(default)]
    pub description: Option<String>,
    /// Enumerated values, when the point uses a closed string vocabulary.
    #[serde(default)]
    pub options: Vec<String>,
}

/// One product-level topology connection group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRule {
    pub products: Vec<String>,
    pub min: u32,
    pub max: Option<u32>,
}

/// Visual-topology capabilities declared by a product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default)]
    pub connections: Vec<ConnectionRule>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Built-in product definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinProduct {
    /// Product name (unique identifier)
    pub name: String,
    /// Product classification used for catalog grouping and cloud routing.
    #[serde(rename = "type")]
    pub product_type: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Products without topology are available to device management but are
    /// not draggable energy-topology nodes (for example Station and Env).
    #[serde(default)]
    pub topology: Option<TopologyDefinition>,
    #[serde(rename = "defaultDisplayMeasureIds", default)]
    pub default_display_measure_ids: Vec<u32>,
    /// Property definitions (P)
    #[serde(rename = "P", default)]
    pub properties: Vec<PointDef>,
    /// Measurement point definitions (M)
    #[serde(rename = "M", default)]
    pub measurements: Vec<PointDef>,
    /// Action point definitions (A)
    #[serde(rename = "A", default)]
    pub actions: Vec<PointDef>,
}

fn validate_products(products: &[BuiltinProduct]) -> Result<()> {
    let mut names = HashSet::new();
    for product in products {
        if product.name.is_empty() {
            anyhow::bail!("product name is empty");
        }
        if product.product_type.is_empty() {
            anyhow::bail!("product '{}' type is empty", product.name);
        }
        if !names.insert(product.name.as_str()) {
            anyhow::bail!("duplicate product name '{}'", product.name);
        }

        let measurement_ids: HashSet<u32> = product.measurements.iter().map(|p| p.id).collect();
        if measurement_ids.len() != product.measurements.len() {
            anyhow::bail!("product '{}' has duplicate measurement IDs", product.name);
        }
        if product
            .properties
            .iter()
            .map(|p| p.id)
            .collect::<HashSet<_>>()
            .len()
            != product.properties.len()
        {
            anyhow::bail!("product '{}' has duplicate property IDs", product.name);
        }
        if product
            .actions
            .iter()
            .map(|p| p.id)
            .collect::<HashSet<_>>()
            .len()
            != product.actions.len()
        {
            anyhow::bail!("product '{}' has duplicate action IDs", product.name);
        }
        if product
            .default_display_measure_ids
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != product.default_display_measure_ids.len()
        {
            anyhow::bail!(
                "product '{}' has duplicate default display measurement IDs",
                product.name
            );
        }
        for id in &product.default_display_measure_ids {
            if !measurement_ids.contains(id) {
                anyhow::bail!(
                    "product '{}' default display measurement {} does not exist",
                    product.name,
                    id
                );
            }
        }
    }

    let by_name: HashMap<&str, &BuiltinProduct> =
        products.iter().map(|p| (p.name.as_str(), p)).collect();
    for product in products {
        let Some(topology) = &product.topology else {
            continue;
        };
        let mut targets = HashSet::new();
        for rule in &topology.connections {
            if rule.products.is_empty() {
                anyhow::bail!("product '{}' has an empty connection group", product.name);
            }
            if let Some(max) = rule.max
                && rule.min > max
            {
                anyhow::bail!(
                    "product '{}' connection rule has min {} greater than max {}",
                    product.name,
                    rule.min,
                    max
                );
            }
            for target in &rule.products {
                if target == &product.name {
                    anyhow::bail!("product '{}' cannot connect to itself", product.name);
                }
                if !targets.insert(target.as_str()) {
                    anyhow::bail!(
                        "product '{}' repeats connection target '{}'",
                        product.name,
                        target
                    );
                }
                let peer = by_name.get(target.as_str()).with_context(|| {
                    format!(
                        "product '{}' references unknown connection target '{}'",
                        product.name, target
                    )
                })?;
                let reciprocal = peer.topology.as_ref().is_some_and(|peer_topology| {
                    peer_topology
                        .connections
                        .iter()
                        .any(|peer_rule| peer_rule.products.contains(&product.name))
                });
                if !reciprocal {
                    anyhow::bail!(
                        "connection '{}'-'{}' is not reciprocal",
                        product.name,
                        target
                    );
                }
            }
        }
    }
    Ok(())
}

// Embed all product JSON files at compile time (auto-discovered by build.rs)
// Note: products/ is a git submodule (voltage-product-lib)
static BUILTIN_PRODUCTS: LazyLock<Vec<BuiltinProduct>> = LazyLock::new(|| {
    let jsons: &[&str] = include!(concat!(env!("OUT_DIR"), "/product_includes.rs"));

    let products = jsons
        .iter()
        .enumerate()
        .map(|(index, json)| {
            serde_json::from_str(json).unwrap_or_else(|error| {
                panic!("invalid built-in product JSON at index {index}: {error}")
            })
        })
        .collect::<Vec<_>>();
    if let Err(error) = validate_products(&products) {
        panic!("invalid built-in product library: {error}");
    }
    products
});

/// Get all built-in products
pub fn get_builtin_products() -> &'static [BuiltinProduct] {
    &BUILTIN_PRODUCTS
}

/// Get a built-in product by name
pub fn get_builtin_product(name: &str) -> Option<&'static BuiltinProduct> {
    BUILTIN_PRODUCTS.iter().find(|p| p.name == name)
}

/// Get all product names
pub fn get_product_names() -> Vec<&'static str> {
    BUILTIN_PRODUCTS.iter().map(|p| p.name.as_str()).collect()
}

/// Check if a product exists in the built-in library
pub fn product_exists(name: &str) -> bool {
    BUILTIN_PRODUCTS.iter().any(|p| p.name == name)
}

/// Runtime product library with external override support
///
/// Merges compile-time built-in products with optional external JSON files.
/// External products (from `config/products/*.json`) override built-in ones
/// by name, enabling new device types without recompilation.
///
/// # Priority
/// External `config/products/*.json` > built-in `BUILTIN_PRODUCTS`
///
/// # Example
/// ```ignore
/// let lib = ProductLibrary::load(Some(Path::new("config/products")))?;
/// let battery = lib.get("Battery").expect("Battery product");
/// ```
pub struct ProductLibrary {
    products: Vec<BuiltinProduct>,
}

impl ProductLibrary {
    /// Load products: external dir overrides built-in defaults
    ///
    /// If `products_dir` is None or doesn't exist, returns built-in products only.
    pub fn load(products_dir: Option<&Path>) -> Result<Self> {
        let mut products: Vec<BuiltinProduct> = BUILTIN_PRODUCTS.clone();

        if let Some(dir) = products_dir
            && dir.is_dir()
        {
            let entries = std::fs::read_dir(dir)
                .with_context(|| format!("Failed to read products dir: {}", dir.display()))?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }

                let content = std::fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;

                let product: BuiltinProduct = serde_json::from_str(&content)
                    .with_context(|| format!("Invalid product JSON: {}", path.display()))?;

                // Override existing or append new
                if let Some(idx) = products.iter().position(|p| p.name == product.name) {
                    tracing::info!(
                        "Product '{}' overridden from {}",
                        product.name,
                        path.display()
                    );
                    products[idx] = product;
                } else {
                    tracing::info!("Product '{}' loaded from {}", product.name, path.display());
                    products.push(product);
                }
            }
        }

        validate_products(&products)?;

        Ok(Self { products })
    }

    /// Create from built-in products only (no external overrides)
    pub fn builtin_only() -> Self {
        Self {
            products: BUILTIN_PRODUCTS.clone(),
        }
    }

    /// Get all products
    pub fn all(&self) -> &[BuiltinProduct] {
        &self.products
    }

    /// Get product by name
    pub fn get(&self, name: &str) -> Option<&BuiltinProduct> {
        self.products.iter().find(|p| p.name == name)
    }

    /// Get all product names
    pub fn names(&self) -> Vec<&str> {
        self.products.iter().map(|p| p.name.as_str()).collect()
    }

    /// Check if product exists
    pub fn exists(&self, name: &str) -> bool {
        self.products.iter().any(|p| p.name == name)
    }

    /// Get number of products
    pub fn len(&self) -> usize {
        self.products.len()
    }

    /// Check if library is empty
    pub fn is_empty(&self) -> bool {
        self.products.is_empty()
    }
}

/// Validate product JSON files in a directory without loading them into a library
///
/// Returns a list of (filename, error_message) for invalid files.
/// Valid files return an empty list.
pub fn validate_product_dir(dir: &Path) -> Vec<(String, String)> {
    let mut errors = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            errors.push(("(directory)".to_string(), e.to_string()));
            return errors;
        },
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                errors.push((filename, format!("read error: {}", e)));
                continue;
            },
        };

        match serde_json::from_str::<BuiltinProduct>(&content) {
            Ok(p) => {
                if p.name.is_empty() {
                    errors.push((filename, "product name is empty".to_string()));
                }
            },
            Err(e) => {
                errors.push((filename, format!("JSON parse error: {}", e)));
            },
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_products_loaded() {
        let products = get_builtin_products();
        assert!(!products.is_empty(), "Should have built-in products");
    }

    #[test]
    fn test_get_product_by_name() {
        let battery = get_builtin_product("Battery").expect("Battery should exist");
        assert_eq!(battery.name, "Battery");
        assert_eq!(battery.product_type, "ESS");
        let topology = battery.topology.as_ref().expect("Battery topology");
        assert_eq!(topology.connections[0].products, ["Hybrid_Inverter", "PCS"]);
        assert_eq!(topology.connections[0].min, 1);
        assert_eq!(topology.connections[0].max, Some(1));
        assert!(!battery.measurements.is_empty());
        assert!(!battery.default_display_measure_ids.is_empty());
    }

    #[test]
    fn test_product_capabilities() {
        let station = get_builtin_product("Station").expect("Station should exist");
        assert!(station.topology.is_none());
        assert!(get_builtin_product("ESS").is_none());
        assert!(get_builtin_product("Distribution_Board").is_none());

        let hybrid = get_builtin_product("Hybrid_Inverter").expect("Hybrid_Inverter");
        assert_eq!(hybrid.properties.len(), 9);
        assert_eq!(hybrid.measurements.len(), 15);
        assert_eq!(hybrid.actions.len(), 4);
        assert_eq!(hybrid.topology.as_ref().unwrap().connections.len(), 3);

        let meter = get_builtin_product("Meter").expect("Meter");
        assert!(meter.topology.is_some());
    }

    #[test]
    fn test_product_catalog_types() {
        assert_eq!(
            get_builtin_product("Station").unwrap().product_type,
            "Station"
        );
        assert_eq!(get_builtin_product("Battery").unwrap().product_type, "ESS");
        assert_eq!(get_builtin_product("Meter").unwrap().product_type, "Meter");
    }

    #[test]
    fn test_product_count_and_removed_placeholders() {
        assert_eq!(get_builtin_products().len(), 13);
        for removed in ["ESS", "Load", "Generator", "Inverter", "Distribution_Board"] {
            assert!(!product_exists(removed));
        }
    }

    #[test]
    fn test_product_exists() {
        assert!(product_exists("Battery"));
        assert!(product_exists("PCS"));
        assert!(!product_exists("NonExistent"));
    }

    // ========== ProductLibrary Tests ==========

    #[test]
    fn test_product_library_builtin_only() {
        let lib = ProductLibrary::builtin_only();
        assert!(lib.len() >= 10);
        assert!(lib.exists("Battery"));
        assert!(lib.exists("PCS"));
        assert!(!lib.exists("CustomDevice"));
        assert!(!lib.is_empty());
    }

    #[test]
    fn test_product_library_load_no_dir() -> anyhow::Result<()> {
        let lib = ProductLibrary::load(None)?;
        assert!(lib.len() >= 10);
        Ok(())
    }

    #[test]
    fn test_product_library_load_nonexistent_dir() -> anyhow::Result<()> {
        let lib = ProductLibrary::load(Some(Path::new("/nonexistent/path")))?;
        assert!(lib.len() >= 10); // Falls back to built-in only
        Ok(())
    }

    #[test]
    fn test_product_library_load_with_override() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let products_dir = temp_dir.path();

        // Write a custom product that overrides Battery
        let custom_battery = r#"{
            "name": "Battery",
            "type": "ESS",
            "topology": {"connections": [{"products": ["Hybrid_Inverter", "PCS"], "min": 1, "max": 1}]},
            "defaultDisplayMeasureIds": [1],
            "M": [{"id": 1, "name": "CustomVoltage", "unit": "V"}],
            "A": [],
            "P": []
        }"#;
        std::fs::write(products_dir.join("Battery.json"), custom_battery)?;

        let lib = ProductLibrary::load(Some(products_dir))?;
        assert!(lib.len() >= 10); // Same count (override, not add)

        let battery = lib.get("Battery").context("Battery not found")?;
        assert_eq!(battery.measurements.len(), 1);
        assert_eq!(battery.measurements[0].name, "CustomVoltage");
        Ok(())
    }

    #[test]
    fn test_product_library_load_with_new_product() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let products_dir = temp_dir.path();

        // Write a brand new product
        let custom_product = r#"{
            "name": "WindTurbine",
            "type": "Generator",
            "topology": {"connections": []},
            "defaultDisplayMeasureIds": [1],
            "M": [{"id": 1, "name": "WindSpeed", "unit": "m/s"}],
            "A": [],
            "P": []
        }"#;
        std::fs::write(products_dir.join("WindTurbine.json"), custom_product)?;

        let lib = ProductLibrary::load(Some(products_dir))?;
        let builtin_count = get_builtin_products().len();
        assert_eq!(lib.len(), builtin_count + 1); // built-in + 1 new
        assert!(lib.exists("WindTurbine"));

        let wind = lib.get("WindTurbine").context("WindTurbine not found")?;
        assert_eq!(wind.product_type, "Generator");
        Ok(())
    }

    #[test]
    fn test_product_library_names() {
        let lib = ProductLibrary::builtin_only();
        let names = lib.names();
        assert!(names.contains(&"Battery"));
        assert!(names.contains(&"Station"));
    }

    #[test]
    fn test_product_library_topology_filter() {
        let lib = ProductLibrary::builtin_only();
        let topology_names: Vec<&str> = lib
            .all()
            .iter()
            .filter(|product| product.topology.is_some())
            .map(|product| product.name.as_str())
            .collect();
        assert!(topology_names.contains(&"Battery"));
        assert!(!topology_names.contains(&"Station"));
    }

    #[test]
    fn test_validate_product_dir_valid() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let dir = temp_dir.path();

        let valid = r#"{
            "name": "Test",
            "type": "Test",
            "topology": {"connections": []},
            "M": [], "A": [], "P": []
        }"#;
        std::fs::write(dir.join("Test.json"), valid)?;

        let errors = validate_product_dir(dir);
        assert!(errors.is_empty());
        Ok(())
    }

    #[test]
    fn test_validate_product_dir_invalid_json() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let dir = temp_dir.path();

        std::fs::write(dir.join("Bad.json"), "not json")?;

        let errors = validate_product_dir(dir);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].1.contains("JSON parse error"));
        Ok(())
    }

    #[test]
    fn test_validate_product_dir_empty_name() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let dir = temp_dir.path();

        let empty_name = r#"{
            "name": "",
            "type": "Test",
            "topology": {"connections": []},
            "M": [], "A": [], "P": []
        }"#;
        std::fs::write(dir.join("Empty.json"), empty_name)?;

        let errors = validate_product_dir(dir);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].1.contains("empty"));
        Ok(())
    }
}
