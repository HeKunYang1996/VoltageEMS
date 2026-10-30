<template>
  <div class="curves">
    <div class="curves__content">
      <!-- 工具栏 -->
      <div class="curves__toolbar">
        <!-- 左侧设备筛选，teleported="false" 让下拉框挂载在此 div 内 -->
        <div class="curves__toolbar-left" ref="deviceSelectWrapperRef">
          <el-select
            v-model="selectedDevice"
            class="device-select"
            fit-input-width
            :title="selectedDevice === 'all' ? 'All Devices' : (deviceConfigs.find((dev) => dev.id === selectedDevice)?.label ?? '')"
            placeholder="All Devices"
            :teleported="false"
            :append-to="deviceSelectWrapperRef"
          >
            <el-option label="All Devices" value="all" />
            <el-option
              v-for="dev in deviceConfigs"
              :key="dev.id"
              :label="`${dev.label} (${dev.subtitle})`"
              :value="dev.id"
            ><span class="select-option-text" :title="`${dev.label} (${dev.subtitle})`">{{ dev.label }} ({{ dev.subtitle }})</span></el-option>
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
      <LoadingBg :loading="isConfigLoading">
      <div class="curves__charts">
        <template v-for="dev in activeDevices" :key="dev.id">
          <div class="curves__device-section" :aria-busy="pendingDeviceIds.has(dev.id)">
            <!-- 设备区块标题（点击收起/展开） -->
            <div
              class="curves__device-header"
              :style="{ borderLeftColor: dev.accentColor }"
              @click="toggleCollapse(dev.id)"
            >
              <span class="curves__device-label">{{ dev.label }}</span>
              <span class="curves__device-sub">{{ dev.subtitle }}</span>
              <span v-if="pendingDeviceIds.has(dev.id)" class="curves__device-loading" role="status">Loading...</span>
              <span
                class="device-collapse-arrow"
                :class="{ 'is-collapsed': collapsedMap[dev.id] }"
              >&#9660;</span>
            </div>

            <!-- 该设备的点位图表网格（可收起） -->
            <div v-if="!collapsedMap[dev.id] && deviceErrors[dev.id]" role="alert">
              Historical data unavailable
            </div>
            <div class="curves__device-charts" v-if="!collapsedMap[dev.id]">
              <div
                class="curves__chart-item"
                v-for="(point, pIdx) in dev.points"
                :key="`${dev.id}-${point.id}`"
              >
                <ModuleCard :title="`${point.name} Curve`">
                  <lineChart
                    lazy
                    :xAxiosOption="getPointChartProps(dev.redisKey, point.id, point.name, getColor(pIdx)).xAxis"
                    :yAxiosOption="{ yUnit: point.unit }"
                    :series="getPointChartProps(dev.redisKey, point.id, point.name, getColor(pIdx)).series"
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
  values: (number | null)[]
}

const PRODUCT_ACCENT: Record<string, string> = {
  Battery: '#6DD400',       // --vt-color-chart-online
  Diesel: '#F6C85F',        // --vt-color-chart-dg
  PCS: '#4FADF7',           // --vt-color-chart-ess
  'PV DCDC': '#69CBFF',     // --vt-color-chart-pv
  PVInverter: '#69CBFF',    // --vt-color-chart-pv
  AC_Inverter: '#69CBFF',   // --vt-color-chart-pv
  Load: '#FF4D4F',          // --vt-color-chart-alarm
  Meter: '#FF4D4F',         // --vt-color-chart-alarm
}

const getAccentColor = (productName: string) => PRODUCT_ACCENT[productName] ?? '#69CBFF'

