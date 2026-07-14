export interface PaginatedList<T> {
  list: T[]
  page: number
  page_size: number
  total: number
  total_pages: number
  has_next: boolean
  has_previous: boolean
}

/** GET /ruleApi/api/rules 列表项 */
export interface ModRuleSummary {
  id: string | number
  name: string
  enabled: boolean
  description?: string | null
}

/** 触发历史可读化展示（后端执行时快照） */
export interface RuleHistoryDisplayVariable {
  key: string
  label: string
  value: number
  unit?: string
  instance_name?: string
  point_name?: string
}

export interface RuleHistoryDisplayStep {
  node_id: string
  label: string
  type?: string
  matched_label?: string
}

export interface RuleHistoryDisplayAction {
  description: string
  success?: boolean
}

export interface RuleHistoryDisplay {
  summary?: string
  trigger_reason?: string
  variables?: RuleHistoryDisplayVariable[]
  execution_steps?: RuleHistoryDisplayStep[]
  actions?: RuleHistoryDisplayAction[]
}

export interface RuleExecutionResult {
  success?: boolean
  display?: RuleHistoryDisplay
}

/** GET /ruleApi/api/rules/{id}/history 单条记录 */
export interface RuleHistoryItem {
  id: number
  rule_id: number
  triggered_at: string
  error: string | null
  result: RuleExecutionResult | null
}

/** 控制模块历史记录列表项 */
export interface RuleHistoryRecord extends RuleHistoryItem {
  rule_name?: string
}
