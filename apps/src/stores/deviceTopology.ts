import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { getChannelBindings, getStationTopology } from '@/api/stationTopology'
import { getAllInstances } from '@/api/devicesManagement'
import type { NodeChannelBinding, StationTopology } from '@/types/stationTopology'
import type { DeviceInstanceListItem } from '@/types/deviceConfiguration'

export type LogicalTopologyDevice = 'pv' | 'diesel' | 'battery' | 'meterLoad'

export interface LogicalTopologyBinding {
  nodeIds: string[]
  instanceIds: number[]
  channelIds: number[]
}

export type TopologyEquipmentGroupKind = 'pv' | 'battery'

export interface TopologyEquipmentGroup {
  id: string
  kind: TopologyEquipmentGroupKind
  relation: 'direct' | 'hybrid-component'
  /** The external product node (PV_Group or Battery). */
  primaryNodeId: string
  primaryNodeIds: string[]
  /** The directly connected product node or Hybrid component node. */
  relatedNodeId: string
  /** Hybrid_Inverter container when relatedNodeId is one of its components. */
  containerNodeId?: string
  nodeIds: string[]
  instanceIds: number[]
  channelIds: number[]
  primaryInstanceIds: number[]
  relatedInstanceIds: number[]
  primaryChannelIds: number[]
  relatedChannelIds: number[]
  complete: boolean
  displayName: string
}

export interface TopologyEquipmentGroupOption extends TopologyEquipmentGroup {
  label: string
}

const LOGICAL_DEVICE_PRODUCTS: Record<LogicalTopologyDevice, string[]> = {
  pv: ['AC_Inverter'],
  diesel: ['Diesel'],
  battery: ['PCS', 'Battery'],
  meterLoad: ['Meter'],
}

/**
 * 站点设备拓扑 Store。
 *
 * 在用户登录后（App.vue initWebSocket）调用 load()，同时拉取：
 *   1. channel-bindings — 实例→通道映射（WS 订阅用）
 *   2. station/topology — 完整 flow_json（客户端显示拓扑图用）
 *
 * 使用示例：
 *   const topo = useDeviceTopologyStore()
 *   topo.getInstanceIds('Battery')   // → [1]（inst 源 WS channels）
 *   topo.getChannelIds('Battery')    // → [2]（comsrv 源 WS channels）
 *   topo.topology                    // → StationTopology | null（完整拓扑）
 */
