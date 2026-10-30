<template>
  <div class="devices-meter vt-page-shell">
    <div v-if="meterInstances.length" class="devices-meter__header">
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
      <el-empty v-if="!meterInstances.length" description="No Meter device" />
      <InstanceValueMonitoring v-else :instance-ids="selectedMeterId === undefined ? [] : [selectedMeterId]" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import InstanceValueMonitoring from '@/components/device/InstanceValueMonitoring.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'

const topoStore = useDeviceTopologyStore()
const selectedMeterId = ref<number>()
const meterInstances = computed(() => topoStore.getProductInstances('Meter'))
const selectedMeterName = computed(
  () => meterInstances.value.find((item) => item.id === selectedMeterId.value)?.name ?? '',
)
watch(meterInstances, (items) => {
  if (!items.some((item) => item.id === selectedMeterId.value)) selectedMeterId.value = items[0]?.id
}, { immediate: true })
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
.device-instance-selector {
  display: flex;
  align-items: center;
  gap: 0.16rem;
}
.device-instance-selector {
  margin-left: auto;
}
.device-instance-selector .el-select {
  width: 2rem;
}
.devices-meter__content {
  min-height: 0;
}
</style>
