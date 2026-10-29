import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { getChannelBindings, getStationTopology } from '@/api/stationTopology'
import { getAllInstances } from '@/api/devicesManagement'
import type { NodeChannelBinding, StationTopology } from '@/types/stationTopology'
import type { DeviceInstanceListItem } from '@/types/deviceConfiguration'
import { buildTopologyNeighbours, resolveMeterLoadNodeIds } from '@/utils/topologyGraph'
import { collectFlowNodeInstanceIds, getFlowNodeInstanceId } from '@/utils/topologyNode'

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
  relatedProductName: string
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

/** One PCS or Hybrid_Inverter node linked to a Battery in the topology graph. */
export interface BatteryLinkedConverter {
  nodeId: string
  productName: string
  instanceIds: number[]
  channelIds: number[]
}

/** Operational Battery page context: one Battery node and its converter neighbours. */
export interface BatterySystemGroup {
  id: string
  primaryNodeId: string
  nodeIds: string[]
  primaryInstanceIds: number[]
  primaryChannelIds: number[]
  converters: BatteryLinkedConverter[]
  displayName: string
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

  /** Actual instances, including devices that are not connected to a Load node. */
  function getProductInstances(productName: string): DeviceInstanceListItem[] {
    return instances.value.filter((item) => item.product_name === productName)
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
    const edges = topology.value?.flow_json?.edges ?? []

    if (device === 'meterLoad') {
      return resolveMeterLoadNodeIds(nodes, edges)
    }

    const products = new Set(LOGICAL_DEVICE_PRODUCTS[device])
    return nodes
      .filter((node) => products.has(node.data.productName ?? node.data.label))
      .map((node) => node.id)
  }

