<template>
  <div class="voltage-class devices-battery vt-page-shell">
    <!-- 页面头部 -->
    <div class="devices-battery__header vt-page-header">
      <div class="devices-battery__tabs vt-page-tabs">
        <el-button
          :type="activeTab === 'overview' ? 'primary' : 'warning'"
          @click="handleTabClick('overview')"
          class="devices-battery__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmCurrentIcon" class="devices-battery__tab-icon vt-page-tab-icon" />
          Overview
        </el-button>
        <el-button
          :type="activeTab === 'value' ? 'primary' : 'warning'"
          @click="handleTabClick('value')"
          class="devices-battery__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmHistoryIcon" class="devices-battery__tab-icon vt-page-tab-icon" />
          Value Monitoring
        </el-button>
        <el-button
          :type="activeTab === 'management' ? 'primary' : 'warning'"
          @click="handleTabClick('management')"
          class="devices-battery__tab-btn vt-page-tab-btn"
        >
          <img :src="alarmCurrentIcon" class="devices-battery__tab-icon vt-page-tab-icon" />
          Battery Management
        </el-button>
      </div>
    </div>
    <!-- 路由内容区域 -->
    <div class="devices-battery__content vt-page-content">
      <router-view />
    </div>
  </div>
</template>

<script setup lang="ts">
import alarmCurrentIcon from '@/assets/icons/alarm-current.svg'
import alarmHistoryIcon from '@/assets/icons/alarm-history.svg'

const route = useRoute()
const router = useRouter()

// 根据当前路由计算激活的标签
const activeTab = computed(() => {
  const path = route.path
  if (path.includes('/value')) {
    return 'value'
  } else if (path.includes('/management')) {
    return 'management'
  }
  return 'overview'
})

// 处理标签点击事件
const handleTabClick = (tab: 'overview' | 'value' | 'management') => {
  if (tab === 'overview') {
    router.push('/devices/deviceBattery/overview')
  } else if (tab === 'value') {
    router.push('/devices/deviceBattery/value')
  } else {
    router.push('/devices/deviceBattery/management')
  }
}
</script>

<style scoped lang="scss">
.voltage-class.devices-battery {
  .devices-battery__content {
    padding-top: 0.2rem;
    min-height: 0;
  }
}
</style>
