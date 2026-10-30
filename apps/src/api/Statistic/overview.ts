import Request, { type ApiResponse } from '@/utils/request'
import type { BatchQueryRequest, BatchQueryData } from '@/types/Statistics/OverView'

// 批量查询历史数据（batch-query POST接口）
// signal 可选，用于外部通过 AbortController 取消请求
export const batchQueryHistory = async (
  data: BatchQueryRequest,
  signal?: AbortSignal,
): Promise<ApiResponse<BatchQueryData>> => {
  const response = await Request.post<BatchQueryData>(
    // Each history view owns its loading and cancellation scope. The global
    // duplicate-request handler must not cancel another view's history query.
    '/hisApi/data/batch-query', data, { signal, skipGlobalLoading: true },
  )
  // hissrv returns `time`; keep the existing frontend `timestamp` contract.
  if (response.data?.series) {
    response.data.series = response.data.series.map((series) => ({
      ...series,
      data: (series.data ?? []).flatMap((point) => {
        const timestamp = point.timestamp ?? (point as typeof point & { time?: string }).time
        const time = timestamp ? Date.parse(timestamp) : NaN
        return Number.isFinite(time)
          ? [{ timestamp: new Date(time).toISOString(), value: point.value }] : []
      }),
    }))
  }
  return response
}
