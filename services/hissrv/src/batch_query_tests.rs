use super::*;
use chrono::Duration;

fn start() -> DateTime<Utc> {
    "2026-11-05T00:00:00Z".parse().unwrap()
}

#[test]
fn bucket_count_keeps_partial_and_subsecond_windows() {
    let begin = start();
    assert_eq!(
        interval_bucket_count(begin, begin + Duration::days(1), 300).unwrap(),
        288
    );
    assert_eq!(
        interval_bucket_count(begin, begin + Duration::days(1), 60).unwrap(),
        1440
    );
    assert_eq!(
        interval_bucket_count(begin, begin + Duration::seconds(901), 300).unwrap(),
        4
    );
    assert_eq!(
        interval_bucket_count(begin, begin + Duration::milliseconds(1), 300).unwrap(),
        1
    );
    assert_eq!(
        interval_bucket_count(begin, begin + Duration::seconds(901), i64::MAX).unwrap(),
        1
    );
}

#[test]
fn interval_validation_rejects_invalid_and_excessive_ranges() {
    let begin = start();
    let end = begin + Duration::seconds(5000);
    assert_eq!(
        interval_bucket_count(begin, end, 1).unwrap(),
        MAX_BATCH_POINTS
    );
    assert!(interval_bucket_count(begin, end + Duration::milliseconds(1), 1).is_err());
    assert!(interval_bucket_count(begin, end, 0).is_err());
    assert!(interval_bucket_count(begin, end, -1).is_err());
    assert!(interval_bucket_count(begin, begin, 1).is_err());
    assert!(interval_bucket_count(end, begin, 1).is_err());
}

#[test]
fn legacy_request_and_response_remain_compatible() {
    let request: BatchQueryRequest = serde_json::from_value(serde_json::json!({
        "start_time": "2026-11-05T00:00:00Z", "end_time": "2026-11-06T00:00:00Z",
        "series": [{"redis_key": "fixture", "point_id": "1"}], "limit_per_series": 300,
    }))
    .unwrap();
    assert_eq!(request.interval_seconds, None);
    assert_eq!(request.limit_per_series, Some(300));
    let response = BatchQueryResponse {
        start_time: request.start_time,
        end_time: request.end_time,
        series: vec![],
        interval_seconds: None,
        aggregation: None,
    };
    let json = serde_json::to_value(response).unwrap();
    assert!(json.get("interval_seconds").is_none());
    assert!(json.get("aggregation").is_none());
    assert_eq!(json.as_object().unwrap().len(), 3);
}

#[test]
fn interval_request_and_response_expose_time_granularity() {
    let request: BatchQueryRequest = serde_json::from_value(serde_json::json!({
        "start_time": "2026-11-05T00:00:00Z", "end_time": "2026-11-06T00:00:00Z",
        "series": [{"redis_key": "fixture", "point_id": "1"}], "interval_seconds": 300,
    }))
    .unwrap();
    assert_eq!(request.interval_seconds, Some(300));
    assert_eq!(request.limit_per_series, None);
    let response = BatchQueryResponse {
        start_time: request.start_time,
        end_time: request.end_time,
        series: vec![],
        interval_seconds: request.interval_seconds,
        aggregation: Some("last"),
    };
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(json["interval_seconds"], 300);
    assert_eq!(json["aggregation"], "last");
}
