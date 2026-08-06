import { beforeEach, describe, expect, it, vi } from 'vitest'
import { getAllChannels, getPointsTables } from '../channelsManagement'

vi.mock('@/utils/request', () => ({
  Request: {
    get: vi.fn(),
  },
}))

describe('api/channelsManagement.ts', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('reads channel points and channel list', async () => {
    const { Request } = await import('@/utils/request')
    vi.mocked(Request.get)
      .mockResolvedValueOnce({ success: true, data: [] })
      .mockResolvedValueOnce({ success: true, data: { list: [] } })

    await getPointsTables(3, 'T')
    await getAllChannels()

    expect(Request.get).toHaveBeenNthCalledWith(1, '/comApi/api/channels/3/points', { type: 'T' }, undefined)
    expect(Request.get).toHaveBeenNthCalledWith(2, '/comApi/api/channels/list')
  })
})
