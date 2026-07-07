<template>
  <div class="voltage-class curves">
    <div class="curves__content">
      <!-- 工具栏 -->
      <div class="curves__toolbar">
        <!-- 左侧设备筛选，teleported="false" 让下拉框挂载在此 div 内 -->
        <div class="curves__toolbar-left" ref="deviceSelectWrapperRef">
          <el-select
            v-model="selectedDevice"
            class="device-select"
            placeholder="All Devices"
            :teleported="false"
            :append-to="deviceSelectWrapperRef"
            @change="fetchAllData"
          >
            <el-option label="All Devices" value="all" />
            <el-option
              v-for="dev in deviceConfigs"
              :key="dev.id"
              :label="`${dev.label} (${dev.subtitle})`"
              :value="dev.id"
            />
          </el-select>
        </div>

        <!-- 右侧时间选择 -->
        <div class="curves__toolbar-right">
          <div class="curves__toolbar-time-btns" @click="handleTimeBtnClick">
            <el-date-picker
              v-if="selectedTimeBtn === 'custom'"
              v-model="rangeArray"
              type="datetimerange"
              value-format="YYYY-MM-DD HH:mm:ss"
              format="YYYY-MM-DD HH:mm:ss"
              range-separator="To"
              :default-time="defaultTime"
              start-placeholder="Select Start Date"
              end-placeholder="Select End Date"
              :teleported="false"
              @change="handleDateRangeChange"
            />
            <div
              v-for="btn in timeBtnList"
              :key="btn.value"
              class="curves__toolbar-time-btn"
              :class="{ 'is-active': selectedTimeBtn === btn.value }"
              :data-value="btn.value"
            >
              {{ btn.label }}
            </div>
          </div>
        </div>
      </div>

      <!-- 图表区域（可滚动） -->
      <div class="curves__charts-outer">
      <LoadingBg :loading="isConfigLoading || isLoading">
      <div class="curves__charts">
        <template v-for="dev in activeDevices" :key="dev.id">
          <div class="curves__device-section">
            <!-- 设备区块标题（点击收起/展开） -->
            <div
              class="curves__device-header"
              :style="{ borderLeftColor: dev.accentColor }"
              @click="toggleCollapse(dev.id)"
            >
              <span class="curves__device-label">{{ dev.label }}</span>
              <span class="curves__device-sub">{{ dev.subtitle }}</span>
              <span
                class="device-collapse-arrow"
                :class="{ 'is-collapsed': collapsedMap[dev.id] }"
              >&#9660;</span>
            </div>

            <!-- 该设备的点位图表网格（可收起） -->
            <div class="curves__device-charts" v-if="!collapsedMap[dev.id]">
              <div
                class="curves__chart-item"
                v-for="(point, pIdx) in dev.points"
                :key="`${dev.id}-${point.id}`"
              >
                <ModuleCard :title="`${point.name} Curve`">
                  <lineChart
                    lazy
                    :xAxiosOption="{ xAxiosData: getPointXAxis(dev.redisKey, point.id) }"
                    :yAxiosOption="{ yUnit: point.unit }"
                    :series="getPointSeries(dev.redisKey, point.id, point.name, getColor(pIdx))"
                  />
                </ModuleCard>
              </div>
            </div>
          </div>
        </template>

        <!-- 空状态 -->
        <div v-if="!isConfigLoading && !isLoading && activeDevices.length === 0" class="curves__empty">
          No data available
        </div>
      </div>
      </LoadingBg>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { batchQueryHistory } from '@/api/Statistic/overview'
import { getInstancePoints } from '@/api/devicesManagement'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import dayjs from 'dayjs'
import { getRecentHoursRange, getRecentDaysRange, getRecentWeekRange } from '@/utils/date.ts'
import type { BatchQueryResponse } from '@/types/Statistics/OverView'
import type { InstanceMeasurementItem } from '@/types/deviceConfiguration'

// ─── 类型定义 ───────────────────────────────────────────────
interface PointDef {
  id: string
  name: string
  unit: string
}

interface DeviceConfig {
  id: string
  label: string
  subtitle: string
  redisKey: string
  accentColor: string
  points: PointDef[]
}

interface ChartSeriesData {
  xLabels: string[]
  values: number[]
}

const PRODUCT_LABELS: Record<string, string> = {
  Battery: 'Battery',
  Diesel: 'DG',
  PCS: 'PCS',
  'PV DCDC': 'PV',
  PVInverter: 'PV',
  Load: 'Load',
}

