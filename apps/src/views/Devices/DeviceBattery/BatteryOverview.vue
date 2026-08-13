<template>
  <div class="pv-overview">
    <div class="pv-overview__right">
      <BatteryCard
        class="battery-card"
        v-for="item in batteryCardData"
        :key="item.pointId"
        :title="item.title"
        :value="item.value"
        :unit="item.unit"
      ></BatteryCard>
    </div>
  </div>
</template>
<script setup lang="ts">
import { formatNumber } from '@/utils/common'
import { watch, ref, computed } from 'vue'
import useTopologySubscribe from '@/composables/useTopologySubscribe'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'

interface BatteryCardItem {
  title: string
  value?: number | null | string
  unit?: string
  pointId: number
}

const topoStore = useDeviceTopologyStore()
const batteryInstanceIds = computed(() => topoStore.selectedBatteryGroup?.primaryInstanceIds ?? [])
const batteryInstanceId = computed<number | undefined>(
  () => topoStore.getInstanceIds('Battery')[0] ?? batteryInstanceIds.value[0],
)
const wsData = ref<any>(null)

// 拓扑加载后自动订阅，加载前静默等待
useTopologySubscribe(
  () => batteryInstanceIds.value,
  { source: 'inst', dataTypes: ['A', 'M', 'P'] as any, interval: 1000 },
  { onBatchDataUpdate: (data: any) => { wsData.value = data } },
)

watch(
  wsData,
  (data) => {
    if (!data?.updates?.length) return
    const instanceId = batteryInstanceId.value
    const mUpdate = data.updates.find(
      (item: any) => item.channel_id === instanceId && item.data_type === 'M',
    )
    if (!mUpdate) return
    const values = mUpdate.values || {}
    batteryCardData.value.forEach((item) => {
      const pointValue = values[item.pointId]
      if (pointValue !== undefined && pointValue !== null) {
        if (item.pointId === 15) {
          item.value = pointValue === 1 ? 'Charge' : 'Discharge'
        } else {
          item.value = formatNumber(pointValue)
        }
      }
    })
  },
  { deep: true, immediate: true },
)

const batteryCardData = ref<BatteryCardItem[]>([
  { pointId: 15, title: 'Status',               value: '-', unit: '' },
  { pointId: 3,  title: 'SoC',                  value: '-', unit: '%' },
  { pointId: 4,  title: 'SoH',                  value: '-', unit: '%' },
  { pointId: 1,  title: 'Voltage',              value: '-', unit: 'V' },
  { pointId: 2,  title: 'Current',              value: '-', unit: 'A' },
  { pointId: 3,  title: 'Power',                value: '-', unit: 'KW' },
  { pointId: 7,  title: 'Max Cell Voltage',     value: '-', unit: 'V' },
  { pointId: 8,  title: 'Min Cell Voltage',     value: '-', unit: 'V' },
  { pointId: 9,  title: 'Avg Cell Voltage',     value: '-', unit: 'V' },
  { pointId: 10, title: 'Cell Voltage Difference', value: '-', unit: 'V' },
  { pointId: 11, title: 'Avg Cell Temperature', value: '-', unit: '℃' },
])
</script>
<style scoped lang="scss">
.pv-overview {
  height: 100%;
  width: 100%;
  display: flex;
  justify-content: flex-end;
  position: relative;
  z-index: 1;

  &::after {
    content: '';
    position: absolute;
    left: -0.2rem;
    width: calc(100% + 0.4rem);
    height: calc(100% + 0.2rem);
    background-image: url('@/assets/images/battery-bg.png');
    background-repeat: no-repeat;
    background-size: 60% 100%;
    background-position: left center;
    z-index: 0;
    pointer-events: none;
  }

  .pv-overview__right {
    width: 48.2%;
    height: 100%;
    overflow: auto;
    display: flex;
    flex-wrap: wrap;
    gap: 0.16rem;
    position: relative;
    z-index: 2;

    .battery-card {
      height: calc((100% - 0.48rem) / 4);
      width: calc((100% - 0.32rem) / 3);
    }
  }
}
</style>
