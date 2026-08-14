//! Built-in Product Library
//!
//! This module provides the built-in product templates that are embedded
//! at compile time. Products define the structure for device instances
//! including their measurements, actions, and properties.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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
}

/// Product capability in the visual topology editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TopologyType {
    TopLevel,
    Standalone,
    Composite,
    Container,
}

/// A component contained by a composite or container topology product.
///
/// Product-backed components set `productName`. Inline components (for example
/// a distribution-board meter) set `name` and may constrain selectable product
/// types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TopologyComponent {
    Product {
        #[serde(rename = "productName")]
        product_name: String,
    },
    Inline {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        image: Option<String>,
        #[serde(rename = "selectableProductTypes", default)]
        selectable_product_types: Vec<String>,
    },
}

/// Visual-topology capabilities declared by a product.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyDefinition {
    pub enabled: bool,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub topology_type: Option<TopologyType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default)]
    pub components: Vec<TopologyComponent>,
    /// Product names explicitly declared as valid topology connection peers.
    /// This is the source library's one-sided rule; API consumers receive the
    /// symmetric closure computed by modsrv.
    #[serde(rename = "connectableProducts", default)]
    pub connectable_products: Vec<String>,
}

/// Built-in product definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinProduct {
    /// Product name (unique identifier)
    pub name: String,
    /// Parent product name for hierarchy (e.g., Battery -> ESS -> Station)
    #[serde(rename = "pName")]
    pub parent_name: Option<String>,
    /// Whether users may create an instance from this product.
    #[serde(rename = "canCreateInstance")]
    pub can_create_instance: bool,
    /// Visual-topology capabilities. `enabled = false` marks catalog-only
    /// products that cannot be placed in the topology editor.
    pub topology: TopologyDefinition,
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

// Embed all product JSON files at compile time (auto-discovered by build.rs)
// Note: products/ is a git submodule (voltage-product-lib)
static BUILTIN_PRODUCTS: LazyLock<Vec<BuiltinProduct>> = LazyLock::new(|| {
    let jsons: &[&str] = include!(concat!(env!("OUT_DIR"), "/product_includes.rs"));

    jsons
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect()
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

/// Get child products of a given parent
pub fn get_child_products(parent_name: &str) -> Vec<&'static BuiltinProduct> {
    BUILTIN_PRODUCTS
        .iter()
        .filter(|p| p.parent_name.as_deref() == Some(parent_name))
        .collect()
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

    /// Get child products of a given parent
    pub fn children(&self, parent_name: &str) -> Vec<&BuiltinProduct> {
        self.products
            .iter()
            .filter(|p| p.parent_name.as_deref() == Some(parent_name))
            .collect()
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
        assert_eq!(battery.parent_name.as_deref(), Some("ESS"));
        assert!(battery.can_create_instance);
        assert_eq!(
            battery.topology.topology_type,
            Some(TopologyType::Standalone)
        );
        assert!(battery.topology.enabled);
        assert_eq!(
            battery.topology.connectable_products,
            ["Hybrid_Inverter", "PCS"]
        );
        assert!(!battery.measurements.is_empty());
    }

    #[test]
    fn test_product_capabilities() {
        let station = get_builtin_product("Station").expect("Station should exist");
        let station_topology = &station.topology;
        assert_eq!(station_topology.topology_type, Some(TopologyType::TopLevel));
        assert!(station_topology.enabled);
        assert!(station_topology.image.is_none());

        let ess = get_builtin_product("ESS").expect("ESS should exist");
        assert!(!ess.can_create_instance);
        assert!(!ess.topology.enabled);
        assert!(ess.topology.topology_type.is_none());

        let hybrid = get_builtin_product("Hybrid_Inverter").expect("Hybrid_Inverter");
        assert!(hybrid.can_create_instance);
        assert!(hybrid.properties.is_empty());
        assert!(hybrid.measurements.is_empty());
        assert!(hybrid.actions.is_empty());
        let components = &hybrid.topology.components;
        assert_eq!(components.len(), 2);
        assert!(matches!(
            &components[0],
            TopologyComponent::Product { product_name } if product_name == "AC_Inverter"
        ));
        assert!(matches!(
            &components[1],
            TopologyComponent::Product { product_name } if product_name == "PCS"
        ));

        let board = get_builtin_product("Distribution_Board").expect("Distribution_Board");
        assert!(!board.can_create_instance);
        let meter = &board.topology.components[0];
        assert!(matches!(
            meter,
            TopologyComponent::Inline {
                name,
                selectable_product_types,
                ..
            } if name == "Meter"
                && selectable_product_types == &["Single_Phase_Load", "Three_Phase_Load"]
        ));
    }

    #[test]
    fn test_product_hierarchy() {
        // Station is root
        let station = get_builtin_product("Station").expect("Station should exist");
        assert!(station.parent_name.is_none());

        // ESS -> Station
        let ess = get_builtin_product("ESS").expect("ESS should exist");
        assert_eq!(ess.parent_name.as_deref(), Some("Station"));

        // Battery -> ESS
        let battery = get_builtin_product("Battery").expect("Battery should exist");
        assert_eq!(battery.parent_name.as_deref(), Some("ESS"));
    }

    #[test]
    fn test_get_child_products() {
        let station_children = get_child_products("Station");
        let names: Vec<_> = station_children.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"ESS"));
        assert!(names.contains(&"Generator"));
        assert!(names.contains(&"Env"));
        assert!(names.contains(&"Load"));
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
            "pName": "ESS",
            "canCreateInstance": true,
            "topology": {"enabled": true, "type": "standalone", "connectableProducts": []},
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
            "pName": "Station",
            "canCreateInstance": true,
            "topology": {"enabled": true, "type": "standalone", "connectableProducts": []},
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
        assert_eq!(wind.parent_name.as_deref(), Some("Station"));
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
    fn test_product_library_children() {
        let lib = ProductLibrary::builtin_only();
        let station_children = lib.children("Station");
        let names: Vec<&str> = station_children.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"ESS"));
        assert!(names.contains(&"Generator"));
    }

    #[test]
    fn test_validate_product_dir_valid() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let dir = temp_dir.path();

        let valid = r#"{
            "name": "Test",
            "canCreateInstance": true,
            "topology": {"enabled": true, "type": "standalone", "connectableProducts": []},
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
            "canCreateInstance": true,
            "topology": {"enabled": true, "type": "standalone", "connectableProducts": []},
            "M": [], "A": [], "P": []
        }"#;
        std::fs::write(dir.join("Empty.json"), empty_name)?;

        let errors = validate_product_dir(dir);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].1.contains("empty"));
        Ok(())
    }
}
