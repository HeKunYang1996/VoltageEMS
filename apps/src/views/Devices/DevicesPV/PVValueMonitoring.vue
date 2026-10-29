<template>
  <div class="pv__content">
    <InstanceValueMonitoring :instance-ids="instanceIds" />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import InstanceValueMonitoring from '@/components/device/InstanceValueMonitoring.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'

const topoStore = useDeviceTopologyStore()
const instanceIds = computed(() => topoStore.selectedPvGroup?.relatedInstanceIds ?? [])
</script>

<style scoped lang="scss">
.pv__content { width: 100%; height: 100%; min-height: 0; }
</style>
