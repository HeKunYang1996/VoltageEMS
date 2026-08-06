import Request from '@/utils/request'
import type { PaginatedList, RuleHistoryRecord } from '@/types/controlRule'

export interface RuleHistoryListQuery {
  page?: number
  page_size?: number
  /** Exact rule id filter (prefer alone; can combine with rule_name) */
  rule_id?: number | string
  /** Case-insensitive substring match on rule name */
  rule_name?: string
  start_time?: number
  end_time?: number
}

/** GET /api/rules/history — page-end only needs history listing (query filters). */
export const listRuleHistoryRecords = async (params: RuleHistoryListQuery = {}) => {
  return await Request.get<PaginatedList<RuleHistoryRecord>>('/ruleApi/api/rules/history', params)
}