const mapMeasurementPoints = (measurements: InstanceMeasurementItem[]): PointDef[] => {
  return measurements
    .map((item) => ({
      id: String(item.measurement_id),
      name: item.name || `Point ${item.measurement_id}`,
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
let disposed = false
let configVersion = 0

const loadDeviceConfigs = async () => {
  const version = ++configVersion
  isConfigLoading.value = true
  try {
    const instances = topoStore.getTopologyDevices()

    const configs = await Promise.all(
      instances.map(async (instance) => {
        try {
          const res = await getInstancePoints(instance.id)
          const points = res.success && res.data
            ? mapMeasurementPoints(res.data.measurements)
            : []
          return {
            id: String(instance.id),
            label: instance.name,
            subtitle: instance.product_name,
            redisKey: `inst:${instance.id}:M`,
            accentColor: getAccentColor(instance.product_name),
            points,
          } satisfies DeviceConfig
        } catch {
          return null
        }
      }),
    )

    if (disposed || version !== configVersion) return false
    deviceConfigs.value = configs.filter(
      (item): item is DeviceConfig => Boolean(item && item.points.length > 0),
    )
    initCollapseState()
    if (selectedDevice.value !== 'all' && !deviceConfigs.value.some((dev) => dev.id === selectedDevice.value)) {
      selectedDevice.value = 'all'
    }
    return true
  } catch (error) {
    if (disposed || version !== configVersion) return false
    console.error('Failed to load curves device configs:', error)
    deviceConfigs.value = []
    return false
  } finally {
    if (version === configVersion) isConfigLoading.value = false
  }
}

// ─── 时间选择 ─────────────────────────────────────────────────
const defaultTime: [Date, Date] = [new Date(2000, 0, 1, 0, 0, 0), new Date(2000, 0, 1, 23, 59, 59)]
const rangeArray = ref<string[] | null>([])
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
const pendingDeviceIds = ref<Set<string>>(new Set())
const isLoading = computed(() => pendingDeviceIds.value.size > 0)
const deviceRequests = new Map<string, { controller: AbortController; promise: Promise<void> }>()
const deviceErrors = reactive<Record<string, boolean>>({})
let queryGeneration = 0
let chartRangeKey = JSON.stringify([selectedTimeBtn.value, null])
const chartDataMap = ref<Map<string, ChartSeriesData>>(new Map())
const loadedDeviceIds = ref<Set<string>>(new Set())

interface PointChartProps {
  source: ChartSeriesData | undefined
  xAxis: { xAxiosData: string[] }
  series: { name: string; data: (number | null)[]; color: string }[]
}
const pointChartProps = new Map<string, PointChartProps>()
// Panel/loading updates must not reset unchanged ECharts options and animations.
const getPointChartProps = (redisKey: string, pointId: string, name: string, color: string): PointChartProps => {
  const key = `${redisKey}:${pointId}`
  const source = chartDataMap.value.get(key)
  const cached = pointChartProps.get(key)
  if (cached && cached.source === source && cached.series[0].name === name && cached.series[0].color === color) return cached
  const props = {
    source,
    xAxis: { xAxiosData: source?.xLabels ?? [] },
    series: [{ name, data: source?.values ?? [], color }],
  }
  pointChartProps.set(key, props)
  return props
}

// ─── 时间范围 ─────────────────────────────────────────────────
const getTimeRange = (): { start_time: string; end_time: string } | null => {
  if (selectedTimeBtn.value === 'custom') {
    if (!Array.isArray(rangeArray.value) || rangeArray.value.length !== 2) return null
    const start = dayjs(rangeArray.value[0])
    const end = dayjs(rangeArray.value[1])
    if (!start.isValid() || !end.isValid() || !end.isAfter(start)) return null
    return {
      start_time: start.toISOString(),
      end_time: end.toISOString(),
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

const formatValue = (v: number | null | undefined): number | null =>
  typeof v === 'number' && Number.isFinite(v) ? Number(v.toFixed(3)) : null

const getExpandedDevices = () =>
  activeDevices.value.filter((dev) => !collapsedMap[dev.id])

const fetchDevicesData = async (devs: DeviceConfig[]) => {
  const range = getTimeRange()
  if (disposed || isConfigLoading.value || !range) return
  const generation = queryGeneration
  await Promise.all(devs.map((dev) => {
    if (loadedDeviceIds.value.has(dev.id)) return
    const pending = deviceRequests.get(dev.id)
    if (pending) return pending.promise

    const controller = new AbortController()
    const request = { controller, promise: Promise.resolve() }
    deviceRequests.set(dev.id, request)
    pendingDeviceIds.value = new Set(deviceRequests.keys())
    deviceErrors[dev.id] = false
    request.promise = (async () => {
      try {
        const chunks = []
        for (let i = 0; i < dev.points.length; i += 20) {
          chunks.push(dev.points.slice(i, i + 20).map((point) => ({
            redis_key: dev.redisKey, point_id: point.id,
          })))
        }
        const results = await Promise.all(chunks.map((series) => batchQueryHistory({
          ...range, limit_per_series: 300, series,
        }, controller.signal)))
        if (controller.signal.aborted || generation !== queryGeneration || disposed) return
        if (results.some((result) => !result.success)) throw new Error('History query failed')
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
        loadedDeviceIds.value.add(dev.id)
      } catch (error: unknown) {
        if (controller.signal.aborted || generation !== queryGeneration || disposed) return
        const err = error as { name?: string; code?: string }
        if (err?.name === 'CanceledError' || err?.code === 'ERR_CANCELED' || err?.name === 'AbortError') return
        deviceErrors[dev.id] = true
        controller.abort()
        console.error('Failed to load curves data:', error)
      } finally {
        if (deviceRequests.get(dev.id) === request) {
          deviceRequests.delete(dev.id)
          pendingDeviceIds.value = new Set(deviceRequests.keys())
        }
      }
    })()
    return request.promise
  }))
}

const fetchExpandedDevicesData = async () => {
  await fetchDevicesData(getExpandedDevices())
}

const resetChartData = () => {
  queryGeneration++
  deviceRequests.forEach((request) => request.controller.abort())
  deviceRequests.clear()
  pendingDeviceIds.value = new Set()
  loadedDeviceIds.value = new Set()
  chartDataMap.value = new Map()
  pointChartProps.clear()
  Object.keys(deviceErrors).forEach((id) => { delete deviceErrors[id] })
}

const fetchAllData = async () => {
  const key = JSON.stringify([selectedTimeBtn.value,
    selectedTimeBtn.value === 'custom' ? rangeArray.value : null])
  if (key !== chartRangeKey) {
    chartRangeKey = key
    resetChartData()
  }
  await fetchDevicesData(getExpandedDevices())
}

onUnmounted(() => {
  disposed = true
  configVersion++
  resetChartData()
})

// ─── 事件处理 ─────────────────────────────────────────────────
const handleTimeBtnClick = (event: MouseEvent) => {
  const btn = (event.target as HTMLElement).closest(
    '.curves__toolbar-time-btn',
  ) as HTMLElement | null
  if (btn?.dataset.value) {
    if (btn.dataset.value === selectedTimeBtn.value) return
    selectedTimeBtn.value = btn.dataset.value as '6h' | '1d' | '1w' | '1m' | 'custom'
    rangeArray.value = []
    void fetchAllData()
  }
}

const handleDateRangeChange = () => {
  if (selectedTimeBtn.value === 'custom') {
    void fetchAllData()
  }
}

watch(
  () => JSON.stringify(topoStore.getTopologyDevices().map((item) => [item.id, item.product_name])
    .sort((a, b) => Number(a[0]) - Number(b[0]))),
  async () => {
    resetChartData()
    deviceConfigs.value = []
    if (await loadDeviceConfigs()) await fetchExpandedDevicesData()
  },
  { immediate: true },
)
onMounted(() => { if (!topoStore.loaded) void topoStore.load() })
</script>

<style scoped lang="scss">
.curves {
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
    height: calc(100% - 0.52rem);
    min-height: 0;
  }

  // ── 图表滚动区域（LoadingBg 的 slot 内容） ────────
  .curves__charts {
    height: 100%;
    overflow-y: auto;
    scrollbar-gutter: stable;

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
        border-left: 0.03rem solid var(--vt-color-chart-ess);
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

        .curves__device-loading {
          font-size: 0.12rem;
          color: rgba(255, 255, 255, 0.65);
          white-space: nowrap;
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
