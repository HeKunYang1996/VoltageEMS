import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { getChannelBindings, getStationTopology } from '@/api/stationTopology'
import { getAllInstances } from '@/api/devicesManagement'
import type { NodeChannelBinding, StationTopology } from '@/types/stationTopology'
import type { DeviceInstanceListItem } from '@/types/deviceConfiguration'
import { buildTopologyNeighbours, resolveMeterLoadNodeIds } from '@/utils/topologyGraph'
import { getFlowNodeInstanceId } from '@/utils/topologyNode'

export type LogicalTopologyDevice = 'pv' | 'diesel' | 'battery' | 'meterLoad'

export interface LogicalTopologyBinding {
  nodeIds: string[]
  instanceIds: number[]
  channelIds: number[]
}

export type TopologyEquipmentGroupKind = 'pv' | 'battery'

export interface PVLinkedGroup {
  nodeId: string
  instanceIds: number[]
  channelIds: number[]
  displayName: string
}

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
  pvNodes: PVLinkedGroup[]
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
  pv: ['AC_Inverter', 'Hybrid_Inverter'],
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

  /** Resolve only bindings belonging to a node currently present on the canvas. */
  function resolveNodeBinding(nodeId: string) {
    const node = topology.value?.flow_json?.nodes.find((item) => item.id === nodeId)
    const instanceIds = new Set<number>()
    const channelIds = new Set<number>()
    if (!node) return { instanceIds: [], channelIds: [] }
    const explicitBinding = Object.prototype.hasOwnProperty.call(node.data, 'instanceId')
    const flowInstanceId = getFlowNodeInstanceId(node.data)
    if (flowInstanceId !== null) instanceIds.add(flowInstanceId)
    for (const binding of bindings.value) {
      if (binding.nodeId !== nodeId) continue
      const productName = node.data.productName ?? node.data.label
      if (productName && binding.productName !== productName) continue
      for (const instance of binding.instances ?? []) {
        const id = Number(instance.instanceId)
        if (!Number.isFinite(id) || id <= 0) continue
        // Explicit null means unbound; a changed instanceId must not pull the
        // previous instance back from an older channel-bindings response.
        if (explicitBinding && id !== flowInstanceId) continue
        instanceIds.add(id)
        for (const raw of instance.channelIds ?? []) {
          const channelId = Number(raw)
          if (Number.isFinite(channelId) && channelId > 0) channelIds.add(channelId)
        }
      }
    }
    return { instanceIds: [...instanceIds], channelIds: [...channelIds] }
  }

  /** Device instances used by the saved topology, not the entire instance registry. */
  function getTopologyDevices(productName?: string) {
    const devices = new Map<number, DeviceInstanceListItem & { nodeId: string; channelIds: number[] }>()
    for (const node of topology.value?.flow_json?.nodes ?? []) {
      const product = node.data.productName ?? node.data.label
      if (!product || (productName && product !== productName)) continue
      const binding = resolveNodeBinding(node.id)
      for (const id of binding.instanceIds) {
        const existing = devices.get(id)
        if (existing) {
          existing.channelIds = [...new Set([...existing.channelIds, ...binding.channelIds])]
        } else {
          devices.set(id, { id, name: getInstanceDisplayName(id), product_name: product,
            nodeId: node.id, channelIds: binding.channelIds })
        }
      }
    }
    return [...devices.values()]
  }

  function getInstanceIds(productName: string): number[] {
    return getTopologyDevices(productName).map((item) => item.id)
  }

  function getProductInstances(productName: string): DeviceInstanceListItem[] {
    return getTopologyDevices(productName).map(({ id, name, product_name }) => ({ id, name, product_name }))
  }

  function getChannelIds(productName: string): number[] {
    return [...new Set(getTopologyDevices(productName).flatMap((item) => item.channelIds))]
  }

  /** 按 nodeId 精确查询（Visual Modeling 画布节点用） */
  function getByNodeId(nodeId: string): NodeChannelBinding | undefined {
    return bindings.value.find((b) => b.nodeId === nodeId)
  }

  /** All device instances/channels currently used by the topology. */
  function getAllInstanceIds(): number[] {
    return getTopologyDevices().map((item) => item.id)
  }

  function getAllChannelIds(): number[] {
    return [...new Set(getTopologyDevices().flatMap((item) => item.channelIds))]
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
    const instanceIds = new Set<number>()
    const channelIds = new Set<number>()
    for (const nodeId of nodeIds) {
      const binding = resolveNodeBinding(nodeId)
      binding.instanceIds.forEach((id) => instanceIds.add(id))
      binding.channelIds.forEach((id) => channelIds.add(id))
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

    const getNodeBinding = resolveNodeBinding
    const getNodeInstanceName = (nodeId: string) => {
      const id = getNodeBinding(nodeId).instanceIds[0]
      return id === undefined ? undefined : getInstanceDisplayName(id)
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
    // The inverter instance is the system identity. Adding/removing/reordering
    // PV_Group neighbours must not create a second system or change selection.
    const systems = new Map<number, TopologyEquipmentGroup>()
    for (const inverter of nodes) {
      const productName = inverter.data.productName ?? ''
      if (productName !== 'AC_Inverter' && productName !== 'Hybrid_Inverter') continue
      const inverterBinding = getNodeBinding(inverter.id)
      for (const instanceId of inverterBinding.instanceIds) {
        let system = systems.get(instanceId)
        if (!system) {
          system = {
            id: `pv:inverter:${instanceId}`, kind: 'pv', relation: 'direct',
            primaryNodeId: inverter.id, primaryNodeIds: [],
            relatedNodeId: inverter.id, relatedProductName: productName,
            nodeIds: [], instanceIds: [instanceId], channelIds: [],
            primaryInstanceIds: [], relatedInstanceIds: [instanceId],
            primaryChannelIds: [], relatedChannelIds: [],
            complete: true, displayName: getInstanceDisplayName(instanceId), pvNodes: [],
          }
          systems.set(instanceId, system)
        }
        system.nodeIds = [...new Set([...system.nodeIds, inverter.id])]
        system.relatedChannelIds = [...new Set([...system.relatedChannelIds, ...inverterBinding.channelIds])]
        for (const neighbourId of neighbours.get(inverter.id) ?? []) {
          const pv = nodeById.get(neighbourId)
          if (pv?.data.productName !== 'PV_Group' || system.pvNodes.some((item) => item.nodeId === pv.id)) continue
          const binding = getNodeBinding(pv.id)
          system.pvNodes.push({
            nodeId: pv.id, ...binding,
            displayName: getNodeInstanceName(pv.id) ?? pv.data.label ?? 'PV Group',
          })
        }
        system.primaryNodeIds = system.pvNodes.map((item) => item.nodeId)
        system.primaryNodeId = system.primaryNodeIds[0] ?? system.relatedNodeId
        system.primaryInstanceIds = [...new Set(system.pvNodes.flatMap((item) => item.instanceIds))]
        system.primaryChannelIds = [...new Set(system.pvNodes.flatMap((item) => item.channelIds))]
        system.nodeIds = [...new Set([...system.nodeIds, ...system.primaryNodeIds])]
        system.instanceIds = [...new Set([instanceId, ...system.primaryInstanceIds])]
        system.channelIds = [...new Set([...system.relatedChannelIds, ...system.primaryChannelIds])]
      }
    }
    return [...systems.values()]
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
    return instances.value.find((item) => Number(item.id) === instanceId)?.name
      ?? bindings.value.flatMap((binding) => binding.instances).find((item) => Number(item.instanceId) === instanceId)?.instanceName
      ?? String(instanceId)
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
  const dieselGroups = computed(() => {
    const ids = getInstanceIds('Diesel')
    return ids.map((instanceId) => ({
      id: `diesel:${instanceId}`, instanceId, displayName: getInstanceDisplayName(instanceId),
    }))
  })
  const selectedDieselId = computed(() => {
    const ids = dieselGroups.value.map((group) => group.instanceId)
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
    getTopologyDevices,
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
    dieselGroups,
    selectedPvGroupId,
    selectedBatterySystemId,
    selectedPvGroup,
    selectedBatterySystem,
    getInstanceDisplayName,
    selectedDieselInstanceId,
    selectedDieselId,
  }
})
