<template>
  <div class="devices-battery-monitoring">
    <el-tabs v-model="activeTab" type="card" class="devices-battery-monitoring__tabs">
      <el-tab-pane label="Battery" name="battery">
        <InstanceValueMonitoring v-if="activeTab === 'battery'" :instance-ids="batteryInstanceIds" />
      </el-tab-pane>
      <el-tab-pane
        v-for="tab in converterTabs"
        :key="tab.productName"
        :label="tab.label"
        :name="tab.productName"
      >
        <div v-if="activeTab === tab.productName" class="devices-battery-monitoring__converter">
          <div v-if="tab.instances.length" class="devices-battery-monitoring__toolbar">
            <template v-if="tab.instances.length > 1">
              <span>Device:</span>
              <el-select
                v-model="selectedConverterInstanceId"
                size="small"
                fit-input-width
                :title="selectedConverterName"
              >
                <el-option
                  v-for="item in tab.instances"
                  :key="item.instanceId"
                  :label="item.name"
                  :value="item.instanceId"
                >
                  <span class="select-option-text" :title="item.name">{{ item.name }}</span>
                </el-option>
              </el-select>
            </template>
            <span v-else class="devices-battery-monitoring__device-name">Device: {{ tab.instances[0].name }}</span>
          </div>
          <InstanceValueMonitoring :instance-ids="converterMonitoringInstanceIds" />
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import InstanceValueMonitoring from '@/components/device/InstanceValueMonitoring.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import { productDisplayName } from '@/utils/deviceMeasurements'

const CONVERTER_PRODUCTS = ['PCS', 'Hybrid_Inverter'] as const

const topoStore = useDeviceTopologyStore()
const activeTab = ref('battery')
const selectedConverterInstanceId = ref<number>()

const batteryInstanceIds = computed(() => topoStore.selectedBatterySystem?.primaryInstanceIds ?? [])

const converterTabs = computed(() => {
  const converters = topoStore.selectedBatterySystem?.converters ?? []
  return CONVERTER_PRODUCTS.flatMap((productName) => {
    const links = converters.filter((item) => item.productName === productName)
    const instances = links.flatMap((link) =>
      link.instanceIds.map((instanceId) => ({
        instanceId,
        name: topoStore.getInstanceDisplayName(instanceId),
      })),
    )
    if (!instances.length) return []
    const label =
      instances.length > 1
        ? `${productDisplayName(productName)} (${instances.length})`
        : productDisplayName(productName)
    return [{ productName, label, instances }]
  })
})

const activeConverterTab = computed(() =>
  converterTabs.value.find((tab) => tab.productName === activeTab.value),
)

const converterMonitoringInstanceIds = computed(() => {
  const tab = activeConverterTab.value
  if (!tab?.instances.length) return []
  const id = tab.instances.some((item) => item.instanceId === selectedConverterInstanceId.value)
    ? selectedConverterInstanceId.value
    : tab.instances[0]?.instanceId
  return id === undefined ? [] : [id]
})

const selectedConverterName = computed(() => {
  const id = selectedConverterInstanceId.value
  if (id === undefined) return ''
  return topoStore.getInstanceDisplayName(id)
})

watch(
  () => topoStore.selectedBatterySystem?.id,
  () => {
    activeTab.value = 'battery'
    selectedConverterInstanceId.value = undefined
  },
)

watch(
  activeConverterTab,
  (tab) => {
    if (!tab?.instances.length) {
      selectedConverterInstanceId.value = undefined
      return
    }
    if (!tab.instances.some((item) => item.instanceId === selectedConverterInstanceId.value)) {
      selectedConverterInstanceId.value = tab.instances[0]?.instanceId
    }
  },
  { immediate: true },
)

watch(converterTabs, (tabs) => {
  if (activeTab.value === 'battery') return
  if (!tabs.some((tab) => tab.productName === activeTab.value)) activeTab.value = 'battery'
})
</script>

<style scoped lang="scss">
.devices-battery-monitoring {
  height: 100%;
  min-height: 0;
}
.devices-battery-monitoring__tabs {
  height: 100%;
}
.devices-battery-monitoring__converter {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.devices-battery-monitoring__toolbar {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 0.16rem;
  padding-bottom: 0.1rem;
  color: var(--vt-text-primary);
  font-size: 0.14rem;
}
.devices-battery-monitoring__toolbar .el-select {
  width: 1.6rem;
}
.devices-battery-monitoring__device-name {
  color: var(--vt-text-primary);
}
:deep(.devices-battery-monitoring__tabs .el-tab-pane) {
  height: 100%;
}
.devices-battery-monitoring__converter > .instance-monitoring {
  flex: 1;
  min-height: 0;
}
</style>
