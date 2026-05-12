<template>
  <div class="voltage-class curves">
    <div class="curves__content">
      <!-- 工具栏 -->
      <div class="curves__toolbar">
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

      <!-- 图表区域 -->
      <div class="curves__charts-outer">
      <LoadingBg :loading="isLoading">
      <div class="curves__charts">
        <!-- 概览卡片 -->
        <div class="curves__chart-item">
          <div class="chart__review">
            <div class="chart__review-header">
              <div class="chart__review-header-title">Energy consumption</div>
              <div class="chart__review-header-value">
                {{ totalLoadEnergy }}&nbsp;<span class="chart__review-header-unit">kWh</span>
              </div>
            </div>
            <div class="chart__review-content">
              <div class="chart__review-content-list">
                <div
                  v-for="item in stationInfoList"
                  :key="item.title"
                  class="chart__review-content-item"
                >
                  <EnergyCard
                    :title="item.title"
                    :icon="item.icon"
                    :value="item.value"
                    :unit="item.unit"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Load Energy 柱状图 -->
        <div class="curves__chart-item">
          <ModuleCard title="Load Energy">
            <StackedBarChart
              :xAxiosOption="{ xAxiosData: xAxisData }"
              :yAxiosOption="{ yUnit: 'kWh' }"
              :series="loadEnergySeriesData"
            />
          </ModuleCard>
        </div>

        <!-- Energy 柱状图 (PV + Diesel) -->
        <div class="curves__chart-item">
          <ModuleCard title="Energy">
            <StackedBarChart
              :xAxiosOption="{ xAxiosData: xAxisData }"
              :yAxiosOption="{ yUnit: 'kWh' }"
              :series="energySeriesData"
            />
          </ModuleCard>
        </div>

        <!-- 饼图卡片（保持原样，暂不接入真实数据） -->
        <div class="curves__chart-item">
          <ModuleCard title="Running Statistics">
            <DoughnutChart :series="energyDistributionData" />
          </ModuleCard>
        </div>

        <!-- SOC 折线图 -->
        <div class="curves__chart-item">
          <ModuleCard title="SOC Curve">
            <lineChart
              :xAxiosOption="{ xAxiosData: xAxisData }"
              :yAxiosOption="{ yUnit: '%' }"
              :series="socSeriesData"
            />
          </ModuleCard>
        </div>

        <!-- Power 折线图 (PV + ESS + DG) -->
        <div class="curves__chart-item">
          <ModuleCard title="Power Curve">
            <lineChart
              :xAxiosOption="{ xAxiosData: xAxisData }"
              :yAxiosOption="{ yUnit: 'kW' }"
              :series="powerSeriesData"
            />
          </ModuleCard>
        </div>
      </div>
      </LoadingBg>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import PVEnergy from '@/assets/icons/icon-pv-energy.svg'
import ESS from '@/assets/icons/icon-ess-energy.svg'
import DG from '@/assets/icons/DGEnergy.svg'
import { batchQueryHistory } from '@/api/Statistic/overview'
import dayjs from 'dayjs'
import { getRecentHoursRange, getRecentDaysRange, getRecentWeekRange } from '@/utils/date.ts'
import type { BatchQueryResponse } from '@/types/Statistics/OverView'
import useWebSocket from '@/composables/useWebSocket'
import { formatNumber } from '@/utils/common'

interface ChartSeries {
  name: string
  data: number[]
  color: string
}

// 时间选择
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

// 概览卡片数据
const totalLoadEnergy = ref(0)
const stationInfoList = reactive([
  { title: 'PV', icon: PVEnergy, value: '--', unit: 'kW' },
  { title: 'ESS', icon: ESS, value: '--', unit: 'kW' },
  { title: 'DG', icon: DG, value: '--', unit: 'kW' },
])

// 加载状态 & 请求取消
const isLoading = ref(false)
let fetchAbortController: AbortController | null = null

// 图表公共 X 轴
const xAxisData = ref<string[]>([])

// Load Energy 柱状图 series
const loadEnergySeriesData = ref<ChartSeries[]>([
  { name: 'Load', data: [], color: '#4FADF7' },
])

// Energy 柱状图 series (PV + DG)
const energySeriesData = ref<ChartSeries[]>([
  { name: 'Pv', data: [], color: '#69CBFF' },
  { name: 'DG', data: [], color: '#1D86FF' },
])

// SOC 折线图 series
const socSeriesData = ref<ChartSeries[]>([
  { name: 'SOC', data: [], color: '#6DD400' },
])

// Power 折线图 series (PV + ESS + DG)
const powerSeriesData = ref<ChartSeries[]>([
  { name: 'Pv', data: [], color: '#69CBFF' },
  { name: 'ESS', data: [], color: '#4FADF7' },
  { name: 'DG', data: [], color: '#F6C85F' },
])

