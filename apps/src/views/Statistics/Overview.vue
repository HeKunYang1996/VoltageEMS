<template>
  <div class="curves">
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
      <div v-if="historyError" role="alert">Historical data unavailable</div>
      <LoadingBg :loading="isLoading">
      <div class="curves__charts">
        <!-- 概览卡片 -->
        <div class="curves__chart-item">
          <div class="chart__review">
            <div class="chart__review-header">
              <div>
                <div class="chart__review-header-title">Energy consumption</div>
                <div v-if="loadEnergyCoverage" class="chart__review-coverage">{{ loadEnergyCoverage }}</div>
              </div>
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
import { formatNumber } from '@/utils/common'
import { computed, onUnmounted, ref, watch } from 'vue'
import { useEnergyStatistics } from '@/composables/useEnergyStatistics'
import { buildStatisticChartData, statisticRequests, STATISTIC_HISTORY_LIMIT } from '@/utils/energyStatistics'

interface ChartSeries {
  name: string
  data: (number | null)[]
  color: string
}

const defaultTime: [Date, Date] = [new Date(2000, 0, 1, 0, 0, 0), new Date(2000, 0, 1, 23, 59, 59)]
const rangeArray = ref<string[] | null>([])
const selectedTimeBtn = ref<'6h' | '1d' | '1w' | '1m' | 'custom'>('6h')
const timeBtnList = [
  { label: 'Custom', value: 'custom' },
  { label: '6 Hour', value: '6h' },
  { label: '1 Day', value: '1d' },
  { label: '1 Week', value: '1w' },
  { label: '1 Month', value: '1m' },
] as const
const statistics = useEnergyStatistics()
// Range consumption comes only from historical counter differences. Live
// cumulative readings must never overwrite a selected range's consumption.
const totalLoadEnergy = ref<string>('--')
const loadEnergyCoverage = ref('')
const stationInfoList = computed(() => [
  { title: 'PV', icon: PVEnergy, value: statistics.live.value.pvPower, unit: 'kW' },
  { title: 'ESS', icon: ESS, value: statistics.live.value.essPower, unit: 'kW' },
  { title: 'DG', icon: DG, value: statistics.live.value.dgPower, unit: 'kW' },
].map((item) => ({ ...item, value: item.value === null ? '--' : formatNumber(item.value) })))
const isLoading = ref(false)
const historyError = ref(false)
let fetchAbortController: AbortController | null = null
let activeQueryKey: string | undefined
let requestVersion = 0
const xAxisData = ref<string[]>([])
const loadEnergySeriesData = ref<ChartSeries[]>([])
const energySeriesData = ref<ChartSeries[]>([])
const socSeriesData = ref<ChartSeries[]>([])
const powerSeriesData = ref<ChartSeries[]>([])
const energyDistributionData = [
  { name: 'Online', value: 0, color: '#6DD400' },
  { name: 'Offline', value: 100, color: '#4FADF7' },
  { name: 'Alarm', value: 0, color: '#FF4D4F' },
]

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
  const rangeMap = {
    '6h': getRecentHoursRange(6), '1d': getRecentDaysRange(1),
    '1w': getRecentWeekRange(), '1m': getRecentDaysRange(30),
  }
  const range = rangeMap[selectedTimeBtn.value as keyof typeof rangeMap] ?? getRecentHoursRange(6)
  return { start_time: range.start!, end_time: range.end! }
}
const formatLabel = (timestamp: string) => dayjs(timestamp).format(
  selectedTimeBtn.value === '6h' || selectedTimeBtn.value === '1d' ? 'HH:mm' : 'MM-DD HH:mm',
)

const fetchAllChartData = async () => {
  if (statistics.loading.value) {
    requestVersion++
    fetchAbortController?.abort()
    activeQueryKey = undefined
    isLoading.value = true
    historyError.value = false
    return
  }
  const metrics = statistics.metrics.value
  const series = statisticRequests(metrics)
  const key = JSON.stringify([metrics, selectedTimeBtn.value, rangeArray.value])
  if (activeQueryKey === key) return
  const version = ++requestVersion
  fetchAbortController?.abort()
  fetchAbortController = new AbortController()
  const controller = fetchAbortController
  activeQueryKey = undefined
  historyError.value = false
  totalLoadEnergy.value = '--'
  loadEnergyCoverage.value = ''
  xAxisData.value = []
  loadEnergySeriesData.value = []
  energySeriesData.value = []
  socSeriesData.value = []
  powerSeriesData.value = []
  const range = getTimeRange()
  isLoading.value = series.length > 0 && range !== null
  if (!series.length || !range) return
  activeQueryKey = key
  try {
    const response = await batchQueryHistory({
      ...range, limit_per_series: STATISTIC_HISTORY_LIMIT, series,
    }, controller.signal)
    if (version !== requestVersion) return
    if (!response.success) throw new Error('History query failed')
    const chart = buildStatisticChartData(metrics, response.data?.series ?? [])
    xAxisData.value = chart.timestamps.map(formatLabel)
    totalLoadEnergy.value = chart.loadEnergy === null ? '--' : formatNumber(chart.loadEnergy)
    if (chart.loadEnergyCoverage) {
      const { start, end } = chart.loadEnergyCoverage
      loadEnergyCoverage.value = `Measured ${dayjs(start).format('MM-DD HH:mm')} – ${dayjs(end).format('MM-DD HH:mm')}`
    }
    loadEnergySeriesData.value = [{ name: 'Load', data: chart.values('loadEnergy'), color: '#4FADF7' }]
    energySeriesData.value = [
      { name: 'Pv', data: chart.values('pvEnergy'), color: '#69CBFF' },
      { name: 'DG', data: chart.values('dgEnergy'), color: '#1D86FF' },
    ]
    socSeriesData.value = [{ name: 'SOC', data: chart.values('soc'), color: '#6DD400' }]
    powerSeriesData.value = [
      { name: 'Pv', data: chart.values('pvPower'), color: '#69CBFF' },
      { name: 'ESS', data: chart.values('essPower'), color: '#4FADF7' },
      { name: 'DG', data: chart.values('dgPower'), color: '#F6C85F' },
    ]
  } catch (error: unknown) {
    if (version !== requestVersion || controller.signal.aborted) return
    historyError.value = true
    console.error('Failed to load overview chart data:', error)
  } finally {
    if (version === requestVersion) {
      isLoading.value = false
      activeQueryKey = undefined
    }
  }
}

// Fetch whatever equipment exists; PV/ESS/Diesel are independently optional.
watch([statistics.loading, statistics.metrics], fetchAllChartData, { immediate: true })
onUnmounted(() => { requestVersion++; fetchAbortController?.abort() })
const handleTimeBtnClick = (event: MouseEvent) => {
  const btn = (event.target as HTMLElement).closest('.curves__toolbar-time-btn') as HTMLElement | null
  if (!btn?.dataset.value) return
  if (btn.dataset.value === selectedTimeBtn.value) return
  selectedTimeBtn.value = btn.dataset.value as typeof selectedTimeBtn.value
  rangeArray.value = []
  fetchAllChartData()
}
const handleDateRangeChange = () => {
  if (selectedTimeBtn.value === 'custom') fetchAllChartData()
}

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
    height: calc(100% - 0.52rem);
    min-height: 0;
  }

  .curves__charts {
    height: 100%;
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

          .chart__review-coverage {
            margin-top: 0.04rem;
            font-size: 0.11rem;
            color: rgba(255, 255, 255, 0.6);
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
