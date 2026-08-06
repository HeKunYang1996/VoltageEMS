import Request, { type ApiResponse } from '@/utils/request'
import type { BatchQueryRequest, BatchQueryData } from '@/types/Statistics/OverView'

// 批量查询历史数据（batch-query POST接口）
// signal 可选，用于外部通过 AbortController 取消请求
export const batchQueryHistory = (
  data: BatchQueryRequest,
  signal?: AbortSignal,
): Promise<ApiResponse<BatchQueryData>> => {
  return Request.post('/hisApi/data/batch-query', data, signal ? { signal } : undefined)
}