  /** Resolve instance and channel IDs for one logical page device. */
  function getLogicalDeviceBinding(device: LogicalTopologyDevice): LogicalTopologyBinding {
    const nodeIds = getLogicalDeviceNodeIds(device)
    const nodeIdSet = new Set(nodeIds)
    const instanceIds = new Set<number>()
    const channelIds = new Set<number>()

    for (const id of collectFlowNodeInstanceIds(
      topology.value?.flow_json?.nodes ?? [],
      nodeIdSet,
    )) {
      instanceIds.add(id)
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

  function createTopologyContext() {
    const nodes = topology.value?.flow_json?.nodes ?? []
    const edges = topology.value?.flow_json?.edges ?? []
    const nodeById = new Map(nodes.map((node) => [node.id, node]))
    const neighbours = buildTopologyNeighbours(nodes, edges)

    const getNodeBinding = (nodeId: string) => {
      const instanceIds = new Set<number>()
      const channelIds = new Set<number>()
      const flowInstanceId = getFlowNodeInstanceId(nodeById.get(nodeId)?.data)
      if (flowInstanceId !== null) instanceIds.add(flowInstanceId)
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
      const ids: number[] = []
      const flowInstanceId = getFlowNodeInstanceId(node?.data)
      if (flowInstanceId !== null) ids.push(flowInstanceId)
      ids.push(
        ...nodeBindings.flatMap((item) =>
          item.instances.map((instance) => Number(instance.instanceId)),
        ),
      )
      const apiInstance = instances.value.find((item) => ids.includes(Number(item.id)))
      if (apiInstance?.name) return apiInstance.name
      const bindingInstance = nodeBindings[0]?.instances?.[0]?.instanceName
      if (bindingInstance) return bindingInstance
      return undefined
    }

    return { nodes, nodeById, neighbours, getNodeBinding, getNodeInstanceName }
  }

  /**
   * Records topology relationships that are useful to later business features.
   * This is derived from the saved graph; it is deliberately not persisted as
   * a second source of truth.
   */
  function getEquipmentGroups(): TopologyEquipmentGroup[] {
    const { nodes, nodeById, neighbours, getNodeBinding, getNodeInstanceName } = createTopologyContext()

    const groups: TopologyEquipmentGroup[] = []

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
          relatedProductName: nodeById.get(relatedNodeId)?.data.productName ?? '',
          ...binding,
          primaryInstanceIds: primaryBinding.instanceIds,
          relatedInstanceIds: relatedBinding.instanceIds,
          primaryChannelIds: primaryBinding.channelIds,
          relatedChannelIds: relatedBinding.channelIds,
          // PV monitoring is sourced from AC_Inverter. PV_Group is a topology
          // relation node and does not need its own instance binding.
          complete: relatedBinding.instanceIds.length > 0,
          displayName: formatGroupName([primaryNodeId], relatedNodeId, containerNodeId),
        })
      }
    }

    const thisBinding = (nodeIds: string[]): Pick<TopologyEquipmentGroup, 'instanceIds' | 'channelIds'> => {
      const ids = new Set(nodeIds)
      const instanceIds = new Set<number>()
      const channelIds = new Set<number>()
      for (const id of collectFlowNodeInstanceIds(nodes, ids)) {
        instanceIds.add(id)
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

    for (const primary of nodes) {
      if (primary.data.productName !== 'PV_Group') continue

      for (const neighbourId of neighbours.get(primary.id) ?? []) {
        const neighbour = nodeById.get(neighbourId)
        if (!neighbour) continue
        if (neighbour.data.productName === 'AC_Inverter') {
          addGroup('pv', 'direct', primary.id, neighbour.id)
        }
      }
    }
    const grouped: TopologyEquipmentGroup[] = []
    for (const group of groups) {
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

  function getBatterySystems(): BatterySystemGroup[] {
    const { nodes, nodeById, neighbours, getNodeBinding, getNodeInstanceName } = createTopologyContext()
    const systems = new Map<string, BatterySystemGroup>()

    const ensureSystem = (batteryNodeId: string) => {
      let system = systems.get(batteryNodeId)
      if (system) return system
      const primaryBinding = getNodeBinding(batteryNodeId)
      const label = nodeById.get(batteryNodeId)?.data.label
      system = {
        id: `battery:${batteryNodeId}`,
        primaryNodeId: batteryNodeId,
        nodeIds: [batteryNodeId],
        primaryInstanceIds: primaryBinding.instanceIds,
        primaryChannelIds: primaryBinding.channelIds,
        converters: [],
        displayName: getNodeInstanceName(batteryNodeId) ?? label ?? 'Battery',
      }
      systems.set(batteryNodeId, system)
      return system
    }

    for (const node of nodes) {
      if (node.data.productName !== 'Battery') continue
      const system = ensureSystem(node.id)
      for (const neighbourId of neighbours.get(node.id) ?? []) {
        const neighbour = nodeById.get(neighbourId)
        if (!neighbour) continue
        const productName = neighbour.data.productName ?? ''
        if (productName !== 'PCS' && productName !== 'Hybrid_Inverter') continue
        if (system.converters.some((item) => item.nodeId === neighbourId)) continue
        const relatedBinding = getNodeBinding(neighbourId)
        system.converters.push({
          nodeId: neighbourId,
          productName,
          instanceIds: relatedBinding.instanceIds,
          channelIds: relatedBinding.channelIds,
        })
        system.nodeIds = [...new Set([...system.nodeIds, neighbourId])]
      }
    }

    return [...systems.values()].filter((system) => system.primaryInstanceIds.length > 0)
  }

  function getInstanceDisplayName(instanceId: number): string {
    return instances.value.find((item) => Number(item.id) === instanceId)?.name ?? String(instanceId)
  }

  const equipmentGroups = computed(getEquipmentGroups)
  const pvGroups = computed(() => equipmentGroups.value.filter((group) => group.kind === 'pv' && group.complete))
  const batterySystems = computed(getBatterySystems)
  const selectedPvGroupId = ref<string | null>(null)
  const selectedBatterySystemId = ref<string | null>(null)
  const selectedDieselInstanceId = ref<number | null>(null)
  const selectedPvGroup = computed(() => {
    const groups = pvGroups.value
    if (!groups.some((group) => group.id === selectedPvGroupId.value)) selectedPvGroupId.value = groups[0]?.id ?? null
    return groups.find((group) => group.id === selectedPvGroupId.value) ?? null
  })
  const selectedBatterySystem = computed(() => {
    const systems = batterySystems.value
    if (!systems.some((system) => system.id === selectedBatterySystemId.value)) {
      selectedBatterySystemId.value = systems[0]?.id ?? null
    }
    return systems.find((system) => system.id === selectedBatterySystemId.value) ?? null
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
    getProductInstances,
    getChannelIds,
    getByNodeId,
    getAllInstanceIds,
    getAllChannelIds,
    getLogicalDeviceNodeIds,
    getLogicalDeviceBinding,
    getLogicalDeviceInstanceIds,
    getLogicalDeviceChannelIds,
    getEquipmentGroups,
    getBatterySystems,
    equipmentGroups,
    pvGroups,
    batterySystems,
    selectedPvGroupId,
    selectedBatterySystemId,
    selectedPvGroup,
    selectedBatterySystem,
    getInstanceDisplayName,
    selectedDieselInstanceId,
    selectedDieselId,
  }
})
