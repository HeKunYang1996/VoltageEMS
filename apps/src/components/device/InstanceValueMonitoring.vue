<template>
  <div class="instance-monitoring">
    <div v-if="channelIds.length > 1" class="instance-monitoring__toolbar">
      <span>Channel:</span>
      <el-select v-model="selectedChannelId" size="small" fit-input-width>
        <el-option v-for="id in channelIds" :key="id" :label="String(id)" :value="id" />
      </el-select>
    </div>
    <LoadingBg :loading="loading">
      <el-empty v-if="error" :image="''" description="Unable to load device points" />
      <DeviceMonitoringTable
        v-else
        :left-table-data="leftTableData"
        :right-table-data="rightTableData"
      />
    </LoadingBg>
  </div>
</template>

<script setup lang="ts">
import LoadingBg from '@/components/common/LoadingBg.vue'
import DeviceMonitoringTable from '@/components/device/DeviceMonitoringTable.vue'
import { useInstanceChannelMonitoring } from '@/composables/useInstanceChannelMonitoring'

const props = defineProps<{ instanceIds: number[] }>()
const { channelIds, selectedChannelId, leftTableData, rightTableData, loading, error } =
  useInstanceChannelMonitoring(() => props.instanceIds)
</script>

<style scoped lang="scss">
.instance-monitoring { height: 100%; min-height: 0; display: flex; flex-direction: column; }
.instance-monitoring > .loading-bg { flex: 1; height: auto; min-height: 0; }
.instance-monitoring__toolbar {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 0.16rem;
  padding-bottom: 0.1rem;
  color: var(--vt-text-primary);
  font-size: 0.14rem;
}
.instance-monitoring__toolbar .el-select { width: 1.2rem; }
</style>
