<template>
  <div class="devices-meter vt-page-shell">
    <div v-if="meterInstances.length > 1" class="devices-meter__header">
      <div class="device-instance-selector">
        <span>Meter:</span>
        <el-select
          v-model="selectedMeterId"
          size="small"
          fit-input-width
          :title="selectedMeterName"
        >
          <el-option
            v-for="item in meterInstances"
            :key="item.id"
            :label="item.name"
            :value="item.id"
          >
            <span class="select-option-text" :title="item.name">{{ item.name }}</span>
          </el-option>
        </el-select>
      </div>
    </div>
    <div class="devices-meter__content vt-page-content">
      <div v-if="meterChannelIds.length > 1" class="channel-toolbar">
        <span>Channel:</span>
        <el-select v-model="selectedChannelId" size="small" fit-input-width>
          <el-option v-for="id in meterChannelIds" :key="id" :label="String(id)" :value="id" />
        </el-select>
      </div>
      <LoadingBg :loading="globalStore.loading">
        <DeviceMonitoringTable :leftTableData="leftTableData" :rightTableData="rightTableData" />
      </LoadingBg>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import LoadingBg from '@/components/common/LoadingBg.vue'
import DeviceMonitoringTable from '@/components/device/DeviceMonitoringTable.vue'
import { useGlobalStore } from '@/stores/global'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import { getPointsTables } from '@/api/channelsManagement'
import type { PointInfoResponse } from '@/types/channelConfiguration'
import type { LeftTableItem, RightTableItem } from '@/types/deviceMonitoring'
import useTopologySubscribe from '@/composables/useTopologySubscribe'

const globalStore = useGlobalStore()
const topoStore = useDeviceTopologyStore()
const selectedMeterId = ref<number>()
const selectedChannelId = ref<number>()
const leftTableData = ref<LeftTableItem[]>([])
const rightTableData = ref<RightTableItem[]>([])

const meterInstances = computed(() =>
  topoStore.getLogicalDeviceInstanceIds('meterLoad').map((id) => ({
    id,
    name:
      topoStore.instances.find((item) => item.id === id)?.name ??
      topoStore.bindings
        .flatMap((binding) => binding.instances)
        .find((item) => item.instanceId === id)?.instanceName ??
      String(id),
  })),
)
const selectedMeterName = computed(
  () => meterInstances.value.find((item) => item.id === selectedMeterId.value)?.name ?? '',
)
const selectedMeter = computed(() =>
  topoStore.bindings
    .filter((binding) => binding.productName === 'Meter')
    .flatMap((binding) => binding.instances)
    .find((instance) => instance.instanceId === selectedMeterId.value),
)
const meterChannelIds = computed(() => selectedMeter.value?.channelIds ?? [])

watch(
  meterInstances,
  (items) => {
    if (!items.some((item) => item.id === selectedMeterId.value))
      selectedMeterId.value = items[0]?.id
  },
  { immediate: true },
)
watch(
  meterChannelIds,
  (ids) => {
    if (!ids.includes(selectedChannelId.value ?? -1)) selectedChannelId.value = ids[0]
  },
  { immediate: true },
)

watch(
  selectedChannelId,
  async (channelId) => {
    leftTableData.value = []
    rightTableData.value = []
    if (channelId === undefined) return
    try {
      const res = await getPointsTables(channelId)
      if (!res?.success || !res.data) return
      const data = res.data as PointInfoResponse
      leftTableData.value =
        data.telemetry?.map((point) => ({
          pointId: point.point_id,
          name: point.signal_name || '',
          unit: point.unit || '',
          value: null,
          updateTime: null,
        })) ?? []
      rightTableData.value =
        data.signal?.map((point) => ({
          pointId: point.point_id,
          name: point.signal_name || '',
          status: null,
          updateTime: null,
        })) ?? []
    } catch (error) {
      console.error('[DeviceMeter] failed to load point table', error)
    }
  },
  { immediate: true },
)

useTopologySubscribe(
  () => (selectedChannelId.value === undefined ? [] : [selectedChannelId.value]),
  { source: 'comsrv', dataTypes: ['T', 'S'], interval: 1000 },
  {
    onBatchDataUpdate: (data: any) => {
      const update = data.updates?.find((item: any) => item.channel_id === selectedChannelId.value)
      if (!update) return
      const values = update.values || {}
      const timestamps = update.ts || {}
      if (update.data_type === 'T')
        leftTableData.value.forEach((item) => {
          if (values[String(item.pointId)] != null) item.value = values[String(item.pointId)]
          if (timestamps[String(item.pointId)] != null)
            item.updateTime = timestamps[String(item.pointId)]
        })
      if (update.data_type === 'S')
        rightTableData.value.forEach((item) => {
          if (values[String(item.pointId)] != null) item.status = values[String(item.pointId)]
          if (timestamps[String(item.pointId)] != null)
            item.updateTime = timestamps[String(item.pointId)]
        })
    },
  },
)
</script>

<style scoped lang="scss">
.devices-meter {
  height: 100%;
}
.devices-meter__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 0.2rem;
}
.device-instance-selector,
.channel-toolbar {
  display: flex;
  align-items: center;
  gap: 0.16rem;
}
.device-instance-selector {
  margin-left: auto;
}
.device-instance-selector .el-select {
  width: 1.8rem;
}
.channel-toolbar {
  justify-content: flex-end;
  // margin-bottom: 0.1rem;
}
.channel-toolbar > span {
  color: var(--vt-text-primary);
  font-size: 0.14rem;
}
.channel-toolbar .el-select {
  width: 1.2rem;
}
.devices-meter__content {
  min-height: 0;
}
</style>
