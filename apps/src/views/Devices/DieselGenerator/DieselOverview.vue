<template>
  <div class="pv-overview">
    <div class="pv-overview__header">
      <div class="card-item" v-for="item in energyCardData" :key="item.title">
        <PVCard :title="item.title" :icon="item.icon" :value="item.value" :unit="item.unit" />
      </div>
    </div>
    <div class="pv-overview__content"></div>
  </div>
</template>
<script setup lang="ts">
import powerIcon from '@/assets/icons/Power.svg'
import oilIcon from '@/assets/icons/Oil.svg'
import voltageIcon from '@/assets/icons/Voltage.svg'
import coolantTempIcon from '@/assets/icons/CoolantTemp.svg'
import { formatNumber } from '@/utils/common'
import { watch, ref, reactive, computed } from 'vue'
import useTopologySubscribe from '@/composables/useTopologySubscribe'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'

const topoStore = useDeviceTopologyStore()
const dgInstanceId = computed<number | undefined>(() => topoStore.getInstanceIds('Diesel')[0])
const wsData = ref<any>(null)

useTopologySubscribe(
  () => topoStore.getInstanceIds('Diesel'),
  { source: 'inst', dataTypes: ['A', 'M', 'P'] as any, interval: 1000 },
  { onBatchDataUpdate: (data: any) => { wsData.value = data } },
)

const energyCardData = reactive([
  { pointId: 1,  title: 'Power',       icon: powerIcon,       value: '-', unit: 'kW' },
  { pointId: 13, title: 'oil',         icon: oilIcon,         value: '-', unit: '%' },
  { pointId: 3,  title: 'Voltage',     icon: voltageIcon,     value: '-', unit: 'V' },
  { pointId: 14, title: 'Temperature', icon: coolantTempIcon, value: '-', unit: '℃' },
])

watch(
  wsData,
  (data) => {
    if (!data?.updates?.length) return
    const instanceId = dgInstanceId.value
    const mUpdate = data.updates.find(
      (item: any) => item.channel_id === instanceId && item.data_type === 'M',
    )
    if (!mUpdate) return
    const values = mUpdate.values || {}
    energyCardData.forEach((item: any) => {
      const pointValue = values[item.pointId]
      if (pointValue !== undefined && pointValue !== null) {
        item.value = formatNumber(pointValue)
      }
    })
  },
  { deep: true, immediate: true },
)
</script>
<style scoped lang="scss">
.pv-overview {
  height: 100%;
  width: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
  z-index: 1;

  .pv-overview__header {
    width: 100%;
    padding: 0.2rem 0;
    display: flex;
    justify-content: space-between;
    align-items: center;

    &::after {
      content: '';
      position: absolute;
      top: -0.72rem;
      left: -0.2rem;
      width: calc(100% + 0.4rem);
      height: calc(100vh - 0.85rem);
      background-image: url('@/assets/images/DieselGenerator-bg.png');
      background-repeat: no-repeat;
      background-size: 100% 100%;
      background-position: center;
      z-index: 0;
      pointer-events: none;
    }

    .card-item {
      height: 1rem;
      width: calc((100% - 1.2rem) / 4);
      position: relative;
      z-index: 1;
    }
  }

  .pv-overview__content {
    width: 100%;
    flex: 1;
  }
}
</style>
