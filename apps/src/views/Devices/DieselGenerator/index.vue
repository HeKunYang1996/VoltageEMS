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

// 响应式数
const route = useRoute()
const router = useRouter()

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
  .devices-diesel__main {
    min-height: 0;
  }
}
</style>
