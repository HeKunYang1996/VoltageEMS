<template>
  <div class="devices-diesel vt-page-shell">
    <!-- 页面头部 -->
    <div class="devices-diesel__header vt-page-header">
      <div class="devices-diesel__tabs vt-page-tabs">
        <el-button
          :type="activeTab === 'overview' ? 'primary' : 'warning'"
          @click="handleTabClick('overview')"
          class="devices-diesel__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmCurrentIcon" class="devices-diesel__tab-icon vt-page-tab-icon" />
          Overview
        </el-button>
        <el-button
          :type="activeTab === 'monitoring' ? 'primary' : 'warning'"
          @click="handleTabClick('monitoring')"
          class="devices-diesel__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmHistoryIcon" class="devices-diesel__tab-icon vt-page-tab-icon" />
          Value Monitoring
        </el-button>
      </div>
      <div v-if="dieselInstances.length > 1" class="device-instance-selector">
        <span>Diesel Generator:</span>
        <el-select v-model="topoStore.selectedDieselInstanceId" size="small" fit-input-width :title="selectedDieselName">
          <el-option v-for="item in dieselInstances" :key="item.id" :label="item.name" :value="item.id">
            <span class="select-option-text" :title="item.name">{{ item.name }}</span>
          </el-option>
        </el-select>
      </div>
    </div>
    <!-- 路由内容区域 -->
    <div class="devices-diesel__main vt-page-content">
      <router-view />
    </div>
  </div>
</template>

<script setup lang="ts">
// 正确引入SVG图标，避免部署后图片加载不出�?
import alarmCurrentIcon from '@/assets/icons/alarm-current.svg'
import alarmHistoryIcon from '@/assets/icons/alarm-history.svg'
import { computed } from 'vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'

// 响应式数
const route = useRoute()
const router = useRouter()
const topoStore = useDeviceTopologyStore()
const dieselInstances = computed(() => topoStore.getLogicalDeviceInstanceIds('diesel').map((id) => ({
  id,
  name: topoStore.instances.find((item) => item.id === id)?.name
    ?? topoStore.bindings.flatMap((binding) => binding.instances).find((item) => item.instanceId === id)?.instanceName
    ?? String(id),
})))
const selectedDieselName = computed(() => dieselInstances.value.find((item) => item.id === topoStore.selectedDieselInstanceId)?.name ?? '')

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
    router.push('/devices/dieselGenerator/overview')
  } else {
    router.push('/devices/dieselGenerator/monitoring')
  }
}
</script>

<style scoped lang="scss">
.devices-diesel {
  .devices-diesel__header { display: flex; align-items: center; justify-content: space-between; }
  .device-instance-selector { display: flex; align-items: center; gap: 0.16rem; margin-left: auto; }
  .device-instance-selector .el-select { width: 1.8rem; }
  .devices-diesel__main {
    min-height: 0;
  }
}
</style>
