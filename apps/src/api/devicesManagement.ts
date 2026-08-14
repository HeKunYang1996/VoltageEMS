import { Request } from '@/utils/request'
import type { ApiResponse } from '@/types/user'
import type { DeviceInstanceListResponse, InstancePointList } from '@/types/deviceConfiguration'

export const getAllInstances = () =>
  Request.get<DeviceInstanceListResponse>('/modApi/api/instances/list')

/** Read instance points for statistics curves. */
export const getInstancePoints = (instanceId: number): Promise<ApiResponse<InstancePointList>> => {
  return Request.get(`/modApi/api/instances/${instanceId}/points`)
}
