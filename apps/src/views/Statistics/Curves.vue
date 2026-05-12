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
            @change="fetchAllData"
          >
            <el-option label="All Devices" value="all" />
            <el-option
              v-for="dev in DEVICE_CONFIGS"
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
      <LoadingBg :loading="isLoading">
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
            <div class="curves__device-charts" v-show="!collapsedMap[dev.id]">
              <div
                class="curves__chart-item"
                v-for="(point, pIdx) in dev.points"
                :key="point.id"
              >
                <ModuleCard :title="`${point.name} Curve`">
                  <lineChart
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
        <div v-if="!isLoading && activeDevices.length === 0" class="curves__empty">
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
import dayjs from 'dayjs'
import { getRecentHoursRange, getRecentDaysRange, getRecentWeekRange } from '@/utils/date.ts'
import type { BatchQueryResponse } from '@/types/Statistics/OverView'

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

// ─── 设备 & 点位配置 ──────────────────────────────────────────
const DEVICE_CONFIGS: DeviceConfig[] = [
  {
    id: 'pv',
    label: 'PV',
    subtitle: 'pv_01',
    redisKey: 'inst:4:M',
    accentColor: '#69CBFF',
    points: [
      { id: '1', name: 'PV Power (Array)', unit: 'kW' },
      { id: '2', name: 'PV Voltage (Array)', unit: 'V' },
      { id: '3', name: 'PV Current (Array)', unit: 'A' },
      { id: '4', name: 'Sub PVI', unit: 'kW' },
      { id: '5', name: 'Energy Today', unit: 'kWh' },
      { id: '7', name: 'PV Power', unit: 'kW' },
      { id: '8', name: 'PV Voltage', unit: 'V' },
      { id: '9', name: 'PV Current', unit: 'A' },
      { id: '10', name: 'Peak Efficiency', unit: '%' },
      { id: '11', name: 'Average Efficiency', unit: '%' },
      { id: '12', name: 'Minimum Efficiency', unit: '%' },
      { id: '13', name: 'Solar Panels', unit: '%' },
      { id: '14', name: 'PV System', unit: '%' },
      { id: '15', name: 'Energy Total', unit: 'kWh' },
    ],
  },
  {
    id: 'battery',
    label: 'Battery',
    subtitle: 'battery_01',
    redisKey: 'inst:1:M',
    accentColor: '#6DD400',
    points: [
      { id: '1', name: 'Total Voltage', unit: 'V' },
      { id: '2', name: 'Total Current', unit: 'A' },
      { id: '3', name: 'Max Battery Pack Temperature', unit: '°C' },
      { id: '4', name: 'Min Battery Pack Temperature', unit: '°C' },
      { id: '5', name: 'Charge Power', unit: 'kW' },
      { id: '6', name: 'Discharge Power', unit: 'kW' },
      { id: '7', name: 'SOC', unit: '%' },
      { id: '8', name: 'SOH', unit: '%' },
      { id: '9', name: 'Charge Energy', unit: 'kWh' },
      { id: '10', name: 'Discharge Energy', unit: 'kWh' },
      { id: '12', name: 'Max Cell Voltage', unit: 'V' },
      { id: '13', name: 'Min Cell Voltage', unit: 'V' },
      { id: '14', name: 'Avg Cell Voltage', unit: 'V' },
      { id: '15', name: 'Cell Voltage Difference', unit: 'V' },
      { id: '16', name: 'Avg Cell Temperature', unit: '°C' },
      { id: '19', name: 'Battery System', unit: '%' },
      { id: '20', name: 'Charge Energy Today', unit: 'kWh' },
      { id: '21', name: 'Discharge Energy Today', unit: 'kWh' },
      { id: '101', name: 'Daily Charge Energy', unit: 'kWh' },
      { id: '102', name: 'Daily Discharge Energy', unit: 'kWh' },
      { id: '103', name: 'Weekly Charge Energy', unit: 'kWh' },
      { id: '104', name: 'Weekly Discharge Energy', unit: 'kWh' },
      { id: '105', name: 'Monthly Charge Energy', unit: 'kWh' },
      { id: '106', name: 'Monthly Discharge Energy', unit: 'kWh' },
      { id: '107', name: 'Quarterly Charge Energy', unit: 'kWh' },
      { id: '108', name: 'Quarterly Discharge Energy', unit: 'kWh' },
    ],
  },
  {
    id: 'pcs',
    label: 'PCS',
    subtitle: 'pcs_01',
    redisKey: 'inst:3:M',
    accentColor: '#4FADF7',
    points: [
      { id: '1', name: 'Total Power', unit: 'kW' },
      { id: '2', name: 'DC Power', unit: 'kW' },
      { id: '3', name: 'Power A', unit: 'kW' },
      { id: '4', name: 'Power B', unit: 'kW' },
      { id: '5', name: 'Power C', unit: 'kW' },
      { id: '6', name: 'DC Voltage', unit: 'V' },
      { id: '7', name: 'Voltage A', unit: 'V' },
      { id: '8', name: 'Voltage B', unit: 'V' },
      { id: '9', name: 'Voltage C', unit: 'V' },
      { id: '10', name: 'Current A', unit: 'A' },
      { id: '11', name: 'Current B', unit: 'A' },
      { id: '12', name: 'Current C', unit: 'A' },
      { id: '13', name: 'Temperature', unit: '°C' },
      { id: '17', name: 'AC Frequency', unit: 'Hz' },
    ],
  },
  {
    id: 'dg',
    label: 'DG',
    subtitle: 'diesel_gen_01',
    redisKey: 'inst:2:M',
    accentColor: '#F6C85F',
    points: [
      { id: '1', name: 'Diesel Power', unit: 'kW' },
      { id: '2', name: 'Diesel Energy', unit: 'kWh' },
      { id: '3', name: 'Diesel Voltage', unit: 'V' },
      { id: '4', name: 'Diesel Current A', unit: 'A' },
      { id: '5', name: 'Diesel Current B', unit: 'A' },
      { id: '6', name: 'Diesel Current C', unit: 'A' },
      { id: '7', name: 'Diesel Voltage A', unit: 'V' },
      { id: '8', name: 'Diesel Voltage B', unit: 'V' },
      { id: '9', name: 'Diesel Voltage C', unit: 'V' },
      { id: '10', name: 'Diesel Power A', unit: 'kW' },
      { id: '11', name: 'Diesel Power B', unit: 'kW' },
      { id: '12', name: 'Diesel Power C', unit: 'kW' },
      { id: '13', name: 'Diesel Oil', unit: '%' },
      { id: '14', name: 'Diesel Temperature', unit: '°C' },
      { id: '16', name: 'Frequency', unit: 'Hz' },
      { id: '18', name: 'Diesel Energy Today', unit: 'kWh' },
    ],
  },
  {
    id: 'load',
    label: 'Load',
    subtitle: 'Load_01',
    redisKey: 'inst:6:M',
    accentColor: '#FF4D4F',
    points: [
      { id: '1', name: 'Load Power', unit: 'kW' },
      { id: '2', name: 'Load Energy', unit: 'kWh' },
    ],
  },
]

