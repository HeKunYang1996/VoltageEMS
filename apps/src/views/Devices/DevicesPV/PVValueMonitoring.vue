<template>
  <div class="pv__content">
    <div v-if="pvChIds.length > 1" class="channel-toolbar">
      <span class="channel-toolbar__label">Channel:</span>
      <el-select
        v-model="selectedPvChId"
        size="small"
        class="channel-toolbar__select"
        fit-input-width
      >
        <el-option v-for="id in pvChIds" :key="id" :label="String(id)" :value="id" />
      </el-select>
    </div>
    <div class="devices-pv__tables">
      <LoadingBg :loading="globalStore.loading">
        <DeviceMonitoringTable :leftTableData="leftTableData" :rightTableData="rightTableData" />
      </LoadingBg>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import LoadingBg from '@/components/common/LoadingBg.vue'
import DeviceMonitoringTable from '@/components/device/DeviceMonitoringTable.vue'
import { useGlobalStore } from '@/stores/global'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import type { LeftTableItem, RightTableItem } from '@/types/deviceMonitoring'
import { getPointsTables } from '@/api/channelsManagement'
import type { PointInfoResponse } from '@/types/channelConfiguration'
import useTopologySubscribe from '@/composables/useTopologySubscribe'

const globalStore = useGlobalStore()
const topoStore = useDeviceTopologyStore()

// PV 页面统一由拓扑中的 AC_Inverter 节点提供通道。
const pvChIds = computed(() => topoStore.selectedPvGroup?.relatedChannelIds ?? [])
const selectedPvChId = ref<number>()
const pvChId = computed<number | undefined>(() => selectedPvChId.value)

const leftTableData = ref<LeftTableItem[]>([])
const rightTableData = ref<RightTableItem[]>([])

async function loadPointTable(chId: number) {
  try {
    const res = await getPointsTables(chId)
    if (res?.success && res.data) {
      const data = res.data as PointInfoResponse
      leftTableData.value =
        data.telemetry?.map((p) => ({
          pointId: p.point_id,
          name: p.signal_name || '',
          unit: p.unit || '',
          value: null,
          updateTime: null,
        })) || []
      rightTableData.value =
        data.signal?.map((p) => ({
          pointId: p.point_id,
          name: p.signal_name || '',
          status: null,
          updateTime: null,
        })) || []
    }
  } catch (err) {
    console.error('[PVValueMonitoring] 加载点位表失败:', err)
  }
}

watch(
  pvChIds,
  (ids) => {
    if (!ids.includes(selectedPvChId.value ?? -1)) selectedPvChId.value = ids[0]
  },
  { immediate: true },
)
watch(
  pvChId,
  (id) => {
    if (id !== undefined) loadPointTable(id)
  },
  { immediate: true },
)

const makeHandlers = () => ({
  onBatchDataUpdate: (data: any) => {
    const chId = pvChId.value
    const tUpdate = data.updates?.find((i: any) => i.channel_id === chId && i.data_type === 'T')
    if (tUpdate) {
      const { values = {}, ts = {} } = tUpdate
      leftTableData.value.forEach((item) => {
        const v = values[String(item.pointId)]
        const t = ts[String(item.pointId)]
        if (v != null) item.value = v
        if (t != null) item.updateTime = t
      })
    }
    const sUpdate = data.updates?.find((i: any) => i.channel_id === chId && i.data_type === 'S')
    if (sUpdate) {
      const { values = {}, ts = {} } = sUpdate
      rightTableData.value.forEach((item) => {
        const v = values[String(item.pointId)]
        const t = ts[String(item.pointId)]
        if (v != null) item.status = v
        if (t != null) item.updateTime = t
      })
    }
  },
})

useTopologySubscribe(
  () => pvChIds.value,
  { source: 'comsrv', dataTypes: ['T', 'S'], interval: 1000 },
  makeHandlers(),
)
</script>

<style scoped lang="scss">
.pv__content {
  width: 100%;
  height: calc(100% - 0.4rem);

  .group-toolbar,
  .channel-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.16rem;
    height: 0.38rem;
    margin-bottom: 0.1rem;
  }
  .channel-toolbar__label {
    color: var(--vt-text-primary);
    font-size: 0.14rem;
  }
  .channel-toolbar__select {
    width: 1.2rem;
  }

  .devices-pv__tables {
    width: 100%;
    height: 100%;
    gap: 0.2rem;
  }
}
</style>
