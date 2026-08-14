<template>
  <div class="devices-pv vt-page-shell">
    <!-- 页面头部 -->
    <div class="devices-pv__header vt-page-header">
      <div class="devices-pv__tabs vt-page-tabs">
        <el-button
          :type="activeTab === 'overview' ? 'primary' : 'warning'"
          @click="handleTabClick('overview')"
          class="devices-pv__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmCurrentIcon" class="devices-pv__tab-icon vt-page-tab-icon" />
          Overview
        </el-button>
        <el-button
          :type="activeTab === 'monitoring' ? 'primary' : 'warning'"
          @click="handleTabClick('monitoring')"
          class="devices-pv__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmHistoryIcon" class="devices-pv__tab-icon vt-page-tab-icon" />
          Value Monitoring
        </el-button>
      </div>
      <div v-if="topoStore.pvGroups.length > 1" class="device-group-selector">
        <span>PV System:</span>
        <el-select v-model="topoStore.selectedPvGroupId" size="small" fit-input-width :title="topoStore.selectedPvGroup?.displayName ?? ''">
          <el-option v-for="group in topoStore.pvGroups" :key="group.id" :label="group.displayName" :value="group.id">
            <span class="select-option-text" :title="group.displayName">{{ group.displayName }}</span>
          </el-option>
        </el-select>
      </div>
    </div>
    <!-- 路由内容区域 -->
    <div class="devices-pv__content vt-page-content">
      <router-view />
    </div>
  </div>
</template>

<script setup lang="ts">
// 正确引入SVG图标，避免部署后图片加载不出�?
import alarmCurrentIcon from '@/assets/icons/alarm-current.svg'
import alarmHistoryIcon from '@/assets/icons/alarm-history.svg'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'

// 响应式数�?
const route = useRoute()
const router = useRouter()
const topoStore = useDeviceTopologyStore()

// 根据当前路由计算激活的标签
const activeTab = computed(() => {
  const path = route.path
  if (path.includes('/monitoring')) {
    return 'monitoring'
  }
  return 'overview'
})

// 处理标签点击事件
const handleTabClick = (tab: 'overview' | 'monitoring') => {
  if (tab === 'overview') {
    router.push('/devices/devicesPV/overview')
  } else {
    router.push('/devices/devicesPV/monitoring')
  }
}
</script>

<style scoped lang="scss">
.devices-pv {
  .devices-pv__header { display: flex; align-items: center; justify-content: space-between; }
  .device-group-selector { display: flex; align-items: center; gap: 0.16rem; margin-left: auto; }
  .device-group-selector .el-select { width: 2rem; }
  .devices-pv__content {
    min-height: 0;
  }
}
</style>