// ─── 颜色配置 ─────────────────────────────────────────────────
const CHART_COLORS = [
  '#69CBFF', '#4FADF7', '#1D86FF',
  '#6DD400', '#F6C85F', '#FF4D4F',
  '#9B59B6', '#E67E22', '#2ECC71',
  '#3498DB', '#E74C3C', '#1ABC9C',
]
const getColor = (index: number) => CHART_COLORS[index % CHART_COLORS.length]

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
const toggleCollapse = (deviceId: string) => {
  const wasCollapsed = collapsedMap[deviceId]
  collapsedMap[deviceId] = !wasCollapsed
  // 展开时触发 resize，确保 ECharts 在可见后重新测量容器尺寸，避免图例错位
  if (wasCollapsed) {
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
  if (selectedDevice.value === 'all') return DEVICE_CONFIGS
  return DEVICE_CONFIGS.filter((d) => d.id === selectedDevice.value)
})

// ─── 图表数据 ─────────────────────────────────────────────────
const isLoading = ref(false)
let fetchAbortController: AbortController | null = null
const chartDataMap = ref<Map<string, ChartSeriesData>>(new Map())

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

// ─── 数据获取 ─────────────────────────────────────────────────
const fetchAllData = async () => {
  // 取消上一次未完成的请求
  fetchAbortController?.abort()
  fetchAbortController = new AbortController()
  const signal = fetchAbortController.signal

  isLoading.value = true
  chartDataMap.value = new Map()

  const { start_time, end_time } = getTimeRange()
  const devs = activeDevices.value

  // 将每个设备的点位按每批 20 拆分，并行请求（共用同一个 signal）
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
      batchQueryHistory({ start_time, end_time, limit_per_series: 500, series: chunk }, signal),
    )
  })

  try {
    const results = await Promise.all(allRequests)
    const newMap = new Map<string, ChartSeriesData>()

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
  } catch (error: any) {
    // 主动取消时静默处理
    if (error?.name === 'CanceledError' || error?.code === 'ERR_CANCELED' || error?.name === 'AbortError') return
    console.error('Failed to load curves data:', error)
  } finally {
    isLoading.value = false
  }
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
      fetchAllData()
    }
  }
}

const handleDateRangeChange = () => {
  if (selectedTimeBtn.value === 'custom' && rangeArray.value.length === 2) {
    fetchAllData()
  }
}

onMounted(() => {
  fetchAllData()
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

        :deep(.el-input__wrapper) {
          background: rgba(255, 255, 255, 0.08);
          border: 0.01rem solid rgba(255, 255, 255, 0.2);
          box-shadow: none;
        }

        :deep(.el-input__inner) {
          color: rgba(255, 255, 255, 0.85);
          font-size: 0.14rem;
        }

        :deep(.el-select__caret) {
          color: rgba(255, 255, 255, 0.6);
        }
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
