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
    pub topology_enabled: Option<bool>,
}

impl ProductListQuery {
    fn matches(&self, product: &Product) -> bool {
        self.topology_enabled
            .is_none_or(|expected| product.topology.is_some() == expected)
    }
}

/// List all available product templates (lightweight)
///
/// Returns product names, classifications, descriptions, display defaults,
/// and topology capabilities.
/// This endpoint is optimized for frontend dropdown lists and product selection interfaces.
/// For detailed product information including measurements/actions/properties, use GET /api/products/{product_name}/points.
///
#[cfg_attr(feature = "swagger-ui", utoipa::path(
    get,
    path = "/api/products",
    tag = "products",
    params(
        ("topology_enabled" = Option<bool>, Query, description = "Filter by whether the product participates in the energy topology")
    ),
    responses(
        (status = 200, description = "Lightweight product list retrieved successfully",
            body = inline(Object),
            example = json!({
                "success": true,
                "data": {
                "count": 13,
                    "products": [
                        {
                            "product_name": "Station",
                            "type": "Station",
                            "description": "Station profile",
                            "topology": null,
                            "defaultDisplayMeasureIds": [1, 2]
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
                "type": product.product_type,
                "description": product.description,
                "topology": product.topology,
                "defaultDisplayMeasureIds": product.default_display_measure_ids
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
                        "type": "ESS",
                        "description": "Battery energy storage device.",
                        "defaultDisplayMeasureIds": [1, 3, 4],
                        "topology": {
                            "image": "device-Battery.png",
                            "connections": [{"products": ["Hybrid_Inverter", "PCS"], "min": 1, "max": 1}]
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
    use crate::config::TopologyDefinition;

    fn product(topology_enabled: bool) -> Product {
        Product {
            product_name: "Test".to_string(),
            product_type: "Test".to_string(),
            description: None,
            default_display_measure_ids: Vec::new(),
            topology: topology_enabled.then_some(TopologyDefinition {
                image: None,
                connections: Vec::new(),
                description: None,
            }),
            measurements: Vec::new(),
            actions: Vec::new(),
            properties: Vec::new(),
        }
    }

    #[test]
    fn product_list_query_without_filters_matches_all() {
        let query = ProductListQuery::default();
        assert!(query.matches(&product(false)));
        assert!(query.matches(&product(true)));
    }

    #[test]
    fn product_list_query_filters_single_capability() {
        let query = ProductListQuery {
            topology_enabled: Some(true),
        };
        assert!(query.matches(&product(true)));
        assert!(!query.matches(&product(false)));
    }
}
