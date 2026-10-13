import type { TopologyFlowEdge, TopologyNodeData } from '@/types/stationTopology'

/** Load 类产品 name，与 voltage-product-lib 保持一致。 */
export const TOPOLOGY_LOAD_PRODUCTS = [
  'Single_Phase_Load',
  'Three_Phase_Load',
  'EV_Charging_Load',
  'HVAC_Load',
] as const

export type TopologyLoadProductName = (typeof TOPOLOGY_LOAD_PRODUCTS)[number]

const loadProductSet = new Set<string>(TOPOLOGY_LOAD_PRODUCTS)

export function isTopologyLoadProduct(productName: string | undefined): productName is TopologyLoadProductName {
  return Boolean(productName && loadProductSet.has(productName))
}

type TopologyGraphNode = { id: string; data: TopologyNodeData }

/** 从无向边构建邻接表（与 getEquipmentGroups 相同语义）。 */
export function buildTopologyNeighbours(
  nodes: TopologyGraphNode[],
  edges: Pick<TopologyFlowEdge, 'source' | 'target'>[],
): Map<string, Set<string>> {
  const nodeById = new Map(nodes.map((node) => [node.id, node]))
  const neighbours = new Map<string, Set<string>>()

  for (const edge of edges) {
    if (!nodeById.has(edge.source) || !nodeById.has(edge.target)) continue
    if (!neighbours.has(edge.source)) neighbours.set(edge.source, new Set())
    if (!neighbours.has(edge.target)) neighbours.set(edge.target, new Set())
    neighbours.get(edge.source)!.add(edge.target)
    neighbours.get(edge.target)!.add(edge.source)
  }

  return neighbours
}

/**
 * 负载计量 Meter：与任一 Load 类产品在无向图上相邻。
 * 对齐产品库——Load 必须连 Meter，关系由边表达，不依赖 parentNode / Distribution_Board。
 */
export function resolveMeterLoadNodeIds(
  nodes: TopologyGraphNode[],
  edges: Pick<TopologyFlowEdge, 'source' | 'target'>[],
): string[] {
  const nodeById = new Map(nodes.map((node) => [node.id, node]))
  const neighbours = buildTopologyNeighbours(nodes, edges)

  return nodes
    .filter((node) => node.data.productName === 'Meter')
    .filter((meter) => {
      for (const neighbourId of neighbours.get(meter.id) ?? []) {
        const neighbour = nodeById.get(neighbourId)
        if (isTopologyLoadProduct(neighbour?.data.productName)) return true
      }
      return false
    })
    .map((node) => node.id)
}
