import type { InstanceMeasurementItem } from '@/types/deviceConfiguration'
import type {
  BatchQueryData,
  BatchQueryRequest,
  BatchQueryResponse,
  BatchSeriesItem,
} from '@/types/Statistics/OverView'

export type StatisticMetricName =
  | 'pvPower' | 'pvEnergy' | 'essPower' | 'essEnergy' | 'soc'
  | 'dgPower' | 'dgEnergy' | 'loadEnergy'

export interface StatisticMetric {
  instanceId: number
  terms: { pointId: string; factor: number }[]
  counter: boolean
}

export type StatisticMetrics = Partial<Record<StatisticMetricName, StatisticMetric>>
/** Maximum bucket count allowed by hissrv batch-query. */
export const MAX_HISTORY_SEGMENTS = 5000
/** @deprecated Use interval_seconds via resolveHistoryIntervalSeconds instead. */
export const STATISTIC_HISTORY_LIMIT = MAX_HISTORY_SEGMENTS
/** Target chart points for Statistics/Curves (formerly limit_per_series: 300). */
export const CURVES_HISTORY_TARGET_POINTS = 300

export interface HistoryQueryMeta {
  start_time: string
  end_time: string
  interval_seconds: number
}

export function expectedHistorySegmentCount(
  startTime: string,
  endTime: string,
  intervalSeconds: number,
): number {
  const spanMs = Date.parse(endTime) - Date.parse(startTime)
  if (!Number.isFinite(spanMs) || spanMs <= 0 || intervalSeconds <= 0) return 0
  return Math.floor(spanMs / (intervalSeconds * 1000))
}

/** Pick interval_seconds so bucket count stays within maxPoints (default 5000). */
export function resolveHistoryIntervalSeconds(
  startTime: string,
  endTime: string,
  maxPoints = MAX_HISTORY_SEGMENTS,
): number {
  const spanSec = Math.max(
    1,
    Math.floor((Date.parse(endTime) - Date.parse(startTime)) / 1000),
  )
  const cap = Math.max(1, Math.min(maxPoints, MAX_HISTORY_SEGMENTS))
  return Math.max(1, Math.ceil(spanSec / cap))
}

export function buildBatchQueryRequest(
  range: { start_time: string; end_time: string },
  series: BatchSeriesItem[],
  maxPoints = MAX_HISTORY_SEGMENTS,
): BatchQueryRequest {
  return {
    ...range,
    interval_seconds: resolveHistoryIntervalSeconds(range.start_time, range.end_time, maxPoints),
    series,
  }
}

export function historyQueryMetaFromResponse(
  range: { start_time: string; end_time: string },
  data?: BatchQueryData | null,
): HistoryQueryMeta {
  return {
    start_time: range.start_time,
    end_time: range.end_time,
    interval_seconds:
      data?.interval_seconds ??
      resolveHistoryIntervalSeconds(range.start_time, range.end_time),
  }
}

function unitFactor(unit: string | null, expected: 'power' | 'energy' | 'soc'): number | undefined {
  const units = expected === 'power' ? { w: 0.001, kw: 1, mw: 1000 }
    : expected === 'energy' ? { wh: 0.001, kwh: 1, mwh: 1000 } : { '%': 1 }
  return units[(unit ?? '').trim().toLowerCase() as keyof typeof units]
}

/** Names and units come from the selected instance, never another product's IDs. */
export function resolveStatisticMetric(
  instanceId: number | undefined,
  measurements: InstanceMeasurementItem[],
  names: readonly string[],
  expected: 'power' | 'energy' | 'soc',
  coefficients: readonly number[] = [1],
): StatisticMetric | undefined {
  if (instanceId === undefined) return undefined
  const terms: StatisticMetric['terms'] = []
  for (const [index, name] of names.entries()) {
    const point = measurements.find((item) => item.name === name)
    if (!point) return undefined
    const factor = unitFactor(point.unit, expected)
    if (factor === undefined) return undefined
    terms.push({ pointId: String(point.measurement_id), factor: factor * (coefficients[index] ?? 1) })
  }
  return { instanceId, terms, counter: expected === 'energy' }
}

