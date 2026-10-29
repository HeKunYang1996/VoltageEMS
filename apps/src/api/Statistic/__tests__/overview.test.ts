import { beforeEach, describe, expect, it, vi } from 'vitest'
import { batchQueryHistory } from '../overview'
import Request from '@/utils/request'

vi.mock('@/utils/request', () => ({ default: { post: vi.fn() } }))
const params = {
  start_time: '2026-10-29T00:00:00Z', end_time: '2026-10-29T06:00:00Z',
  series: [{ redis_key: 'inst:1:M', point_id: '14' }],
}

describe('history timestamp contract', () => {
  beforeEach(() => vi.clearAllMocks())

  it('normalizes hissrv time and legacy timestamp, preserves null and zero', async () => {
    vi.mocked(Request.post).mockResolvedValue({ code: 200, success: true, message: 'OK', data: {
      series: [{ ...params.series[0], count: 3, data: [
        { time: '2026-10-29T00:00:00Z', value: 0 },
        { timestamp: '2026-10-29T08:01:00+08:00', value: null },
        { time: 'invalid', value: 99 },
      ] }],
    } })
    const signal = new AbortController().signal
    const result = await batchQueryHistory(params, signal)
    expect(Request.post).toHaveBeenCalledWith('/hisApi/data/batch-query', params, { signal })
    expect(result.data.series[0].data).toEqual([
      { timestamp: '2026-10-29T00:00:00.000Z', value: 0 },
      { timestamp: '2026-10-29T00:01:00.000Z', value: null },
    ])
  })
})
