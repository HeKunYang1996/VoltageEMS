WITH requested AS (
    SELECT DISTINCT redis_key, point_id
    FROM UNNEST($3::TEXT[], $4::TEXT[]) AS s(redis_key, point_id)
), last_samples AS (
    SELECT DISTINCT ON (h.redis_key, h.point_id, bucket)
           h.redis_key, h.point_id,
           LEAST(
               FLOOR(EXTRACT(EPOCH FROM (h.time - $1::timestamptz)) / $5::bigint)::bigint,
               $6::bigint - 1
           ) AS bucket,
           h.value
    FROM history h
    JOIN requested r USING (redis_key, point_id)
    WHERE h.time >= $1::timestamptz AND h.time <= $2::timestamptz
    ORDER BY h.redis_key, h.point_id, bucket, h.time DESC
)
SELECT $1::timestamptz + (b.bucket * $5::bigint) * INTERVAL '1 second' AS time,
       r.redis_key, r.point_id, s.value
FROM requested r
CROSS JOIN generate_series(0::bigint, $6::bigint - 1) AS b(bucket)
LEFT JOIN last_samples s
       ON s.redis_key = r.redis_key AND s.point_id = r.point_id AND s.bucket = b.bucket
ORDER BY r.redis_key, r.point_id, time ASC
