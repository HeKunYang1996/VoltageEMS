SELECT time, redis_key, point_id, value
FROM (
    SELECT time, redis_key, point_id, value,
           ROW_NUMBER() OVER (
               PARTITION BY redis_key, point_id ORDER BY time ASC
           ) AS rn,
           COUNT(*) OVER (PARTITION BY redis_key, point_id) AS total
    FROM history
    WHERE time >= $1 AND time <= $2
      AND (redis_key, point_id) IN (
          SELECT * FROM UNNEST($3::TEXT[], $4::TEXT[])
      )
) ranked
WHERE total <= $5
   OR ($5 = 1 AND rn = total)
   OR ($5 > 1 AND (
       rn = 1
       OR (rn - 1) * ($5 - 1) / NULLIF(total - 1, 0)
          > (rn - 2) * ($5 - 1) / NULLIF(total - 1, 0)
   ))
ORDER BY redis_key, point_id, time ASC
