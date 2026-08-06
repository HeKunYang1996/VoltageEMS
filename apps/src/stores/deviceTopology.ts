import { defineStore } from 'pinia'
import { ref } from 'vue'
import { getChannelBindings, getStationTopology } from '@/api/stationTopology'
import type { NodeChannelBinding, StationTopology } from '@/types/stationTopology'

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
  /** 完整站点拓扑（含 flow_json），null 表示尚未加载或站点未配置 */
  const topology = ref<StationTopology | null>(null)
  const loaded = ref(false)
  const loading = ref(false)

  /** 并行加载 channel-bindings 与完整拓扑 */
  async function load() {
    if (loading.value) return
    loading.value = true
    try {
      const [bindingsRes, topoRes] = await Promise.all([
        getChannelBindings(),
        getStationTopology(),
      ])
      if (bindingsRes?.success && bindingsRes.data) {
        bindings.value = bindingsRes.data.bindings ?? []
      }
      if (topoRes?.success && topoRes.data) {
        topology.value = topoRes.data
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

  return {
    bindings,
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
  }
})
