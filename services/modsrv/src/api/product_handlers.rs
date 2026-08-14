//! Product Management API Handlers (Read-only)
//!
//! Provides endpoints for querying product templates and definitions.
//! Products are compile-time constants and cannot be created via API.

#![allow(clippy::disallowed_methods)] // json! macro internally uses unwrap (safe for known valid JSON)

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use common::SuccessResponse;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::config::Product;
use crate::error::ModSrvError;

/// Optional capability filters for the product catalog.
#[derive(Debug, Default, Deserialize)]
pub struct ProductListQuery {
    pub can_create_instance: Option<bool>,
    pub topology_enabled: Option<bool>,
}

impl ProductListQuery {
    fn matches(&self, product: &Product) -> bool {
        self.can_create_instance
            .is_none_or(|expected| product.can_create_instance == expected)
            && self
                .topology_enabled
                .is_none_or(|expected| product.topology.enabled == expected)
    }
}

/// List all available product templates (lightweight)
///
/// Returns product names, parent relationships, and instance/topology capabilities.
/// This endpoint is optimized for frontend dropdown lists and product selection interfaces.
/// For detailed product information including measurements/actions/properties, use GET /api/products/{product_name}/points.
///
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/products",
    tag = "products",
    params(
        ("can_create_instance" = Option<bool>, Query, description = "Filter by whether instances can be created"),
        ("topology_enabled" = Option<bool>, Query, description = "Filter by whether the product is enabled for topology use")
    ),
    responses(
        (status = 200, description = "Lightweight product list retrieved successfully",
            body = inline(Object),
            example = json!({
                "success": true,
                "data": {
                    "count": 9,
                    "products": [
                        {
                            "product_name": "Station",
                            "parent_name": null,
                            "can_create_instance": true,
                            "topology": {
                                "enabled": true,
                                "type": "top-level",
                                "components": [],
                                "connectableProducts": []
                            }
                        },
                        {
                            "product_name": "ESS",
                            "parent_name": "Station",
                            "can_create_instance": false,
                            "topology": {
                                "enabled": false,
                                "components": [],
                                "connectableProducts": []
                            }
                        }
                    ]
                }
            })
        )
    )
))]
pub async fn list_products(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProductListQuery>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    // Products are compile-time constants, no async needed
    let product_definitions = state.instance_manager.product_loader().get_all_products();

    let products: Vec<serde_json::Value> = product_definitions
        .into_iter()
        .filter(|product| query.matches(product))
        .map(|product| {
            json!({
                "product_name": product.product_name,
                "parent_name": product.parent_name,
                "can_create_instance": product.can_create_instance,
                "topology": product.topology
            })
        })
        .collect();

    Ok(Json(SuccessResponse::new(json!({
        "count": products.len(),
        "products": products
    }))))
}

/// Get product definition with nested structure
///
/// Returns detailed product information including all measurement,
/// action, and property points.
///
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/products/{product_name}/points",
    tag = "products",
    params(
        ("product_name" = String, Path, description = "Product identifier")
    ),
    responses(
        (status = 200, description = "Product details with all points retrieved successfully",
            body = inline(Object),
            example = json!({
                "success": true,
                "data": {
                    "product": {
                        "product_name": "Battery",
                        "parent_name": "ESS",
                        "can_create_instance": true,
                        "topology": {
                            "enabled": true,
                            "type": "standalone",
                            "image": "device-Battery.png",
                            "components": [],
                            "connectableProducts": ["Hybrid_Inverter", "PCS"]
                        },
                        "measurements": [
                            {"measurement_id": 1, "name": "SOC", "unit": "%", "description": null}
                        ],
                        "actions": [
                            {"action_id": 1, "name": "Charge", "unit": null, "description": null}
                        ],
                        "properties": []
                    }
                }
            })
        ),
        (status = 404, description = "Product not found")
    )
))]
pub async fn get_product_points(
    State(state): State<Arc<AppState>>,
    Path(product_name): Path<String>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ModSrvError> {
    // Products are compile-time constants, no async needed
    match state
        .instance_manager
        .product_loader()
        .get_product(&product_name)
    {
        Ok(product) => Ok(Json(SuccessResponse::new(json!({
            "product": product
        })))),
        Err(e) => Err(ModSrvError::InternalError(format!(
            "Not found: Product '{}' not found ({})",
            product_name, e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TopologyDefinition, TopologyType};

    fn product(topology_enabled: bool, can_create_instance: bool) -> Product {
        Product {
            product_name: "Test".to_string(),
            parent_name: None,
            can_create_instance,
            topology: TopologyDefinition {
                enabled: topology_enabled,
                topology_type: topology_enabled.then_some(TopologyType::Standalone),
                image: None,
                components: Vec::new(),
                connectable_products: Vec::new(),
            },
            measurements: Vec::new(),
            actions: Vec::new(),
            properties: Vec::new(),
        }
    }

    #[test]
    fn product_list_query_without_filters_matches_all() {
        let query = ProductListQuery::default();
        assert!(query.matches(&product(false, true)));
        assert!(query.matches(&product(true, false)));
    }

    #[test]
    fn product_list_query_filters_single_capability() {
        let query = ProductListQuery {
            can_create_instance: Some(true),
            topology_enabled: None,
        };
        assert!(query.matches(&product(false, true)));
        assert!(!query.matches(&product(false, false)));
    }

    #[test]
    fn product_list_query_combines_filters() {
        let query = ProductListQuery {
            can_create_instance: Some(false),
            topology_enabled: Some(false),
        };
        assert!(query.matches(&product(false, false)));
        assert!(!query.matches(&product(true, false)));
        assert!(!query.matches(&product(false, true)));
    }
}
