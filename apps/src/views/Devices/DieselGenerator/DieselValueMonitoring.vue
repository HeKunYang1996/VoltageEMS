<template>
  <div class="voltage-class pv__content">
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

// 无回退值：store 未加载时为 undefined，响应式触发后再请求/订阅
const dgChId = computed<number | undefined>(() => topoStore.getChannelIds('Diesel')[0])

const leftTableData  = ref<LeftTableItem[]>([])
const rightTableData = ref<RightTableItem[]>([])

async function loadPointTable(chId: number) {
  try {
    const res = await getPointsTables(chId)
    if (res?.success && res.data) {
      const data = res.data as PointInfoResponse
      leftTableData.value = data.telemetry?.map((p) => ({
        pointId: p.point_id, name: p.signal_name || '', unit: p.unit || '', value: null, updateTime: null,
      })) || []
      rightTableData.value = data.signal?.map((p) => ({
        pointId: p.point_id, name: p.signal_name || '', status: null, updateTime: null,
      })) || []
    }
  } catch (err) {
    console.error('[DieselValueMonitoring] 加载点位表失败:', err)
  }
}

watch(dgChId, (id) => { if (id !== undefined) loadPointTable(id) }, { immediate: true })

const makeHandlers = () => ({
  onBatchDataUpdate: (data: any) => {
    const chId = dgChId.value
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
  () => dgChId.value !== undefined ? [dgChId.value] : [],
  { source: 'comsrv', dataTypes: ['T', 'S'], interval: 1000 },
  makeHandlers(),
)
</script>

<style scoped lang="scss">
.voltage-class.pv__content {
  width: 100%;
  height: calc(100% - 0.4rem);

  .devices-pv__tables {
    width: 100%;
    height: 100%;
    gap: 0.2rem;
  }
}
</style>
