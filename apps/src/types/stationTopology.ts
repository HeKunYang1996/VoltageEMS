/** 单个实例的通道绑定（来自 channel-bindings 接口） */
export interface InstanceChannelBinding {
  instanceId: number
  instanceName: string
  /** 实时从 measurement_routing 表计算，路由变更自动反映 */
  channelIds: number[]
}

/** 单个拓扑节点的绑定信息 */
export interface NodeChannelBinding {
  nodeId: string
  productName: string
  instances: InstanceChannelBinding[]
}

/** GET /modApi/api/station/topology/channel-bindings 的 data 字段 */
export interface ChannelBindingsData {
  bindings: NodeChannelBinding[]
}

// ──────────────────────── 完整拓扑（GET /modApi/api/station/topology）────────────────────────

/** 拓扑节点的业务数据（data 字段，与 PCManagement flow_json 对齐） */
export interface TopologyNodeData {
  label: string
  displayName?: string
  display_name?: string
  productName?: string
  /** 当前节点绑定的唯一设备实例；null 表示尚未绑定。 */
  instanceId?: number | null
  isContainer?: boolean
  description?: string
}

/** flow_json 中的单个节点 */
export interface TopologyFlowNode {
  id: string
  type: 'station' | 'product' | 'group' | string
  position: { x: number; y: number }
  data: TopologyNodeData
  parentNode?: string
  /** Vue Flow 扩展字段，可能存在 */
  style?: Record<string, string | number>
}

/** flow_json 中的单条连线 */
export interface TopologyFlowEdge {
  id: string
  source: string
  target: string
  label?: string
  type?: string
}

/** flow_json 完整结构 */
export interface TopologyFlowData {
  nodes: TopologyFlowNode[]
  edges: TopologyFlowEdge[]
  /** 固定在画布左上角的绑定，不属于拓扑节点或连线。 */
  fixedBindings?: {
    stationInstanceId: number | null
    environmentInstanceId: number | null
  }
}

/**
 * 站点拓扑完整记录（与当前接口的 camelCase 响应一致）。
 */
export interface StationTopology {
  station_id: string
  station_name: string
  description: string | null
  created_at: string | null
  updated_at: string | null
  flow_json: TopologyFlowData
}
