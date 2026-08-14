<template>
  <div class="devices-pv__content">
    <div class="devices-pv__tables">
      <LoadingBg :loading="globalStore.loading">
        <el-tabs v-model="activeTab" type="card" class="devices-pv__tabs">
          <el-tab-pane label="Battery" name="battery">
            <div v-if="batteryChIds.length > 1" class="channel-toolbar">
              <span class="channel-toolbar__label">Channel:</span>
              <el-select v-model="selectedBatteryChId" size="small" fit-input-width>
                <el-option v-for="id in batteryChIds" :key="id" :label="String(id)" :value="id" />
              </el-select>
            </div>
            <DeviceMonitoringTable
              :leftTableData="BatteryleftTableData"
              :rightTableData="BatteryrightTableData"
            />
          </el-tab-pane>
          <el-tab-pane label="PCS" name="pcs">
            <div v-if="pcsChIds.length > 1" class="channel-toolbar">
              <span class="channel-toolbar__label">Channel:</span>
              <el-select v-model="selectedPcsChId" size="small" fit-input-width>
                <el-option v-for="id in pcsChIds" :key="id" :label="String(id)" :value="id" />
              </el-select>
            </div>
            <DeviceMonitoringTable
              :leftTableData="PCSleftTableData"
              :rightTableData="PCSrightTableData"
            />
          </el-tab-pane>
        </el-tabs>
      </LoadingBg>
    </div>
  </div>
</template>

<script setup lang="ts">
import LoadingBg from '@/components/common/LoadingBg.vue'
import { useGlobalStore } from '@/stores/global'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import type { LeftTableItem, RightTableItem } from '@/types/deviceMonitoring'
import { getPointsTables } from '@/api/channelsManagement'
import type { PointInfoResponse } from '@/types/channelConfiguration'
import { ref, computed, watch } from 'vue'
import useTopologySubscribe from '@/composables/useTopologySubscribe'

const globalStore = useGlobalStore()
const topoStore = useDeviceTopologyStore()

// 无回退值：store 未加载时为 undefined，响应式触发后再请求/订阅
const batteryChIds = computed(() => topoStore.selectedBatteryGroup?.primaryChannelIds ?? [])
const pcsChIds = computed(() => topoStore.selectedBatteryGroup?.relatedChannelIds ?? [])
const selectedBatteryChId = ref<number>()
const selectedPcsChId = ref<number>()
const batteryChId = computed<number | undefined>(() => selectedBatteryChId.value)
const pcsChId = computed<number | undefined>(() => selectedPcsChId.value)

const BatteryleftTableData  = ref<LeftTableItem[]>([])
const BatteryrightTableData = ref<RightTableItem[]>([])
const PCSleftTableData      = ref<LeftTableItem[]>([])
const PCSrightTableData     = ref<RightTableItem[]>([])

// ── 点位表加载 ─────────────────────────────────────────────────────────────────

async function loadPointTables(batteryId?: number, pcsId?: number) {
  try {
    const [batteryRes, pcsRes] = await Promise.all([
      batteryId === undefined ? Promise.resolve(null) : getPointsTables(batteryId),
      pcsId === undefined ? Promise.resolve(null) : getPointsTables(pcsId),
    ])
    if (batteryRes?.success && batteryRes.data) {
      const d = batteryRes.data as PointInfoResponse
      BatteryleftTableData.value = d.telemetry?.map((p) => ({
        pointId: p.point_id, name: p.signal_name || '', unit: p.unit || '', value: null, updateTime: null,
      })) || []
      BatteryrightTableData.value = d.signal?.map((p) => ({
        pointId: p.point_id, name: p.signal_name || '', status: null, updateTime: null,
      })) || []
    }
    if (pcsRes?.success && pcsRes.data) {
      const d = pcsRes.data as PointInfoResponse
      PCSleftTableData.value = d.telemetry?.map((p) => ({
        pointId: p.point_id, name: p.signal_name || '', unit: p.unit || '', value: null, updateTime: null,
      })) || []
      PCSrightTableData.value = d.signal?.map((p) => ({
        pointId: p.point_id, name: p.signal_name || '', status: null, updateTime: null,
      })) || []
    }
  } catch (err) {
    console.error('[BatteryValue] 加载点位表失败:', err)
  }
}

