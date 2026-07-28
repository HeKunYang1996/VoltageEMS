/** Alarm rule operators (UI + API). */
export type Operator = '>' | '>=' | '<' | '<=' | '=' | 'gt' | 'gte' | 'lt' | 'lte' | 'eq'

/** Instance point kinds used by alarmsrv Redis keys `inst:{id}:M|A`. */
export type AlarmDataType = 'M' | 'A'

export interface RuleFormModel {
  rule_name: string
  service_type: string
  /** Instance id when service_type is `inst` (field name kept for API compat). */
  channel_id: number | undefined
  point_id: number | null
  data_type: AlarmDataType | null
  warning_level: number | null
  operator: Operator | null
  value: number | null
  description?: string
  enabled: boolean
}

export interface RuleInfo {
  id: number
  channel_id?: number
  rule_name: string
  service_type: string
  point_id: number | null
  data_type: AlarmDataType | string | null
  warning_level: number | null
  operator: Operator | null
  value: number | null
  notification?: string[]
  enabled: boolean
  description?: string
  created_at: number
  updated_at?: number
}

export interface DialogExpose {
  dialogVisible: boolean
}

export interface RuleResponse {
  list: RuleInfo[]
  total: number
}

export interface RuleDetailResponse {
  list: RuleInfo[]
  total: number
}

/** PUT /alarmApi/rules/{id} — all fields optional. */
export interface UpdateAlarmRulePayload {
  service_type?: string
  channel_id?: number
  data_type?: string
  point_id?: number
  rule_name?: string
  warning_level?: number
  operator?: Operator
  value?: number
  enabled?: boolean
  description?: string
}