const PRODUCT_ACCENT: Record<string, string> = {
  Battery: '#6DD400',
  Diesel: '#F6C85F',
  PCS: '#4FADF7',
  'PV DCDC': '#69CBFF',
  PVInverter: '#69CBFF',
  Load: '#FF4D4F',
}

const getProductLabel = (productName: string) => PRODUCT_LABELS[productName] ?? productName
const getAccentColor = (productName: string) => PRODUCT_ACCENT[productName] ?? '#69CBFF'

const mapMeasurementPoints = (measurements: Record<string, InstanceMeasurementItem>): PointDef[] => {
  return Object.entries(measurements)
    .map(([id, item]) => ({
      id,
      name: item.name || `Point ${id}`,
      unit: item.unit || '',
    }))
    .sort((a, b) => Number(a.id) - Number(b.id))
}

// ─── 颜色配置 ─────────────────────────────────────────────────
const CHART_COLORS = [
  '#69CBFF', '#4FADF7', '#1D86FF',
  '#6DD400', '#F6C85F', '#FF4D4F',
  '#9B59B6', '#E67E22', '#2ECC71',
  '#3498DB', '#E74C3C', '#1ABC9C',
]
const getColor = (index: number) => CHART_COLORS[index % CHART_COLORS.length]

// ─── 拓扑与设备配置 ───────────────────────────────────────────
const topoStore = useDeviceTopologyStore()
const deviceConfigs = ref<DeviceConfig[]>([])
const isConfigLoading = ref(false)

const loadDeviceConfigs = async () => {
  isConfigLoading.value = true
  try {
    if (!topoStore.loaded) {
      await topoStore.load()
    }

    const instances = topoStore.bindings.flatMap((binding) =>
      binding.instances.map((inst) => ({ binding, inst })),
    )

    const configs = await Promise.all(
      instances.map(async ({ binding, inst }) => {
        try {
          const res = await getInstancePoints(inst.instanceId)
          const points = res.success && res.data
            ? mapMeasurementPoints(res.data.measurements)
            : []
          return {
            id: String(inst.instanceId),
            label: getProductLabel(binding.productName),
            subtitle: inst.instanceName,
            redisKey: `inst:${inst.instanceId}:M`,
            accentColor: getAccentColor(binding.productName),
            points,
          } satisfies DeviceConfig
        } catch {
          return null
        }
      }),
    )

    deviceConfigs.value = configs.filter(
      (item): item is DeviceConfig => Boolean(item && item.points.length > 0),
    )
    initCollapseState()
  } catch (error) {
    console.error('Failed to load curves device configs:', error)
    deviceConfigs.value = []
  } finally {
    isConfigLoading.value = false
  }
}

// ─── 时间选择 ─────────────────────────────────────────────────
const defaultTime: [Date, Date] = [new Date(2000, 0, 1, 0, 0, 0), new Date(2000, 0, 1, 23, 59, 59)]
const rangeArray = ref<string[]>([])
const selectedTimeBtn = ref<'6h' | '1d' | '1w' | '1m' | 'custom'>('6h')
const timeBtnList: { label: string; value: '6h' | '1d' | '1w' | '1m' | 'custom' }[] = [
  { label: 'Custom', value: 'custom' },
  { label: '6 Hour', value: '6h' },
  { label: '1 Day', value: '1d' },
  { label: '1 Week', value: '1w' },
  { label: '1 Month', value: '1m' },
]

// ─── 设备选择框包装 ref ───────────────────────────────────────
const deviceSelectWrapperRef = ref<HTMLElement | null>(null)

// ─── 区块折叠状态 ─────────────────────────────────────────────
const collapsedMap = reactive<Record<string, boolean>>({})

const initCollapseState = () => {
  deviceConfigs.value.forEach((dev, index) => {
    if (collapsedMap[dev.id] === undefined) {
      collapsedMap[dev.id] = index !== 0
    }
  })
}

const toggleCollapse = (deviceId: string) => {
  const wasCollapsed = collapsedMap[deviceId]
  collapsedMap[deviceId] = !wasCollapsed
  if (wasCollapsed) {
    const dev = deviceConfigs.value.find((item) => item.id === deviceId)
    if (dev && !loadedDeviceIds.value.has(deviceId)) {
      void fetchDevicesData([dev])
    }
    nextTick(() => {
      setTimeout(() => {
        window.dispatchEvent(new Event('resize'))
      }, 50)
    })
  }
}

// ─── 设备筛选 ─────────────────────────────────────────────────
const selectedDevice = ref<string>('all')

const activeDevices = computed(() => {
  if (selectedDevice.value === 'all') return deviceConfigs.value
  return deviceConfigs.value.filter((d) => d.id === selectedDevice.value)
})

