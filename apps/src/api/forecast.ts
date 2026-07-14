/**
 * Forecast 预测模块 API
 * 对应 forecastsrv 服务（http://192.168.30.10:36008）
 * 响应格式为裸 JSON（如 {"total_pv_generation": 489.33, ...}），
 * 不走 /api 网关的 {code/message/data/success} 封装。
 *
 * 因此不能使用主应用的 Request/service（其 response interceptor 会校验
 * data.code/data.success/data.status），需要独立的 axios 实例。
 */

import axios from 'axios'
import type {
  ForecastSummary,
  TimeSeriesResponse,
  AccuracyResponse,
  SchedulingSuggestion,
} from '@/types/forecast'

/**
 * 独立的 axios 实例：
 * - 不加 response interceptor，直接透传裸 JSON
 * - baseURL 为 /forecastApi，由 vite proxy 转发到 http://192.168.30.10:36008
 * - GET 请求自动带 _t 防缓存（与主应用一致），但 forecast 无需 token，故不加 Authorization
 */
const forecastService = axios.create({
  baseURL: '/forecastApi',
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' },
})

/** 基础路径，通过 vite proxy 转发到 http://192.168.30.10:36008 */
const BASE = '/api/forecast'

/**
 * 包装 GET 请求，直接取 response.data（裸 JSON）
 */
async function rawGet<T>(url: string, params?: Record<string, any>): Promise<T> {
  const res = await forecastService.get(url, { params })
  return res.data as T
}

/** 预测摘要 */
export function fetchForecastSummary(): Promise<ForecastSummary> {
  return rawGet<ForecastSummary>(`${BASE}/summary`)
}

/** PV 预测时序 */
export function fetchForecastPV(startTime?: string, endTime?: string): Promise<TimeSeriesResponse> {
  return rawGet<TimeSeriesResponse>(`${BASE}/pv`, { start_time: startTime, end_time: endTime })
}

/** 负荷预测时序 */
export function fetchForecastLoad(startTime?: string, endTime?: string): Promise<TimeSeriesResponse> {
  return rawGet<TimeSeriesResponse>(`${BASE}/load`, { start_time: startTime, end_time: endTime })
}

/** 预测准确率 */
export function fetchForecastAccuracy(days = 7): Promise<AccuracyResponse> {
  return rawGet<AccuracyResponse>(`${BASE}/accuracy`, { days })
}

/** 调度建议列表 */
export function fetchForecastSuggestions(): Promise<SchedulingSuggestion[]> {
  return rawGet<SchedulingSuggestion[]>(`${BASE}/suggestions`)
}

/** 调度建议详情 */
export function fetchForecastSuggestionDetail(id: string): Promise<SchedulingSuggestion> {
  return rawGet<SchedulingSuggestion>(`${BASE}/suggestions/${id}`)
}