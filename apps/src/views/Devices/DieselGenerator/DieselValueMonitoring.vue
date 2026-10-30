<template>
  <div class="diesel-monitoring">
    <InstanceValueMonitoring :instance-ids="instanceIds" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import InstanceValueMonitoring from '@/components/device/InstanceValueMonitoring.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'

const topoStore = useDeviceTopologyStore()
const instanceIds = computed(() => topoStore.selectedDieselId === null ? [] : [topoStore.selectedDieselId])
</script>

<style scoped lang="scss">
.diesel-monitoring { height: 100%; min-height: 0; }
</style>