watch(selectedDevice, (value) => {
  if (value === 'all') {
    deviceConfigs.value.forEach((dev, index) => {
      collapsedMap[dev.id] = index !== 0
    })
  } else {
    collapsedMap[value] = false
  }
  void fetchExpandedDevicesData()
})

// ─── 图表数据 ─────────────────────────────────────────────────
const isLoading = ref(false)
let fetchAbortController: AbortController | null = null
const chartDataMap = ref<Map<string, ChartSeriesData>>(new Map())
const loadedDeviceIds = ref<Set<string>>(new Set())

const getPointXAxis = (redisKey: string, pointId: string): string[] => {
  return chartDataMap.value.get(`${redisKey}:${pointId}`)?.xLabels ?? []
}

const getPointSeries = (redisKey: string, pointId: string, name: string, color: string) => {
  const data = chartDataMap.value.get(`${redisKey}:${pointId}`)
  return [{ name, data: data?.values ?? [], color }]
}

// ─── 时间范围 ─────────────────────────────────────────────────
const getTimeRange = (): { start_time: string; end_time: string } => {
  if (selectedTimeBtn.value === 'custom' && rangeArray.value.length === 2) {
    return {
      start_time: dayjs(rangeArray.value[0]).toISOString(),
      end_time: dayjs(rangeArray.value[1]).toISOString(),
    }
  }
  const rangeMap: Record<string, { start?: string; end?: string }> = {
    '6h': getRecentHoursRange(6),
    '1d': getRecentDaysRange(1),
    '1w': getRecentWeekRange(),
    '1m': getRecentDaysRange(30),
  }
  const range = rangeMap[selectedTimeBtn.value] ?? getRecentHoursRange(6)
  return { start_time: range.start!, end_time: range.end! }
}

const formatLabel = (ts: string): string => {
  const btn = selectedTimeBtn.value
  return btn === '6h' || btn === '1d'
    ? dayjs(ts).format('HH:mm')
    : dayjs(ts).format('MM-DD HH:mm')
}

const formatValue = (v: number | null | undefined): number =>
  Number(Number(v ?? 0).toFixed(3))

const getExpandedDevices = () =>
  activeDevices.value.filter((dev) => !collapsedMap[dev.id])

