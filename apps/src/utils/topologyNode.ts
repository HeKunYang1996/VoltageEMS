import type { TopologyNodeData } from '@/types/stationTopology'

/** 从 flow_json 节点 data 解析唯一 instanceId；无效或未绑定时返回 null。 */
export function getFlowNodeInstanceId(data: TopologyNodeData | undefined): number | null {
  if (data?.instanceId == null) return null
  const id = Number(data.instanceId)
  return Number.isFinite(id) && id > 0 ? id : null
}

/** 收集若干节点在 flow_json 中声明的 instanceId（不含 channel-bindings）。 */
export function collectFlowNodeInstanceIds(
  nodes: Array<{ id: string; data: TopologyNodeData }>,
  nodeIds: Set<string>,
): number[] {
  const ids = new Set<number>()
  for (const node of nodes) {
    if (!nodeIds.has(node.id)) continue
    const id = getFlowNodeInstanceId(node.data)
    if (id !== null) ids.add(id)
  }
  return [...ids]
}
