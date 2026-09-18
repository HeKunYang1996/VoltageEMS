//! Product API Integration Tests
//!
//! Tests the product management functionality with compile-time built-in products:
//! - Lightweight product name listing
//! - Detailed product information with measurements/actions/properties
//!
//! Products are now embedded at compile time from voltage-model crate.

#![allow(clippy::disallowed_methods)] // Integration test - unwrap is acceptable

mod common;

use anyhow::Result;
use common::TestEnv;
use modsrv::product_loader::ProductLoader;

#[tokio::test]
#[ignore] // requires Redis
async fn test_product_list_lightweight() -> Result<()> {
    // 1. Create test environment
    let env = TestEnv::create().await?;

    // 2. Create product loader (products are compile-time constants)
    let product_loader = ProductLoader::new(env.pool().clone());

    // 3. Call get_all_product_names (lightweight method)
    // Products are now compile-time constants from voltage-model crate
    let product_names = product_loader.get_all_product_names();

    // 4. Verify specific built-in products exist
    let battery = product_names
        .iter()
        .find(|(name, _)| name == "Battery")
        .expect("Should find Battery");
    assert_eq!(battery.1, None);

    let station = product_names
        .iter()
        .find(|(name, _)| name == "Station")
        .expect("Should find Station");
    assert_eq!(station.1, None, "Station should be a root product");

    assert!(
        product_names.iter().all(|(name, _)| name != "ESS"),
        "ESS is a product classification, not a built-in product"
    );

    // 6. Cleanup
    env.cleanup().await?;

    Ok(())
}

#[tokio::test]
#[ignore] // requires Redis
async fn test_product_detail_complete() -> Result<()> {
    // 1. Create test environment
    let env = TestEnv::create().await?;

    // 2. Create product loader
    let product_loader = ProductLoader::new(env.pool().clone());

    // 3. Call get_product for Battery (detailed method)
    let product = product_loader
        .get_product("Battery")
        .expect("get_product should succeed for Battery");

    // 4. Verify complete response structure
    assert_eq!(product.product_name, "Battery");
    assert_eq!(product.product_type, "ESS");
    assert!(product.topology.is_some());

    // 5. Verify measurements exist
    assert!(
        !product.measurements.is_empty(),
        "Battery should have measurements"
    );

    // 6. Verify actions exist
    assert!(!product.actions.is_empty(), "Battery should have actions");

    // 7. Cleanup
    env.cleanup().await?;

    Ok(())
}

#[tokio::test]
#[ignore] // requires Redis
async fn test_product_closed_loop() -> Result<()> {
    // 1. Create test environment
    let env = TestEnv::create().await?;

    // 2. Create product loader
    let product_loader = ProductLoader::new(env.pool().clone());

    // 3. STEP 1: Get product list (lightweight)
    let product_names = product_loader.get_all_product_names();

    // 4. STEP 2: For each product, fetch detailed information
    for (product_name, _) in &product_names {
        // Fetch detail
        let product = product_loader
            .get_product(product_name)
            .unwrap_or_else(|_| panic!("get_product should succeed for {}", product_name));

        // Verify product name matches
        assert_eq!(
            &product.product_name, product_name,
            "Product name should match"
        );
        assert!(!product.product_type.is_empty());
    }

    // 5. Verify specific products
    let battery = product_loader.get_product("Battery")?;
    assert_eq!(battery.product_type, "ESS");

    let pcs = product_loader.get_product("PCS")?;
    assert_eq!(pcs.product_type, "ESS");

    // 6. Cleanup
    env.cleanup().await?;

    Ok(())
}

#[tokio::test]
#[ignore] // requires Redis
async fn test_product_not_found() -> Result<()> {
    // 1. Create test environment
    let env = TestEnv::create().await?;

    // 2. Create product loader
    let product_loader = ProductLoader::new(env.pool().clone());

    // 3. Try to get a non-existent product
    let result = product_loader.get_product("nonexistent_product");

    // 4. Verify error
    assert!(
        result.is_err(),
        "Should return error for non-existent product"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("not found") || error_msg.contains("Product not found"),
        "Error message should indicate not found: {}",
        error_msg
    );

    // 5. Cleanup
    env.cleanup().await?;

    Ok(())
}

#[tokio::test]
#[ignore] // requires Redis
async fn test_product_names_have_no_legacy_hierarchy() -> Result<()> {
    // 1. Create test environment
    let env = TestEnv::create().await?;

    // 2. Create product loader
    let product_loader = ProductLoader::new(env.pool().clone());

    // 3. Get product list
    let product_names = product_loader.get_all_product_names();

    // 4. The legacy parent relationship was removed from the product contract.
    assert!(
        product_names.iter().all(|(_, parent)| parent.is_none()),
        "All products should be reported as roots"
    );
    assert!(product_names.iter().any(|(name, _)| name == "Station"));
    assert!(product_names.iter().any(|(name, _)| name == "Battery"));
    assert!(product_names.iter().any(|(name, _)| name == "PCS"));
    assert!(product_names.iter().all(|(name, _)| name != "ESS"));

    // 5. Cleanup
    env.cleanup().await?;

    Ok(())
}

#[tokio::test]
#[ignore] // requires Redis
async fn test_product_exists() -> Result<()> {
    // 1. Create test environment
    let env = TestEnv::create().await?;

    // 2. Create product loader
    let product_loader = ProductLoader::new(env.pool().clone());

    // 3. Verify built-in products exist
    assert!(product_loader.product_exists("Battery"));
    assert!(product_loader.product_exists("PCS"));
    assert!(product_loader.product_exists("Station"));

    // 4. Verify classifications, removed placeholders, and unknown products don't exist
    assert!(!product_loader.product_exists("ESS"));
    assert!(!product_loader.product_exists("Generator"));
    assert!(!product_loader.product_exists("NonExistentProduct"));
    assert!(!product_loader.product_exists("FakeProduct"));

    // 5. Cleanup
    env.cleanup().await?;

    Ok(())
}
