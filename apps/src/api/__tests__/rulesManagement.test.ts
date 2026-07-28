import { beforeEach, describe, expect, it, vi } from 'vitest'
import { listRuleHistoryRecords } from '../rulesManagement'

vi.mock('@/utils/request', () => {
  const Request = {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
  }
  return {
    default: Request,
    Request,
  }
})

describe('api/rulesManagement.ts', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('lists rule history via query params', async () => {
    const RequestModule = await import('@/utils/request')
    vi.mocked(RequestModule.default.get).mockResolvedValueOnce({
      success: true,
      data: { list: [] },
    })

    await listRuleHistoryRecords({
      rule_id: 12,
      rule_name: 'Battery',
      start_time: 1,
      end_time: 2,
      page: 1,
      page_size: 20,
    })

    expect(RequestModule.default.get).toHaveBeenCalledWith('/ruleApi/api/rules/history', {
      rule_id: 12,
      rule_name: 'Battery',
      start_time: 1,
      end_time: 2,
      page: 1,
      page_size: 20,
    })
  })
})