export function statisticRequests(metrics: StatisticMetrics): BatchSeriesItem[] {
  const series = new Map<string, BatchSeriesItem>()
  for (const metric of Object.values(metrics)) {
    if (!metric) continue
    for (const term of metric.terms) {
      const item = { redis_key: `inst:${metric.instanceId}:M`, point_id: term.pointId }
      series.set(`${item.redis_key}:${item.point_id}`, item)
    }
  }
  return [...series.values()]
}

function finiteValue(value: unknown): number | null {
  if (typeof value !== 'number' && (typeof value !== 'string' || !value.trim())) return null
  const numeric = Number(value)
  return Number.isFinite(numeric) ? numeric : null
}

export function statisticLiveValue(
  metric: StatisticMetric | undefined,
  values: Record<string, unknown>,
): number | null {
  if (!metric) return null
  let total = 0
  for (const term of metric.terms) {
    const value = finiteValue(values[term.pointId])
    if (value === null) return null
    total += value * term.factor
  }
  return total
}

/** Difference consecutive counter readings; reset/missing intervals remain unknown. */
export function counterIntervals(values: (number | null)[]): (number | null)[] {
  return values.map((value, index) => {
    const previous = values[index - 1]
    return value === null || previous === undefined || previous === null || value < previous
      ? null : value - previous
  })
}

export function statisticHistory(
  metric: StatisticMetric | undefined,
  responses: BatchQueryResponse[],
  query?: HistoryQueryMeta,
): { values: Map<string, number | null>; total: number | null; coverage: { start: string; end: string } | null } {
  if (!metric) return { values: new Map(), total: null, coverage: null }
  const series = metric.terms.map((term) => responses.find((item) =>
    item.redis_key === `inst:${metric.instanceId}:M` && item.point_id === term.pointId))
  const timestamps = [...new Set(series.flatMap((item) => item?.data.map((p) => p.timestamp) ?? []))]
    .sort((a, b) => Date.parse(a) - Date.parse(b))
  const maps = series.map((item) => new Map(item?.data.map((p) => [p.timestamp, p.value]) ?? []))
  const samples = timestamps.map((timestamp) => {
    let total = 0
    for (const [index, term] of metric.terms.entries()) {
      const value = finiteValue(maps[index].get(timestamp))
      if (value === null) return null
      total += value * term.factor
    }
    return total
  })
  const values = metric.counter ? counterIntervals(samples) : samples
  const expected =
    query === undefined
      ? undefined
      : expectedHistorySegmentCount(query.start_time, query.end_time, query.interval_seconds)
  // Aggregated buckets must cover the full requested window before counter totals are trusted.
  const complete =
    expected !== undefined &&
    expected > 0 &&
    series.every((item) => item && item.count === expected)
  const intervals = values.slice(1)
  const total = metric.counter && complete && intervals.length && intervals.every((v) => v !== null)
    ? intervals.reduce<number>((sum, value) => sum + (value ?? 0), 0) : null
  const coverage = total === null ? null : { start: timestamps[0], end: timestamps[timestamps.length - 1] }
  return { values: new Map(timestamps.map((timestamp, index) => [timestamp, values[index]])), total, coverage }
}

export function buildStatisticChartData(
  metrics: StatisticMetrics,
  responses: BatchQueryResponse[],
  query?: HistoryQueryMeta,
) {
  const histories = Object.fromEntries(Object.entries(metrics).map(([name, metric]) =>
    [name, statisticHistory(metric, responses, query)])) as Partial<Record<StatisticMetricName, ReturnType<typeof statisticHistory>>>
  const timestamps = [...new Set(Object.values(histories).flatMap((history) => [...history.values.keys()]))]
    .sort((a, b) => Date.parse(a) - Date.parse(b))
  const values = (name: StatisticMetricName) => timestamps.map((timestamp) => {
    const value = histories[name]?.values.get(timestamp)
    return value === undefined || value === null ? null : Number(value.toFixed(3))
  })
  return { timestamps, values, loadEnergy: histories.loadEnergy?.total ?? null,
    loadEnergyCoverage: histories.loadEnergy?.coverage ?? null }
}
