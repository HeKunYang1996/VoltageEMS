// batch-query 请求体中的单个 series 定义
export interface BatchSeriesItem {
  redis_key: string
  point_id: string
}

// POST /hisApi/data/batch-query 请求体
export interface BatchQueryRequest {
  start_time: string
  end_time: string
  limit_per_series?: number
  series: BatchSeriesItem[]
}

// 单个数据点
export interface HistoryDataPoint {
  timestamp: string
  value: number | null
}

// batch-query 响应中的单条 series 数据
export interface BatchQueryResponse {
  redis_key: string
  point_id: string
  count: number
  data: HistoryDataPoint[]
}

// batch-query 接口顶层响应 data 字段结构
export interface BatchQueryData {
  start_time: string
  end_time: string
  series: BatchQueryResponse[]
}

// 旧的单点查询参数（保留兼容性）
export interface QueryPowerTrendParams {
  redis_key: string
  start_time?: string
  end_time?: string
  page?: number
  page_size?: number
  point_id: string
  interval?: number
}