const fetchDevicesData = async (devs: DeviceConfig[]) => {
  if (devs.length === 0) return

  fetchAbortController?.abort()
  fetchAbortController = new AbortController()
  const signal = fetchAbortController.signal

  isLoading.value = true

  const { start_time, end_time } = getTimeRange()

  const allRequests = devs.flatMap((dev) => {
    const chunks: Array<Array<{ redis_key: string; point_id: string }>> = []
    for (let i = 0; i < dev.points.length; i += 20) {
      chunks.push(
        dev.points.slice(i, i + 20).map((p) => ({
          redis_key: dev.redisKey,
          point_id: p.id,
        })),
      )
    }
    return chunks.map((chunk) =>
      batchQueryHistory({ start_time, end_time, limit_per_series: 300, series: chunk }, signal),
    )
  })

  try {
    const results = await Promise.all(allRequests)
    const newMap = new Map(chartDataMap.value)

    results.forEach((res) => {
      const responses: BatchQueryResponse[] = res.data?.series ?? []
      responses.forEach((r) => {
        const sorted = [...(r.data ?? [])].sort(
          (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
        )
        newMap.set(`${r.redis_key}:${r.point_id}`, {
          xLabels: sorted.map((p) => formatLabel(p.timestamp)),
          values: sorted.map((p) => formatValue(p.value)),
        })
      })
    })

    chartDataMap.value = newMap
    devs.forEach((dev) => loadedDeviceIds.value.add(dev.id))
  } catch (error: unknown) {
    const err = error as { name?: string; code?: string }
    if (err?.name === 'CanceledError' || err?.code === 'ERR_CANCELED' || err?.name === 'AbortError') {
      return
    }
    console.error('Failed to load curves data:', error)
  } finally {
    isLoading.value = false
  }
}

const fetchExpandedDevicesData = async () => {
  loadedDeviceIds.value = new Set()
  chartDataMap.value = new Map()
  await fetchDevicesData(getExpandedDevices())
}

const fetchAllData = async () => {
  loadedDeviceIds.value = new Set()
  chartDataMap.value = new Map()
  await fetchDevicesData(getExpandedDevices())
}

onUnmounted(() => {
  fetchAbortController?.abort()
})

// ─── 事件处理 ─────────────────────────────────────────────────
const handleTimeBtnClick = (event: MouseEvent) => {
  const btn = (event.target as HTMLElement).closest(
    '.curves__toolbar-time-btn',
  ) as HTMLElement | null
  if (btn?.dataset.value) {
    selectedTimeBtn.value = btn.dataset.value as '6h' | '1d' | '1w' | '1m' | 'custom'
    rangeArray.value = []
    if (selectedTimeBtn.value !== 'custom') {
      void fetchAllData()
    }
  }
}

const handleDateRangeChange = () => {
  if (selectedTimeBtn.value === 'custom' && rangeArray.value.length === 2) {
    void fetchAllData()
  }
}

onMounted(async () => {
  await loadDeviceConfigs()
  await fetchExpandedDevicesData()
})
</script>

<style scoped lang="scss">
.voltage-class.curves {
  height: 100%;
  width: 100%;

  .curves__content {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
  }

  // ── 工具栏 ───────────────────────────────────────
  .curves__toolbar {
    flex-shrink: 0;
    padding-bottom: 0.2rem;
    display: flex;
    align-items: center;
    justify-content: space-between;

    .curves__toolbar-left {
      position: relative;
      display: flex;
      align-items: center;

      .device-select {
        width: 2.2rem;
        --vt-select-width: 2.2rem;
      }
    }

    .curves__toolbar-right {
      display: flex;
      align-items: center;

      .curves__toolbar-time-btns {
        position: relative;
        height: 0.32rem;
        display: flex;
        align-items: center;

        .curves__toolbar-time-btn {
          height: 0.32rem;
          line-height: 0.32rem;
          padding: 0 0.1rem;
          font-size: 0.14rem;
          background: transparent;
          border-right: 0.01rem solid rgba(255, 255, 255, 0.2);
          cursor: pointer;
          color: rgba(255, 255, 255, 0.85);

          &:last-child {
            border-right: none;
          }

          &.is-active {
            background: rgba(255, 255, 255, 0.2);
          }
        }
      }
    }
  }

  // ── 图表区域外层（撑满剩余高度） ─────────────────
  .curves__charts-outer {
    flex: 1;
    min-height: 0;
  }

  // ── 图表滚动区域（LoadingBg 的 slot 内容） ────────
  .curves__charts {
    flex: 1;
    overflow-y: auto;

    // 自定义滚动条
    &::-webkit-scrollbar {
      width: 0.04rem;
    }
    &::-webkit-scrollbar-track {
      background: rgba(255, 255, 255, 0.05);
    }
    &::-webkit-scrollbar-thumb {
      background: rgba(255, 255, 255, 0.2);
      border-radius: 0.02rem;
    }
  }

  // ── 设备区块 ──────────────────────────────────────
    .curves__device-section {
      margin-bottom: 0.24rem;

      .curves__device-header {
        display: flex;
        align-items: center;
        gap: 0.12rem;
        padding: 0.1rem 0.12rem 0.12rem 0.12rem;
        margin-bottom: 0.12rem;
        border-left: 0.03rem solid #4fadf7;
        cursor: pointer;
        user-select: none;

        &:hover {
          background: rgba(255, 255, 255, 0.04);
          border-radius: 0 0.04rem 0.04rem 0;
        }

        .curves__device-label {
          font-size: 0.18rem;
          font-weight: 700;
          color: rgba(255, 255, 255, 0.9);
          font-family: Arimo, sans-serif;
        }

        .curves__device-sub {
          font-size: 0.12rem;
          color: rgba(255, 255, 255, 0.45);
          font-family: Arimo, sans-serif;
          flex: 1;
        }

        .device-collapse-arrow {
          font-size: 0.16rem;
          color: rgba(255, 255, 255, 0.85);
          transition: transform 0.25s ease;
          display: inline-flex;
          align-items: center;
          justify-content: center;
          width: 0.22rem;
          height: 0.22rem;
          border-radius: 50%;
          background: rgba(255, 255, 255, 0.1);
          flex-shrink: 0;

          &.is-collapsed {
            transform: rotate(-90deg);
          }

          &:hover {
            background: rgba(255, 255, 255, 0.2);
          }
        }
      }

      .curves__device-charts {
        display: flex;
        flex-wrap: wrap;
        gap: 0.2rem;
      }
    }

  // ── 图表卡片 ──────────────────────────────────────
  .curves__chart-item {
    width: calc((100% - 0.4rem) / 3);
    height: 2.4rem;
    flex-shrink: 0;
  }

  // ── 空状态 ────────────────────────────────────────
  .curves__empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 3rem;
    color: rgba(255, 255, 255, 0.4);
    font-size: 0.14rem;
    font-family: Arimo, sans-serif;
  }
}
</style>
