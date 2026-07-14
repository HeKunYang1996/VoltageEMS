// 鎺у埗绠＄悊鐩稿叧绫诲瀷瀹氫箟

// 鎿嶄綔绗︾被鍨?
export type Operator = '>' | '>=' | '<' | '<=' | '=' | 'gt' | 'gte' | 'lt' | 'lte' | 'eq'

// Trigger config 鈥?mirrors backend TriggerConfig enum (serde snake_case tag)
export interface TriggerConfigInterval {
  type: 'interval'
  interval_ms: number
}

export interface PointRef {
  instance: number
  point_type: 'measurement' | 'action'
  point: number
}

export interface TriggerConfigOnChange {
  type: 'on_change'
  point_refs: PointRef[]
  time_deadband_ms: number | null
  value_deadband: null
}

export type TriggerConfig = TriggerConfigInterval | TriggerConfigOnChange

// 瑙勫垯琛ㄥ崟妯″瀷绫诲瀷
export interface RuleFormModel {
  rule_name: string
  service_type: string
  channel_id: number | undefined
  point_id: number | null
  data_type: 'T' | 'S' | null
  warning_level: number | null
  operator: Operator | null
  value: number | null
  description?: string
  enabled: boolean
  trigger_config?: TriggerConfig
}

// 瑙勫垯淇℃伅绫诲瀷
export interface RuleInfo {
  id: number
  channel_id?: number
  rule_name: string
  service_type: string
  point_id: number | null
  data_type: 'T' | 'S' | null
  warning_level: number | null
  operator: Operator | null
  value: number | null
  notification?: string[]
  enabled: boolean
  description?: string
  created_at: number // Unix 鏃堕棿鎴筹紙绉掞級
  updated_at?: number // Unix 鏃堕棿鎴筹紙绉掞級
  trigger_config?: string | null // stored as JSON string in DB
}

// 瀵硅瘽妗嗘毚闇茬被鍨?
export interface DialogExpose {
  dialogVisible: boolean
}

// GET /alarmApi/rules 鍒楄〃鍝嶅簲
export interface RuleResponse {
  list: RuleInfo[]
  total: number
}

// GET /alarmApi/rules/{id} 鍗曟潯瑙勫垯鍝嶅簲
export interface RuleDetailResponse {
  list: RuleInfo[]
  total: number
}

/** PUT /alarmApi/rules/{id} 鈥?鎵€鏈夊瓧娈靛彲閫夛紝浠呮彁浜ら渶瑕佹洿鏂扮殑瀛楁 */
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
