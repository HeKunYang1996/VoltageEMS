<template>
  <div class="voltage-class devices-pv vt-page-shell">
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
          :type="activeTab === 'curves' ? 'primary' : 'warning'"
          @click="handleTabClick('curves')"
          class="devices-pv__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmHistoryIcon" class="devices-pv__tab-icon vt-page-tab-icon" />
          Curves
        </el-button>
        <el-button
          :type="activeTab === 'operationLog' ? 'primary' : 'warning'"
          @click="handleTabClick('operationLog')"
          class="devices-pv__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmHistoryIcon" class="devices-pv__tab-icon vt-page-tab-icon" />
          Operation Log
        </el-button>
        <el-button
          :type="activeTab === 'runingLog' ? 'primary' : 'warning'"
          @click="handleTabClick('runingLog')"
          class="devices-pv__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmHistoryIcon" class="devices-pv__tab-icon vt-page-tab-icon" />
          Running Log
        </el-button>
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

// 响应式数�?
const route = useRoute()
const router = useRouter()

// 根据当前路由计算激活的标签
const activeTab = computed(() => {
  const path = route.path
  if (path.includes('/curves')) {
    return 'curves'
  } else if (path.includes('/operationLog')) {
    return 'operationLog'
  } else if (path.includes('/runingLog')) {
    return 'runingLog'
  }
  return 'overview'
})

// 处理标签点击事件
const handleTabClick = (tab: 'overview' | 'curves' | 'operationLog' | 'runingLog') => {
  if (tab === 'overview') {
    router.push('/statistics/overview')
  } else if (tab === 'curves') {
    router.push('/statistics/curves')
  } else if (tab === 'operationLog') {
    router.push('/statistics/operationLog')
  } else {
    router.push('/statistics/runingLog')
  }
}
</script>

<style scoped lang="scss">
.voltage-class.devices-pv {
  .devices-pv__content {
    min-height: 0;
  }
}
</style>
