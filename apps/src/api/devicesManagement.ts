import { Request } from '@/utils/request'
import type { ApiResponse } from '@/types/user'
import type { InstancePointList } from '@/types/deviceConfiguration'

/** Read instance points for statistics curves. */
export const getInstancePoints = (instanceId: number): Promise<ApiResponse<InstancePointList>> => {
  return Request.get(`/modApi/api/instances/${instanceId}/points`)
}
