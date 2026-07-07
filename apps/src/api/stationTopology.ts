import { Request } from '@/utils/request'
import type { ChannelBindingsData, StationTopology } from '@/types/stationTopology'

/**
 * 获取拓扑节点的实时通道绑定。
 * 后端从 measurement_routing 表实时计算，无需重新保存拓扑。
 * 路径: GET /modApi/api/station/topology/channel-bindings
 */
export function getChannelBindings() {
  return Request.get<ChannelBindingsData>('/modApi/api/station/topology/channel-bindings')
}

/**
 * 获取完整站点拓扑（含 flow_json 节点与连线）。
 * 首次未配置时返回 flow_json: { nodes: [], edges: [] }，不返回 404。
 * 路径: GET /modApi/api/station/topology
 */
export function getStationTopology() {
  return Request.get<StationTopology>('/modApi/api/station/topology')
}
