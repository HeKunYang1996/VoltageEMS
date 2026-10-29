<template>
  <div class="pv-overview">
    <el-empty v-if="!instanceId && !topoStore.selectedBatterySystem" description="No Battery device" />
    <el-empty v-else-if="error" description="Unable to load device points" />
    <div v-else class="pv-overview__right">
      <p v-if="linkedConverterSummary" class="pv-overview__links">{{ linkedConverterSummary }}</p>
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
import { computed } from 'vue'
import { formatNumber } from '@/utils/common'
import { useInstanceMeasurements } from '@/composables/useInstanceMeasurements'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import { BATTERY_OVERVIEW_MEASUREMENTS, selectMeasurements } from '@/utils/deviceMeasurements'

const topoStore = useDeviceTopologyStore()
const instanceId = computed(() => topoStore.selectedBatterySystem?.primaryInstanceIds[0])
const linkedConverterSummary = computed(() => {
  const converters = topoStore.selectedBatterySystem?.converters ?? []
  const pcs = converters.filter((item) => item.productName === 'PCS').length
  const hybrid = converters.filter((item) => item.productName === 'Hybrid_Inverter').length
  const parts: string[] = []
  if (pcs) parts.push(`PCS: ${pcs}`)
  if (hybrid) parts.push(`Hybrid Inverter: ${hybrid}`)
  return parts.length ? parts.join(' · ') : ''
})
const { measurements, values, error } = useInstanceMeasurements(() => instanceId.value)
const batteryCardData = computed(() =>
  selectMeasurements(measurements.value, BATTERY_OVERVIEW_MEASUREMENTS).map((point) => ({
    pointId: point.measurement_id,
    title: point.name,
    unit: point.unit ?? '',
    value: formatNumber(values.value[String(point.measurement_id)]),
  })),
)
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

  .pv-overview__links {
    width: 100%;
    margin: 0 0 0.08rem;
    font-size: 0.14rem;
    color: var(--vt-text-secondary, rgba(255, 255, 255, 0.65));
  }

  .pv-overview__right {
    width: 48.2%;
    height: 100%;
    overflow: auto;
    display: flex;
    flex-wrap: wrap;
    gap: 0.16rem;
    align-content: flex-start;
    position: relative;
    z-index: 2;

    .battery-card {
      height: calc((100% - 0.48rem) / 4);
      width: calc((100% - 0.32rem) / 3);
    }
  }
}
</style>