// 饼图：设备运行状态（Online / Offline / Alarm）
const energyDistributionData = [
  { name: 'Online', value: 0, color: '#6DD400' },
  { name: 'Offline', value: 100, color: '#4FADF7' },
  { name: 'Alarm', value: 0, color: '#FF4D4F' },
]

// WebSocket 订阅 inst 通道 1(ESS)/2(DG)/4(PV)/9(Load)，获取实时功率和负荷电能
const applyChannelValues = (channelId: number, values: Record<string, number>) => {
  switch (channelId) {
    case 4: // PV: pt7 = 功率
      if (values['7'] !== undefined) stationInfoList[0].value = formatNumber(values['7'])
      break
    case 1: // ESS: pt9 = 功率
      if (values['9'] !== undefined) stationInfoList[1].value = formatNumber(values['9'])
      break
    case 2: // DG: pt1 = 功率
      if (values['1'] !== undefined) stationInfoList[2].value = formatNumber(values['1'])
      break
    case 9: // Load: pt2 = 电能（Energy consumption）
      if (values['2'] !== undefined) totalLoadEnergy.value = Math.round(Number(values['2']))
      break
  }
}

useWebSocket(
  { source: 'inst', channels: [1, 2, 4, 9], dataTypes: ['M'], interval: 2000 },
  {
    onDataUpdate: (data) => {
      applyChannelValues(data.channel_id, data.values)
    },
    onBatchDataUpdate: (data: any) => {
      for (const item of data?.updates ?? []) {
        applyChannelValues(item.channel_id, item.values)
      }
    },
  },
)

// 获取当前时间范围（ISO 格式）
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
  const range = rangeMap[selectedTimeBtn.value] || getRecentHoursRange(6)
  return {
    start_time: range.start!,
    end_time: range.end!,
  }
}

// 格式化时间戳为 x 轴标签
const formatLabel = (ts: string): string => {
  const btn = selectedTimeBtn.value
  if (btn === '6h' || btn === '1d') return dayjs(ts).format('HH:mm')
  return dayjs(ts).format('MM-DD HH:mm')
}

const formatValue = (v: number | null | undefined): number =>
  Number(Number(v ?? 0).toFixed(3))

// 批量查询并更新所有图表
const fetchAllChartData = async () => {
  // 取消上一次未完成的请求
  fetchAbortController?.abort()
  fetchAbortController = new AbortController()
  const signal = fetchAbortController.signal

  isLoading.value = true
  const { start_time, end_time } = getTimeRange()

  try {
    const res = await batchQueryHistory(
      {
        start_time,
        end_time,
        limit_per_series: 500,
        series: [
          { redis_key: 'inst:6:M', point_id: '2' },  // 0: Load Energy
          { redis_key: 'inst:4:M', point_id: '15' }, // 1: Energy PV
          { redis_key: 'inst:2:M', point_id: '2' },  // 2: Energy DG
          { redis_key: 'inst:1:M', point_id: '7' },  // 3: SOC
          { redis_key: 'inst:4:M', point_id: '7' },  // 4: Power PV
          { redis_key: 'inst:1:M', point_id: '5' },  // 5: Power ESS
          { redis_key: 'inst:2:M', point_id: '1' },  // 6: Power DG
        ],
      },
      signal,
    )

    const responses: BatchQueryResponse[] = res.data?.series || []

    const findSeries = (redisKey: string, pointId: string) =>
      responses.find((r) => r.redis_key === redisKey && r.point_id === pointId)

    // 合并所有时间戳并排序
    const allTimestamps = new Set<string>()
    responses.forEach((r) => (r.data || []).forEach((p) => allTimestamps.add(p.timestamp)))
    const sortedTimestamps = [...allTimestamps].sort(
      (a, b) => new Date(a).getTime() - new Date(b).getTime(),
    )

    xAxisData.value = sortedTimestamps.map(formatLabel)

    const makeValueArray = (s: BatchQueryResponse | undefined): number[] => {
      if (!s) return sortedTimestamps.map(() => 0)
      const map = new Map((s.data || []).map((p) => [p.timestamp, p.value]))
      return sortedTimestamps.map((ts) => formatValue(map.get(ts)))
    }

    const loadEnergyData = makeValueArray(findSeries('inst:6:M', '2'))
    const pvEnergyData = makeValueArray(findSeries('inst:4:M', '15'))
    const dieselEnergyData = makeValueArray(findSeries('inst:2:M', '2'))
    const socData = makeValueArray(findSeries('inst:1:M', '7'))
    const pvPowerData = makeValueArray(findSeries('inst:4:M', '7'))
    const essPowerData = makeValueArray(findSeries('inst:1:M', '5'))
    const dieselPowerData = makeValueArray(findSeries('inst:2:M', '1'))

    // 更新总负荷电能
    totalLoadEnergy.value = Math.round(loadEnergyData.reduce((a, b) => a + b, 0))

    // 更新 Load Energy 图表
    loadEnergySeriesData.value = [{ name: 'Load', data: loadEnergyData, color: '#4FADF7' }]

    // 更新 Energy 图表
    energySeriesData.value = [
      { name: 'Pv', data: pvEnergyData, color: '#69CBFF' },
      { name: 'DG', data: dieselEnergyData, color: '#1D86FF' },
    ]

    // 更新 SOC 图表
    socSeriesData.value = [{ name: 'SOC', data: socData, color: '#6DD400' }]

    // 更新 Power 图表
    powerSeriesData.value = [
      { name: 'Pv', data: pvPowerData, color: '#69CBFF' },
      { name: 'ESS', data: essPowerData, color: '#4FADF7' },
      { name: 'DG', data: dieselPowerData, color: '#F6C85F' },
    ]
  } catch (error: any) {
    // AbortError / CanceledError 表示请求被主动取消，静默处理
    if (error?.name === 'CanceledError' || error?.code === 'ERR_CANCELED' || error?.name === 'AbortError') return
    console.error('Failed to load overview chart data:', error)
  } finally {
    isLoading.value = false
  }
}