export const useDeviceTopologyStore = defineStore('deviceTopology', () => {
  /** channel-bindings 解析结果，供 getInstanceIds / getChannelIds 查询 */
  const bindings = ref<NodeChannelBinding[]>([])
  const instances = ref<DeviceInstanceListItem[]>([])
  /** 完整站点拓扑（含 flow_json），null 表示尚未加载或站点未配置 */
  const topology = ref<StationTopology | null>(null)
  const loaded = ref(false)
  const loading = ref(false)

  /** 并行加载 channel-bindings 与完整拓扑 */
  async function load() {
    if (loading.value) return
    loading.value = true
    try {
      const [bindingsRes, topoRes, instancesRes] = await Promise.all([
        getChannelBindings(),
        getStationTopology(),
        getAllInstances(),
      ])
      if (bindingsRes?.success && bindingsRes.data) {
        bindings.value = bindingsRes.data.bindings ?? []
      }
      if (topoRes?.success && topoRes.data) {
        topology.value = topoRes.data
      }
      if (instancesRes?.success && instancesRes.data) {
        instances.value = instancesRes.data.list ?? []
      }
      loaded.value = true
    } catch (e) {
      console.error('[deviceTopology] load failed:', e)
    } finally {
      loading.value = false
    }
  }

  /** 强制重新加载（路由配置变更后调用） */
  async function reload() {
    loaded.value = false
    topology.value = null
    await load()
  }

  /**
   * 获取某产品类型的所有 instanceId（inst 源 WebSocket 用）。
   * productName 同 modsrv products 表，如 'Battery'、'Diesel'、'PCS'、'PV DCDC'。
   */
  function getInstanceIds(productName: string): number[] {
    return bindings.value
      .filter((b) => b.productName === productName)
      .flatMap((b) => b.instances.map((i) => i.instanceId))
  }

  /**
   * 获取某产品类型的所有 channelId（comsrv 源 WebSocket 和 getPointsTables 用）。
   * 自动去重，因为多个实例可能复用同一通道。
   */
  function getChannelIds(productName: string): number[] {
    const ids = bindings.value
      .filter((b) => b.productName === productName)
      .flatMap((b) => b.instances.flatMap((i) => i.channelIds))
    return [...new Set(ids)]
  }

  /** 按 nodeId 精确查询（Visual Modeling 画布节点用） */
  function getByNodeId(nodeId: string): NodeChannelBinding | undefined {
    return bindings.value.find((b) => b.nodeId === nodeId)
  }

  /** 全站所有 instanceId（统计/概览汇总用） */
  function getAllInstanceIds(): number[] {
    return [...new Set(bindings.value.flatMap((b) => b.instances.map((i) => i.instanceId)))]
  }

  /** 全站所有 channelId */
  function getAllChannelIds(): number[] {
    return [
      ...new Set(bindings.value.flatMap((b) => b.instances.flatMap((i) => i.channelIds))),
    ]
  }

  /**
   * Resolve the product nodes used by the operational pages.
   * The topology may contain display-only products (for example PV_Group),
   * so pages should use this semantic mapping instead of querying products ad hoc.
   */
  function getLogicalDeviceNodeIds(device: LogicalTopologyDevice): string[] {
    const nodes = topology.value?.flow_json?.nodes ?? []
    const products = new Set(LOGICAL_DEVICE_PRODUCTS[device])
    const distributionBoardIds = new Set(
      nodes
        .filter((node) => node.data.productName === 'Distribution_Board')
        .map((node) => node.id),
    )

    return nodes
      .filter((node) => {
        if (!products.has(node.data.productName ?? node.data.label)) return false
        if (device !== 'meterLoad') return true
        return Boolean(node.parentNode && distributionBoardIds.has(node.parentNode))
      })
      .map((node) => node.id)
  }

  /** Resolve instance and channel IDs for one logical page device. */
  function getLogicalDeviceBinding(device: LogicalTopologyDevice): LogicalTopologyBinding {
    const nodeIds = getLogicalDeviceNodeIds(device)
    const nodeIdSet = new Set(nodeIds)
    const instanceIds = new Set<number>()
    const channelIds = new Set<number>()

    for (const node of topology.value?.flow_json?.nodes ?? []) {
      if (!nodeIdSet.has(node.id)) continue
      for (const instance of node.data.instances ?? []) {
        const id = Number(instance.instanceId)
        if (Number.isFinite(id) && id > 0) instanceIds.add(id)
      }
    }

    for (const binding of bindings.value) {
      if (!nodeIdSet.has(binding.nodeId)) continue
      for (const instance of binding.instances ?? []) {
        const instanceId = Number(instance.instanceId)
        if (Number.isFinite(instanceId) && instanceId > 0) instanceIds.add(instanceId)
        for (const channelId of instance.channelIds ?? []) {
          const id = Number(channelId)
          if (Number.isFinite(id) && id > 0) channelIds.add(id)
        }
      }
    }

    return {
      nodeIds,
      instanceIds: [...instanceIds],
      channelIds: [...channelIds],
    }
  }

  function getLogicalDeviceInstanceIds(device: LogicalTopologyDevice): number[] {
    return getLogicalDeviceBinding(device).instanceIds
  }

  function getLogicalDeviceChannelIds(device: LogicalTopologyDevice): number[] {
    return getLogicalDeviceBinding(device).channelIds
  }

  /**
   * Records topology relationships that are useful to later business features.
   * This is derived from the saved graph; it is deliberately not persisted as
   * a second source of truth.
   */
  function getEquipmentGroups(): TopologyEquipmentGroup[] {
    const nodes = topology.value?.flow_json?.nodes ?? []
    const edges = topology.value?.flow_json?.edges ?? []
    const nodeById = new Map(nodes.map((node) => [node.id, node]))
    const neighbours = new Map<string, Set<string>>()
    for (const edge of edges) {
      if (!nodeById.has(edge.source) || !nodeById.has(edge.target)) continue
      if (!neighbours.has(edge.source)) neighbours.set(edge.source, new Set())
      if (!neighbours.has(edge.target)) neighbours.set(edge.target, new Set())
      neighbours.get(edge.source)!.add(edge.target)
      neighbours.get(edge.target)!.add(edge.source)
    }

    const groups: TopologyEquipmentGroup[] = []
    const getNodeBinding = (nodeId: string) => {
      const instanceIds = new Set<number>()
      const channelIds = new Set<number>()
      const node = nodeById.get(nodeId)
      for (const instance of node?.data.instances ?? []) {
        const id = Number(instance.instanceId)
        if (Number.isFinite(id) && id > 0) instanceIds.add(id)
      }
      for (const binding of bindings.value.filter((item) => item.nodeId === nodeId)) {
        for (const instance of binding.instances ?? []) {
          const instanceId = Number(instance.instanceId)
          if (Number.isFinite(instanceId) && instanceId > 0) instanceIds.add(instanceId)
          for (const channelId of instance.channelIds ?? []) {
            const id = Number(channelId)
            if (Number.isFinite(id) && id > 0) channelIds.add(id)
          }
        }
      }
      return { instanceIds: [...instanceIds], channelIds: [...channelIds] }
    }

    const getNodeInstanceName = (nodeId: string) => {
      const node = nodeById.get(nodeId)
      const nodeBindings = bindings.value.filter((item) => item.nodeId === nodeId)
      const ids = [
        ...(node?.data.instances?.map((item) => Number(item.instanceId)) ?? []),
        ...nodeBindings.flatMap((item) => item.instances.map((instance) => Number(instance.instanceId))),
      ]
      const apiInstance = instances.value.find((item) => ids.includes(Number(item.id)))
      if (apiInstance?.name) return apiInstance.name
      const nodeInstance = node?.data.instances?.[0]?.instanceName
      if (nodeInstance) return nodeInstance
      const bindingInstance = nodeBindings[0]?.instances?.[0]?.instanceName
      if (bindingInstance) return bindingInstance
      return undefined
    }

    const formatGroupName = (primaryNodeIds: string[], relatedNodeId: string, containerNodeId?: string) => {
      const names = [
        ...primaryNodeIds.map(getNodeInstanceName),
        getNodeInstanceName(relatedNodeId),
        ...(containerNodeId ? [getNodeInstanceName(containerNodeId)] : []),
      ].filter((name): name is string => Boolean(name))
      return names.join(' / ')
    }

    const addGroup = (
      kind: TopologyEquipmentGroupKind,
      relation: TopologyEquipmentGroup['relation'],
      primaryNodeId: string,
      relatedNodeId: string,
      containerNodeId?: string,
    ) => {
      const nodeIds = [...new Set([primaryNodeId, relatedNodeId, ...(containerNodeId ? [containerNodeId] : [])])]
      const primaryBinding = getNodeBinding(primaryNodeId)
      const relatedBinding = getNodeBinding(relatedNodeId)
      const binding = thisBinding(nodeIds)
      const id = `${kind}:${relation}:${primaryNodeId}:${relatedNodeId}`
      if (!groups.some((group) => group.id === id)) {
        groups.push({
          id, kind, relation, primaryNodeId, primaryNodeIds: [primaryNodeId], relatedNodeId, containerNodeId, nodeIds,
          ...binding,
          primaryInstanceIds: primaryBinding.instanceIds,
          relatedInstanceIds: relatedBinding.instanceIds,
          primaryChannelIds: primaryBinding.channelIds,
          relatedChannelIds: relatedBinding.channelIds,
          // PV monitoring is sourced from AC_Inverter. PV_Group is a topology
          // relation node and does not need its own instance binding.
          complete: kind === 'pv'
            ? relatedBinding.instanceIds.length > 0
            : primaryBinding.instanceIds.length > 0 && relatedBinding.instanceIds.length > 0,
          displayName: formatGroupName([primaryNodeId], relatedNodeId, containerNodeId),
        })
      }
    }

    const thisBinding = (nodeIds: string[]): Pick<TopologyEquipmentGroup, 'instanceIds' | 'channelIds'> => {
      const ids = new Set(nodeIds)
      const instanceIds = new Set<number>()
      const channelIds = new Set<number>()
      for (const node of nodes) {
        if (!ids.has(node.id)) continue
        for (const instance of node.data.instances ?? []) {
          const instanceId = Number(instance.instanceId)
          if (Number.isFinite(instanceId) && instanceId > 0) instanceIds.add(instanceId)
        }
      }
      for (const binding of bindings.value) {
        if (!ids.has(binding.nodeId)) continue
        for (const instance of binding.instances ?? []) {
          const instanceId = Number(instance.instanceId)
          if (Number.isFinite(instanceId) && instanceId > 0) instanceIds.add(instanceId)
          for (const channelId of instance.channelIds ?? []) {
            const id = Number(channelId)
            if (Number.isFinite(id) && id > 0) channelIds.add(id)
          }
        }
      }
      return { instanceIds: [...instanceIds], channelIds: [...channelIds] }
    }

    const componentByContainer = (containerId: string, productName: string) =>
      nodes.find((node) => node.parentNode === containerId && node.data.productName === productName)

    for (const primary of nodes) {
      const primaryProduct = primary.data.productName
      const kind: TopologyEquipmentGroupKind | undefined =
        primaryProduct === 'PV_Group' ? 'pv' : primaryProduct === 'Battery' ? 'battery' : undefined
      if (!kind) continue

      for (const neighbourId of neighbours.get(primary.id) ?? []) {
        const neighbour = nodeById.get(neighbourId)
        if (!neighbour) continue
        const neighbourProduct = neighbour.data.productName
        const directTarget = kind === 'pv' ? 'AC_Inverter' : 'PCS'
        if (neighbourProduct === directTarget) {
          addGroup(kind, 'direct', primary.id, neighbour.id)
          continue
        }
        if (neighbourProduct !== 'Hybrid_Inverter') continue
        const component = componentByContainer(neighbour.id, directTarget)
        if (component) addGroup(kind, 'hybrid-component', primary.id, component.id, neighbour.id)
      }
    }
    const grouped: TopologyEquipmentGroup[] = []
    for (const group of groups) {
      if (group.kind !== 'pv') {
        grouped.push(group)
        continue
      }
      const key = `${group.relation}:${group.relatedNodeId}:${group.containerNodeId ?? ''}`
      const existing = grouped.find((item) => item.kind === 'pv' && `${item.relation}:${item.relatedNodeId}:${item.containerNodeId ?? ''}` === key)
      if (!existing) {
        grouped.push(group)
        continue
      }
      existing.primaryNodeIds.push(...group.primaryNodeIds)
      existing.primaryNodeId = existing.primaryNodeIds[0]
      existing.nodeIds = [...new Set([...existing.nodeIds, ...group.nodeIds])]
      existing.primaryInstanceIds = [...new Set([...existing.primaryInstanceIds, ...group.primaryInstanceIds])]
      existing.primaryChannelIds = [...new Set([...existing.primaryChannelIds, ...group.primaryChannelIds])]
      existing.instanceIds = [...new Set([...existing.instanceIds, ...group.instanceIds])]
      existing.channelIds = [...new Set([...existing.channelIds, ...group.channelIds])]
      existing.complete = existing.relatedInstanceIds.length > 0
      existing.displayName = formatGroupName(existing.primaryNodeIds, existing.relatedNodeId, existing.containerNodeId)
    }
    return grouped
  }

  const equipmentGroups = computed(getEquipmentGroups)
  const pvGroups = computed(() => equipmentGroups.value.filter((group) => group.kind === 'pv' && group.complete))
  const batteryGroups = computed(() => equipmentGroups.value.filter((group) => group.kind === 'battery' && group.complete))
  const selectedPvGroupId = ref<string | null>(null)
  const selectedBatteryGroupId = ref<string | null>(null)
  const selectedDieselInstanceId = ref<number | null>(null)
  const selectedPvGroup = computed(() => {
    const groups = pvGroups.value
    if (!groups.some((group) => group.id === selectedPvGroupId.value)) selectedPvGroupId.value = groups[0]?.id ?? null
    return groups.find((group) => group.id === selectedPvGroupId.value) ?? null
  })
  const selectedBatteryGroup = computed(() => {
    const groups = batteryGroups.value
    if (!groups.some((group) => group.id === selectedBatteryGroupId.value)) selectedBatteryGroupId.value = groups[0]?.id ?? null
    return groups.find((group) => group.id === selectedBatteryGroupId.value) ?? null
  })
  const selectedDieselId = computed(() => {
    const ids = getLogicalDeviceInstanceIds('diesel')
    if (!ids.includes(selectedDieselInstanceId.value ?? -1)) selectedDieselInstanceId.value = ids[0] ?? null
    return selectedDieselInstanceId.value
  })

  return {
    bindings,
    instances,
    topology,
    loaded,
    loading,
    load,
    reload,
    getInstanceIds,
    getChannelIds,
    getByNodeId,
    getAllInstanceIds,
    getAllChannelIds,
    getLogicalDeviceNodeIds,
    getLogicalDeviceBinding,
    getLogicalDeviceInstanceIds,
    getLogicalDeviceChannelIds,
    getEquipmentGroups,
    equipmentGroups,
    pvGroups,
    batteryGroups,
    selectedPvGroupId,
    selectedBatteryGroupId,
    selectedPvGroup,
    selectedBatteryGroup,
    selectedDieselInstanceId,
    selectedDieselId,
  }
})
