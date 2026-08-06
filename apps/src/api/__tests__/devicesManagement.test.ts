import { beforeEach, describe, expect, it, vi } from 'vitest'
import { getInstancePoints } from '../devicesManagement'

vi.mock('@/utils/request', () => ({
  Request: {
    get: vi.fn(),
  },
}))

describe('api/devicesManagement.ts', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('reads instance points', async () => {
    const { Request } = await import('@/utils/request')
    vi.mocked(Request.get).mockResolvedValue({ success: true, data: { list: [] } })

    await getInstancePoints(3)

    expect(Request.get).toHaveBeenCalledWith('/modApi/api/instances/3/points')
  })
})