onUnmounted(() => {
  fetchAbortController?.abort()
})

// 时间按钮点击（事件代理）
const handleTimeBtnClick = (event: MouseEvent) => {
  const btn = (event.target as HTMLElement).closest('.curves__toolbar-time-btn') as HTMLElement | null
  if (btn?.dataset.value) {
    selectedTimeBtn.value = btn.dataset.value as '6h' | '1d' | '1w' | '1m' | 'custom'
    rangeArray.value = []
    if (selectedTimeBtn.value !== 'custom') {
      fetchAllChartData()
    }
  }
}

// 自定义时间范围变化
const handleDateRangeChange = () => {
  if (selectedTimeBtn.value === 'custom' && rangeArray.value.length === 2) {
    fetchAllChartData()
  }
}

onMounted(() => {
  fetchAllChartData()
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

  .curves__toolbar {
    padding-bottom: 0.2rem;
    display: flex;
    align-items: center;
    justify-content: flex-end;

    .curves__toolbar-right {
      display: flex;
      align-items: center;
      gap: 0.2rem;

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

  .curves__charts-outer {
    flex: 1;
    min-height: 0;
  }

  .curves__charts {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;

    .curves__chart-item {
      width: calc((100% - 0.4rem) / 3);
      height: calc((100% - 0.2rem) / 2);

      .chart__review {
        width: 100%;
        height: 100%;
        padding: 0.2rem;
        display: flex;
        flex-direction: column;
        background-color: rgba(84, 98, 140, 0.2);
        border: 0.01rem solid;

        border-image: linear-gradient(
            117.01deg,
            rgba(148, 166, 197, 0.3) 3.11%,
            rgba(148, 166, 197, 0) 31.6%,
            rgba(148, 166, 197, 0.103266) 70.79%,
            rgba(148, 166, 197, 0.3) 96.39%
          )
          1;
        backdrop-filter: blur(0.1rem);

        .chart__review-header {
          height: 0.83rem;
          padding: 0 0.2rem;
          background-color: rgba(84, 98, 140, 0.2);
          border: 0.01rem solid;

          border-image: linear-gradient(
              117.01deg,
              rgba(148, 166, 197, 0.3) 3.11%,
              rgba(148, 166, 197, 0) 31.6%,
              rgba(148, 166, 197, 0.103266) 70.79%,
              rgba(148, 166, 197, 0.3) 96.39%
            )
            1;
          backdrop-filter: blur(0.1rem);
          display: flex;
          justify-content: space-between;
          align-items: center;

          .chart__review-header-title {
            font-weight: 700;
            font-size: 0.26rem;
            line-height: 100%;
            letter-spacing: 0%;
          }

          .chart__review-header-value {
            font-weight: 700;
            font-size: 0.22rem;
            line-height: 0.3rem;
            letter-spacing: 0%;

            .chart__review-header-unit {
              font-size: 0.14rem;
              line-height: 0.3rem;
              letter-spacing: 0%;
              color: rgba(255, 255, 255, 0.6);
            }
          }
        }

        .chart__review-content {
          flex: 1;
          padding-top: 0.2rem;

          .chart__review-content-list {
            height: 100%;
            overflow-y: hidden;

            .chart__review-content-item {
              margin-bottom: 0.12rem;
              padding-bottom: 0.13rem;
              border-bottom: 0.01rem dashed rgba(255, 255, 255, 0.2);

              &:last-child {
                border-bottom: none;
                padding-bottom: 0;
                margin-bottom: 0;
              }
            }
          }
        }
      }
    }
  }
}
</style>
