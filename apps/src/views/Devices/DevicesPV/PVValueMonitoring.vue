<template>
  <div class="pv-monitoring">
    <el-tabs v-model="activeTab" type="card" class="pv-monitoring__tabs">
      <el-tab-pane :label="inverterLabel" name="inverter">
        <InstanceValueMonitoring v-if="activeTab === 'inverter'" :instance-ids="inverterInstanceIds" />
      </el-tab-pane>
      <el-tab-pane v-for="tab in pvTabs" :key="tab.name" :label="tab.label" :name="tab.name">
        <InstanceValueMonitoring v-if="activeTab === tab.name" :instance-ids="[tab.instanceId]" />
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
const activeTab = ref('inverter')
const inverterInstanceIds = computed(() => topoStore.selectedPvGroup?.relatedInstanceIds ?? [])
const inverterLabel = computed(() => productDisplayName(topoStore.selectedPvGroup?.relatedProductName ?? 'Inverter'))
const pvTabs = computed(() => {
  const seen = new Set<number>()
  return (topoStore.selectedPvGroup?.pvNodes ?? []).flatMap((node) => node.instanceIds.flatMap((instanceId) => {
    if (seen.has(instanceId)) return []
    seen.add(instanceId)
    return [{ name: `pv:${instanceId}`, instanceId, label: topoStore.getInstanceDisplayName(instanceId) }]
  }))
})

watch(() => topoStore.selectedPvGroup?.id, () => { activeTab.value = 'inverter' }, { flush: 'sync' })
watch(pvTabs, (tabs) => {
  if (activeTab.value !== 'inverter' && !tabs.some((tab) => tab.name === activeTab.value)) activeTab.value = 'inverter'
})
</script>

<style scoped lang="scss">
.pv-monitoring { height: 100%; min-height: 0; }
.pv-monitoring__tabs { height: 100%; }
:deep(.pv-monitoring__tabs .el-tab-pane) { height: 100%; }
</style>
