/**
 * 格式化数值显示
 * 如果一个数的小数位大于三，展示数据时需要保留三位小数，否则保持原样
 * @param value 要格式化的值
 * @returns 格式化后的字符串
 */
/** Backend uses ts <= 0 when no valid sample exists (SHM init / missing :ts sidecar). */
export function parseSampleTimestamp(ts: number | string | null | undefined): number | null {
  if (ts === null || ts === undefined) return null
  const n = typeof ts === 'string' ? Number(ts) : ts
  if (!Number.isFinite(n) || n <= 0) return null
  return n
}

/** Format a live measurement only when its sample timestamp is valid. */
export function formatSampleValue(
  value: number | string | null | undefined,
  ts: number | string | null | undefined,
): string {
  if (parseSampleTimestamp(ts) === null) return '-'
  return formatNumber(value)
}

export function formatNumber(value: number | string | null | undefined): string {
  if (value === null || value === undefined) return '-'
  if (typeof value === 'string') {
    const numValue = parseFloat(value)
    if (isNaN(numValue)) return value
    value = numValue
  }
  const str = String(value)
  const decimalIndex = str.indexOf('.')
  if (decimalIndex === -1) {
    // 没有小数部分，直接返回
    return str
  }
  const decimalPart = str.substring(decimalIndex + 1)
  if (decimalPart.length > 3) {
    // 小数位大于3位，保留3位小数
    return value.toFixed(3)
  }
  // 小数位小于等于3位，保持原样
  return str
}

/**
 * 格式化时间戳为本地时间字符串
 * @param timestamp 时间戳（毫秒）
 * @returns 格式化后的时间字符串
 */
export function formatTimestamp(timestamp: number | string | null | undefined): string {
  const ts = parseSampleTimestamp(timestamp)
  if (ts === null) return '-'
  try {
    return new Date(ts).toLocaleString()
  } catch {
    return '-'
  }
}
