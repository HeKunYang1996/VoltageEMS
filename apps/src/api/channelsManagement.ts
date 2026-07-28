import { Request } from '@/utils/request'
import type { PointType } from '@/types/channelConfiguration'

/** Read channel points for device monitoring / alarm form options. */
export const getPointsTables = (id: number, type?: PointType, config?: any) => {
  return Request.get(`/comApi/api/channels/${id}/points`, type ? { type } : null, config)
}

/** Channel list for dropdowns (e.g. alarm rule form). */
export const getAllChannels = () => {
  return Request.get('/comApi/api/channels/list')
}
