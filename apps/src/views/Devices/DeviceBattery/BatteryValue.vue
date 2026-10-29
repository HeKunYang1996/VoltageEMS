<template>
  <div class="devices-battery-monitoring">
    <el-tabs v-model="activeTab" type="card" class="devices-battery-monitoring__tabs">
      <el-tab-pane label="Battery" name="battery">
        <InstanceValueMonitoring v-if="activeTab === 'battery'" :instance-ids="batteryInstanceIds" />
      </el-tab-pane>
      <el-tab-pane v-if="converterProductName" :label="converterLabel" name="converter">
        <InstanceValueMonitoring v-if="activeTab === 'converter'" :instance-ids="converterInstanceIds" />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import InstanceValueMonitoring from '@/components/device/InstanceValueMonitoring.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import { productDisplayName } from '@/utils/deviceMeasurements'

const topoStore = useDeviceTopologyStore()
const activeTab = ref('battery')
const batteryInstanceIds = computed(() => topoStore.selectedBatteryGroup?.primaryInstanceIds ?? [])
const converterInstanceIds = computed(() => topoStore.selectedBatteryGroup?.relatedInstanceIds ?? [])
const converterProductName = computed(() => topoStore.selectedBatteryGroup?.relatedProductName ?? '')
const converterLabel = computed(() => productDisplayName(converterProductName.value))
watch(converterProductName, (name) => {
  if (!name) activeTab.value = 'battery'
})
</script>

<style scoped lang="scss">
.devices-battery-monitoring { height: 100%; min-height: 0; }
.devices-battery-monitoring__tabs { height: 100%; }
:deep(.devices-battery-monitoring__tabs .el-tab-pane) { height: 100%; }
</style>
