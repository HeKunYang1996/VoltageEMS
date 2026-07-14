/**
 * Forecast 预测模块类型定义
 * 对应 Swagger: forecastsrv 0.1.0
 */

/** 预测摘要 */
export interface ForecastSummary {
  total_pv_generation: number
  total_load: number
  self_consumption_ratio: number
  net_power: number
  last_updated: string
  pv_generation_vs_yesterday_pct: number | null
  load_vs_yesterday_pct: number | null
  self_consumption_vs_yesterday_pct: number | null
  net_power_vs_yesterday_pct: number | null
  battery_soc: number | null
  weather: WeatherData | null
}

/** 天气数据 */
export interface WeatherData {
  temperature: number | null
  humidity: number | null
  ghi: number | null
  dni: number | null
  dhi: number | null
  cloud_cover: number | null
  wind_speed: number | null
  weather_code: string | null
  weather_text: string | null
}

/** 时序预测点 */
export interface TimeSeriesPoint {
  ts: string
  predicted: number
  actual: number | null
  p10: number | null
  p50: number | null
  p90: number | null
  weather: WeatherData | null
}

/** 时序预测响应 */
export interface TimeSeriesResponse {
  data: TimeSeriesPoint[]
  target_type: 'pv' | 'load'
  message: string | null
}

/** 准确率指标 */
export interface AccuracyMetrics {
  mae: number
  mape: number
  rmse: number
  sample_count: number
}

/** 准确率响应 */
export interface AccuracyResponse {
  pv: AccuracyMetrics
  load: AccuracyMetrics
  period_days: number
}

/** 调度建议 */
export interface SchedulingSuggestion {
  id: string
  title: string
  description: string
  recommended_action: string
  suggestion_type: string
  priority: string
  status: string
  created_at: string
}

/** 模型状态 */
export interface ModelStatus {
  pv_model_version: string | null
  load_model_version: string | null
  pv_quantile_model_version: string | null
  load_quantile_model_version: string | null
  sync_status: string
  last_sync_time: string | null
}