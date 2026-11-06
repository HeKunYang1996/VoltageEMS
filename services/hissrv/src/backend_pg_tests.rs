use super::*;
use sqlx::postgres::PgPoolOptions;

// Each pool has one connection with a private temporary history table. The
// fixture never changes an existing history table or needs Redis/field devices.
async fn backend() -> PostgresBackend {
    let url = std::env::var("HISSRV_TEST_DATABASE_URL").expect("dedicated test PostgreSQL URL");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query(
        "CREATE TEMP TABLE history (time TIMESTAMPTZ NOT NULL, redis_key TEXT, \
         point_id TEXT, value DOUBLE PRECISION, string_value TEXT)",
    )
    .execute(&pool)
    .await
    .unwrap();
    PostgresBackend::new(pool)
}

fn start() -> DateTime<Utc> {
    "2026-11-05T00:00:00Z".parse().unwrap()
}

async fn seed(backend: &PostgresBackend, key: &str, point: &str, count: i64) {
    let points = (0..count)
        .map(|i| DataPoint {
            time: start() + Duration::seconds(i * 30),
            redis_key: key.into(),
            point_id: point.into(),
            value: Some(i as f64),
            string_value: None,
        })
        .collect();
    backend.write_batch(points).await.unwrap();
}

#[tokio::test]
#[ignore = "requires HISSRV_TEST_DATABASE_URL pointing to a test PostgreSQL server"]
async fn capped_history_spans_full_day_and_preserves_raw_samples() {
    let backend = backend().await;
    seed(&backend, "inst:1:M", "1", 2881).await;
    let end = start() + Duration::days(1);
    let result = backend
        .query_batch(&[("inst:1:M".into(), "1".into())], start(), end, 300, None)
        .await
        .unwrap();
    let data = &result[0].data;
    assert_eq!(result[0].count, 300);
    assert_eq!(data.first().unwrap().time, fmt_ts(&start()));
    assert_eq!(data.last().unwrap().time, fmt_ts(&end));
    assert_eq!(data.first().unwrap().value, Some(0.0));
    assert_eq!(data.last().unwrap().value, Some(2880.0));
    for pair in data.windows(2) {
        assert!(pair[0].time < pair[1].time);
    }
    for point in data {
        let index = (parse_time(&point.time).unwrap() - start()).num_seconds() / 30;
        assert_eq!(point.value, Some(index as f64));
    }
    backend.pool.close().await;
}

#[tokio::test]
#[ignore = "requires HISSRV_TEST_DATABASE_URL pointing to a test PostgreSQL server"]
async fn sampling_handles_small_budgets_sparse_series_and_missing_data() {
    let backend = backend().await;
    seed(&backend, "inst:1:M", "dense", 301).await;
    seed(&backend, "inst:2:M", "sparse", 3).await;
    let series = vec![
        ("inst:2:M".into(), "sparse".into()),
        ("inst:1:M".into(), "dense".into()),
        ("inst:3:M".into(), "missing".into()),
    ];
    for limit in [1, 2, 3, 300, 301, 5000] {
        let result = backend
            .query_batch(&series, start(), start() + Duration::days(1), limit, None)
            .await
            .unwrap();
        for (index, total) in [(0, 3), (1, 301)] {
            let data = &result[index].data;
            assert_eq!(data.len(), total.min(limit as usize));
            assert_eq!(result[index].redis_key, series[index].0);
            assert_eq!(data.last().unwrap().value, Some((total - 1) as f64));
            if limit > 1 {
                assert_eq!(data.first().unwrap().value, Some(0.0));
            }
        }
        assert_eq!(result[2].count, 0);
        assert!(result[2].data.is_empty());
    }
    backend.pool.close().await;
}