// 拓扑加载后（或通道变化时）拉取点位表；immediate: true 覆盖 onMounted
watch(batteryChIds, (ids) => {
  if (!ids.includes(selectedBatteryChId.value ?? -1)) selectedBatteryChId.value = ids[0]
}, { immediate: true })
watch(pcsChIds, (ids) => {
  if (!ids.includes(selectedPcsChId.value ?? -1)) selectedPcsChId.value = ids[0]
}, { immediate: true })
watch([batteryChId, pcsChId], ([bId, pId]) => {
  if (bId !== undefined || pId !== undefined) loadPointTables(bId, pId)
}, { immediate: true })

// ── WebSocket 订阅 ─────────────────────────────────────────────────────────────

const makeHandlers = () => ({
  onBatchDataUpdate: (data: any) => {
    const bId = batteryChId.value
    const pId = pcsChId.value

    const ch2T = data.updates?.find((i: any) => i.channel_id === bId && i.data_type === 'T')
    if (ch2T) {
      const { values = {}, ts = {} } = ch2T
      BatteryleftTableData.value.forEach((item) => {
        const v = values[String(item.pointId)]
        const t = ts[String(item.pointId)]
        if (v != null) item.value = v
        if (t != null) item.updateTime = t
      })
    }

    const ch2S = data.updates?.find((i: any) => i.channel_id === bId && i.data_type === 'S')
    if (ch2S) {
      const { values = {}, ts = {} } = ch2S
      BatteryrightTableData.value.forEach((item) => {
        const v = values[String(item.pointId)]
        const t = ts[String(item.pointId)]
        if (v != null) item.status = v
        if (t != null) item.updateTime = t
      })
    }

    const ch1T = data.updates?.find((i: any) => i.channel_id === pId && i.data_type === 'T')
    if (ch1T) {
      const { values = {}, ts = {} } = ch1T
      PCSleftTableData.value.forEach((item) => {
        const v = values[String(item.pointId)]
        const t = ts[String(item.pointId)]
        if (v != null) item.value = v
        if (t != null) item.updateTime = t
      })
    }

    const ch1S = data.updates?.find((i: any) => i.channel_id === pId && i.data_type === 'S')
    if (ch1S) {
      const { values = {}, ts = {} } = ch1S
      PCSrightTableData.value.forEach((item) => {
        const v = values[String(item.pointId)]
        const t = ts[String(item.pointId)]
        if (v != null) item.status = v
        if (t != null) item.updateTime = t
      })
    }
  },
})

useTopologySubscribe(
  () => {
    return [batteryChId.value, pcsChId.value].filter((id): id is number => id !== undefined)
  },
  { source: 'comsrv', dataTypes: ['T', 'S'], interval: 1000 },
  makeHandlers(),
)

const activeTab = ref<'battery' | 'pcs'>('battery')
</script>

<style scoped lang="scss">
.devices-pv__content {
  width: 100%;
  height: 100%;

  .devices-pv__tables {
    position: relative;
    height: 100%;
    width: 100%;
  }
}

:deep(.devices-pv__tabs.el-tabs) { height: 100%; }
:deep(.devices-pv__tabs .el-tab-pane) { height: 100%; }
.group-toolbar, .channel-toolbar { display: flex; justify-content: flex-end; align-items: center; gap: 0.16rem; height: 0.38rem; margin-bottom: 0.1rem; }
.channel-toolbar__label { color: var(--vt-text-primary); font-size: 0.14rem; }
.channel-toolbar .el-select { width: 1.2rem; }
.group-toolbar .el-select { width: 2rem; }
</style>
