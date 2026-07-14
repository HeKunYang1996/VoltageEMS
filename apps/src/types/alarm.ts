// 鍛婅璁板綍绫诲瀷瀹氫箟

// 瑙勫垯蹇収绫诲瀷
export interface RuleSnapshot {
  rule_name: string
  warning_level: number
  operator: string
  value: number | string
  description: string
}

// 褰撳墠鍛婅鏁版嵁绫诲瀷
export interface CurrentAlarmData {
  id: number
  rule_id: number
  rule_snapshot: string // API 杩斿洖 JSON 瀛楃涓?  service_type: string
  channel_id: number
  data_type: string
  point_id: number
  rule_name: string
  warning_level: number
  operator: string
  threshold_value: number | string
  current_value: number | string
  status: 'active' | 'inactive'
  triggered_at: number
}

// 鍘嗗彶鍛婅鏁版嵁绫诲瀷
export interface HistoryAlarmData {
  id: number
  rule_id: number
  rule_snapshot: string // API 杩斿洖 JSON 瀛楃涓?  service_type: string
  channel_id: number
  data_type: string
  point_id: number
  rule_name: string
  warning_level: number
  operator: string
  threshold_value: number | string
  trigger_value?: number | string
  recovery_value?: number | string | null
  event_type: 'trigger' | 'recovery'
  triggered_at: number
  recovered_at?: number
  duration?: number
}
export interface CurrentAlarmResponse {
  list: CurrentAlarmData[]
  total: number
}
export interface HistoryAlarmResponse {
  list: HistoryAlarmData[]
  total: number
}
// 鍛婅绾у埆鏋氫妇
export enum AlarmLevel {
  LEVEL_1 = 1,
  LEVEL_2 = 2,
  LEVEL_3 = 3,
}

// 鎿嶄綔绗︽灇涓?
export enum AlarmOperator {
  GREATER_THAN = '>',
  LESS_THAN = '<',
  EQUAL = '==',
  NOT_EQUAL = '!=',
  GREATER_EQUAL = '>=',
  LESS_EQUAL = '<=',
}

// 浜嬩欢绫诲瀷鏋氫妇
export enum AlarmEventType {
  TRIGGER = 'trigger',
  RECOVERY = 'recovery',
}

// 鍛婅鐘舵€佹灇涓?
export enum AlarmStatus {
  ACTIVE = 'active',
  INACTIVE = 'inactive',
}

// 鏈嶅姟绫诲瀷鏋氫妇
export enum ServiceType {
  RULESRV = 'rulesrv',
}

// 鏁版嵁绫诲瀷鏋氫妇
export enum DataType {
  TEMPERATURE = 'T',
  STATUS = 'S',
  PRESSURE = 'P',
  VOLTAGE = 'V',
}

// 鍛婅鏌ヨ鍙傛暟
export interface AlarmQueryParams {
  type: 'current' | 'history'
  warning_level?: AlarmLevel
  service_type?: ServiceType
  channel_id?: number
  data_type?: DataType
  point_id?: number
  status?: AlarmStatus
  event_type?: AlarmEventType
  start_time?: number
  end_time?: number
  page?: number
  page_size?: number
}