#[tokio::test]
#[ignore = "requires HISSRV_TEST_DATABASE_URL pointing to a test PostgreSQL server"]
async fn sampling_respects_requested_time_bounds_and_pair_filters() {
    let backend = backend().await;
    seed(&backend, "inst:1:M", "1", 30).await;
    seed(&backend, "inst:1:M", "2", 40).await;
    let begin = start() + Duration::seconds(90);
    let end = start() + Duration::seconds(600);
    let result = backend
        .query_batch(&[("inst:1:M".into(), "1".into())], begin, end, 5, None)
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].count, 5);
    assert_eq!(result[0].data.first().unwrap().time, fmt_ts(&begin));
    assert_eq!(result[0].data.last().unwrap().time, fmt_ts(&end));
    assert!(
        backend
            .query_batch(&[], begin, end, 5, None)
            .await
            .unwrap()
            .is_empty()
    );
    let outside = backend
        .query_batch(
            &[("inst:1:M".into(), "1".into())],
            end + Duration::days(1),
            end + Duration::days(2),
            5,
            None,
        )
        .await
        .unwrap();
    assert_eq!(outside[0].count, 0);
    backend.pool.close().await;
}

#[tokio::test]
#[ignore = "requires HISSRV_TEST_DATABASE_URL pointing to a test PostgreSQL server"]
async fn interval_query_covers_full_day_without_legacy_truncation() {
    let backend = backend().await;
    seed(&backend, "fixture", "1", 2881).await;
    let end = start() + Duration::days(1);
    // One minute creates 1440 buckets even though the legacy cap is only 300.
    let result = backend
        .query_batch(
            &[("fixture".into(), "1".into())],
            start(),
            end,
            300,
            Some(60),
        )
        .await
        .unwrap();
    assert_eq!(result[0].count, 1440);
    assert_eq!(result[0].data[0].time, fmt_ts(&start()));
    assert_eq!(result[0].data[0].value, Some(1.0));
    let last = result[0].data.last().unwrap();
    assert_eq!(last.time, fmt_ts(&(end - Duration::seconds(60))));
    assert_eq!(last.value, Some(2880.0));
    for (index, point) in result[0].data.iter().enumerate() {
        assert_eq!(
            point.time,
            fmt_ts(&(start() + Duration::seconds(index as i64 * 60)))
        );
    }
    backend.pool.close().await;
}

#[tokio::test]
#[ignore = "requires HISSRV_TEST_DATABASE_URL pointing to a test PostgreSQL server"]
async fn interval_query_keeps_gaps_boundaries_and_null_last_readings() {
    let backend = backend().await;
    let points = [
        (301, None),
        (-1, Some(999.0)),
        (299, Some(2.0)),
        (0, Some(1.0)),
        (300, Some(3.0)),
        (900, Some(5.0)),
        (901, Some(6.0)),
        (902, Some(999.0)),
    ]
    .into_iter()
    .map(|(second, value)| DataPoint {
        time: start() + Duration::seconds(second),
        redis_key: "fixture".into(),
        point_id: "1".into(),
        value,
        string_value: None,
    })
    .collect();
    backend.write_batch(points).await.unwrap();
    let series = [
        ("fixture".into(), "missing".into()),
        ("fixture".into(), "1".into()),
        ("fixture".into(), "1".into()),
    ];
    let end = start() + Duration::seconds(901);
    let result = backend
        .query_batch(&series, start(), end, 1, Some(300))
        .await
        .unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].count, 4);
    assert!(result[0].data.iter().all(|point| point.value.is_none()));
    for item in &result[1..] {
        assert_eq!(item.count, 4);
        assert_eq!(
            item.data
                .iter()
                .map(|point| point.value)
                .collect::<Vec<_>>(),
            vec![Some(2.0), None, None, Some(6.0)]
        );
        for (index, point) in item.data.iter().enumerate() {
            assert_eq!(
                point.time,
                fmt_ts(&(start() + Duration::seconds(index as i64 * 300)))
            );
        }
    }
    // Exact end-time samples belong to the final bucket, without an extra bucket.
    let exact = backend
        .query_batch(
            &series[1..2],
            start(),
            start() + Duration::seconds(900),
            1,
            Some(300),
        )
        .await
        .unwrap();
    assert_eq!(exact[0].count, 3);
    assert_eq!(exact[0].data.last().unwrap().value, Some(5.0));
    // Very large valid intervals must not overflow when forming bucket timestamps.
    let huge = backend
        .query_batch(&series[1..2], start(), end, 1, Some(i64::MAX))
        .await
        .unwrap();
    assert_eq!(huge[0].count, 1);
    assert_eq!(huge[0].data[0].value, Some(6.0));
    backend.pool.close().await;
}
