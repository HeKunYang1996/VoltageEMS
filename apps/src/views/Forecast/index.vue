<template>
  <div class="forecast-view">
    <div class="forecast__left">
      <!-- 摘要卡片：4 张 -->
      <div class="forecast__summary">
        <div class="summary-cards">
          <div class="summary-card">
            <div class="summary-card__icon pv-icon"><el-icon><Sunny /></el-icon></div>
            <div class="summary-card__content">
              <div class="summary-card__label">Total PV Generation</div>
              <div class="summary-card__value">
                {{ displayFixed(summary.total_pv_generation) }}<span class="summary-card__unit">kWh</span>
              </div>
              <div
                v-if="hasValue(summary.pv_generation_vs_yesterday_pct)"
                class="summary-card__trend"
                :class="trendUp(summary.pv_generation_vs_yesterday_pct)"
              >
                vs Yesterday <span>{{ displayPct(summary.pv_generation_vs_yesterday_pct) }}</span>
              </div>
            </div>
          </div>
          <div class="summary-card">
            <div class="summary-card__icon load-icon"><el-icon><TrendCharts /></el-icon></div>
            <div class="summary-card__content">
              <div class="summary-card__label">Total Load</div>
              <div class="summary-card__value">
                {{ displayFixed(summary.total_load) }}<span class="summary-card__unit">kWh</span>
              </div>
              <div
                v-if="hasValue(summary.load_vs_yesterday_pct)"
                class="summary-card__trend"
                :class="trendUp(summary.load_vs_yesterday_pct)"
              >
                vs Yesterday <span>{{ displayPct(summary.load_vs_yesterday_pct) }}</span>
              </div>
            </div>
          </div>
          <div class="summary-card">
            <div class="summary-card__icon self-icon"><el-icon><DataAnalysis /></el-icon></div>
            <div class="summary-card__content">
              <div class="summary-card__label">Self-consumption Rate</div>
              <div class="summary-card__value">
                {{ displayFixed(summary.self_consumption_ratio) }}<span class="summary-card__unit">%</span>
              </div>
              <div
                v-if="hasValue(summary.self_consumption_vs_yesterday_pct)"
                class="summary-card__trend"
                :class="trendUp(summary.self_consumption_vs_yesterday_pct)"
              >
                vs Yesterday <span>{{ displayPct(summary.self_consumption_vs_yesterday_pct) }}</span>
              </div>
            </div>
          </div>
          <div class="summary-card">
            <div class="summary-card__icon net-icon"><el-icon><DataAnalysis /></el-icon></div>
            <div class="summary-card__content">
              <div class="summary-card__label">Net Power</div>
              <div class="summary-card__value">
                {{ displayFixed(summary.net_power) }}<span class="summary-card__unit">kW</span>
              </div>
              <div
                v-if="hasValue(summary.net_power_vs_yesterday_pct)"
                class="summary-card__trend"
                :class="trendUp(summary.net_power_vs_yesterday_pct)"
              >
                vs Yesterday <span>{{ displayPct(summary.net_power_vs_yesterday_pct) }}</span>
              </div>
            </div>
          </div>
          <div class="summary-card">
            <div class="summary-card__icon weather-icon"><el-icon><Cloudy /></el-icon></div>
            <div class="summary-card__content">
              <div class="summary-card__label">Weather</div>
              <div class="summary-card__value">
                <span style="font-size:0.18rem">{{ displayFixed(summary.weather?.temperature) }}</span><span class="summary-card__unit">°C</span>
              </div>
              <div class="summary-card__trend">
                Cloud {{ displayFixed(summary.weather?.cloud_cover) }}% · Humidity {{ displayFixed(summary.weather?.humidity) }}%
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- PV 功率预测图表 -->
      <div class="forecast__chart-section">
        <div class="section-header">
          <span class="section-title">PV Power Forecast</span>
          <span class="section-subtitle">Next 24 Hours</span>
        </div>
        <div class="forecast__chart-wrap">
          <ForecastRangeChart
            :data="pvChartData"
            yUnit="kW"
            :splitLines="splitLines"
            :showToolbox="false"
            title="PV Power Forecast"
          />
        </div>
      </div>

      <!-- 负荷预测图表 -->
      <div class="forecast__chart-section">
        <div class="section-header">
          <span class="section-title">Load Forecast</span>
          <span class="section-subtitle">Next 24 Hours</span>
        </div>
        <div class="forecast__chart-wrap">
          <ForecastRangeChart
            :data="loadChartData"
            yUnit="kW"
            :splitLines="splitLines"
            forecastColor="#ff6900"
            actualColor="#a78bfa"
            bandColor="#a78bfa"
            :showToolbox="false"
            title="Load Forecast"
          />
        </div>
      </div>
    </div>

    <div class="forecast__right">
      <div class="panel-card">
        <div class="panel-card__header">
          <div class="panel-card__title">
            <span class="ai-badge">AI</span>
            <span>AI Suggestions ({{ allSuggestions.length }})</span>
          </div>
        </div>
        <div class="panel-card__body suggestion-scroll-body">
          <div
            v-for="item in allSuggestions"
            :key="item.id"
            class="suggestion-item"
            :class="sugClass(item)"
          >
            <div class="suggestion-item__icon" :class="sugClass(item)">
              <el-icon v-if="item.priority === 'high' || item.suggestion_type === 'warning'"><Lightning /></el-icon>
              <el-icon v-else><Box /></el-icon>
            </div>
            <div class="suggestion-item__content">
              <div class="suggestion-item__title">{{ item.title }}</div>
              <div class="suggestion-item__desc">{{ item.description }}</div>
              <div class="suggestion-item__impact">{{ item.recommended_action }}</div>
            </div>
            <div class="suggestion-item__meta-text">
              <span class="suggestion-priority" :class="item.priority">{{ item.priority }}</span>
            </div>
          </div>
          <div v-if="allSuggestions.length === 0" class="suggestion-empty">No suggestions available</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Sunny, TrendCharts, DataAnalysis, Lightning, Box, Cloudy } from '@element-plus/icons-vue'
import ForecastRangeChart from '@/components/charts/ForecastRangeChart.vue'
import type { RangeChartData } from '@/components/charts/ForecastRangeChart.vue'
import {
  fetchForecastLoad,
  fetchForecastPV,
  fetchForecastSummary,
  fetchForecastSuggestions,
} from '@/api/forecast'
import type {
  ForecastSummary,
  SchedulingSuggestion,
  TimeSeriesPoint,
} from '@/types/forecast'
import dayjs from 'dayjs'

// ─── State ────────────────────────────────────────────────
const summaryLastUpdated = ref('')

const summary = reactive<Partial<ForecastSummary>>({
  weather: null,
})

const allSuggestions = ref<SchedulingSuggestion[]>([])

// ─── Chart data ────────────────────────────────────────────
const splitLines = computed(() => {
  const index = findNowIndex(pvChartData.value.length ? pvChartData.value : loadChartData.value)
  return index >= 0 ? [{ index, label: 'Now' }] : []
})

const pvChartData = ref<RangeChartData[]>([])
const loadChartData = ref<RangeChartData[]>([])

// ─── Data fetching ─────────────────────────────────────────
async function loadAll() {
  try {
    await Promise.all([loadSummary(), loadForecastCurves(), loadSuggestions()])
  } catch (e) {
    console.error('[Forecast] load error:', e)
  }
}

async function loadSummary() {
  try {
    const d = await fetchForecastSummary()
    Object.assign(summary, d)
    if (d.last_updated) summaryLastUpdated.value = dayjs(d.last_updated).format('MM-DD HH:mm')
  } catch (e) {
    console.warn('[Forecast] Summary API failed', e)
  }
}

async function loadSuggestions() {
  try {
    const d = await fetchForecastSuggestions()
    allSuggestions.value = Array.isArray(d) ? d : []
  } catch (e) {
    console.warn('[Forecast] Suggestions API failed', e)
    allSuggestions.value = []
  }
}

async function loadForecastCurves() {
  try {
    const window = getForecastWindow()
    const [pv, load] = await Promise.all([
      fetchForecastPV(window.startTime, window.endTime),
      fetchForecastLoad(window.startTime, window.endTime),
    ])
    pvChartData.value = mapTimeSeries(pv.data, window.now)
    loadChartData.value = mapTimeSeries(load.data, window.now)
  } catch (e) {
    console.warn('[Forecast] Time series API failed', e)
    pvChartData.value = []
    loadChartData.value = []
  }
}

function getForecastWindow() {
  const now = dayjs()
  return {
    now,
    startTime: now.subtract(6, 'hour').format('YYYY-MM-DDTHH:mm:ssZ'),
    endTime: now.add(24, 'hour').format('YYYY-MM-DDTHH:mm:ssZ'),
  }
}

function mapTimeSeries(data: TimeSeriesPoint[] = [], now = dayjs()): RangeChartData[] {
  return data.map((item) => {
    const pointTime = dayjs(item.ts)
    const isHistory = pointTime.isValid() && pointTime.isBefore(now)
    const forecastValue = item.p50 ?? item.predicted ?? null

    return {
      label: pointTime.isValid() ? pointTime.format('MM-DD HH:mm') : item.ts,
      timestamp: item.ts,
      actual: item.actual,
      p50: isHistory ? null : forecastValue,
      p10: isHistory ? null : item.p10,
      p90: isHistory ? null : item.p90,
      weather: item.weather,
    }
  })
}

function findNowIndex(data: RangeChartData[]): number {
  if (!data.length) return -1
  const current = dayjs()
  let best = -1
  let bestDiff = Number.POSITIVE_INFINITY
  data.forEach((point, index) => {
    const candidate = dayjs(point.timestamp ?? point.label)
    if (!candidate.isValid()) return
    const diff = Math.abs(candidate.diff(current, 'minute'))
    if (diff < bestDiff) {
      bestDiff = diff
      best = index
    }
  })
  return bestDiff <= 45 ? best : -1
}

// ─── Display helpers ──────────────────────────────────────
function displayFixed(v: number | null | undefined): string {
  if (v === null || v === undefined) return '-'
  return v.toFixed(1)
}

function displayPct(v: number | null | undefined): string {
  if (v === null || v === undefined) return '-'
  const prefix = v >= 0 ? '▲' : '▼'
  return `${prefix} ${Math.abs(v).toFixed(1)}%`
}

function trendUp(v: number | null | undefined): string {
  if (v === null || v === undefined) return ''
  return v >= 0 ? 'up' : 'down'
}

function hasValue(v: number | null | undefined): boolean {
  return v !== null && v !== undefined
}

function sugClass(s: SchedulingSuggestion): string {
  if (s.priority === 'high' || s.suggestion_type === 'warning') return 'warning'
  return 'info'
}

onMounted(loadAll)
</script>

<style lang="scss" scoped>
.forecast-view {
  display: flex;
  height: 100%;
  gap: 0.12rem;
  overflow: hidden;

  // ── 左侧（压缩） ──
  .forecast__left {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    overflow: hidden;
  }

  // ── 右侧（变宽） ──
  .forecast__right {
    width: 3.6rem;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    &::-webkit-scrollbar { width: 0.03rem; }
    &::-webkit-scrollbar-thumb { background: var(--vt-scrollbar-thumb); border-radius: 0.03rem; }
  }

  // ── 摘要卡片 ──
  .forecast__summary {
    flex-shrink: 0;
    .summary-cards { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 0.08rem;
      .summary-card { display: flex; align-items: center; gap: 0.08rem;
        background: var(--vt-bg-glass); border: 1px solid var(--vt-border-color-soft);
        border-radius: var(--vt-radius-md); padding: 0.13rem 0.12rem; min-height: 0.84rem;
        .summary-card__icon { width: 0.32rem; height: 0.32rem; border-radius: 50%; display: flex;
          align-items: center; justify-content: center; flex-shrink: 0; font-size: 0.16rem;
          &.pv-icon { background: rgba(255,165,0,0.15); color: #ffa500; }
          &.load-icon { background: rgba(105,203,255,0.15); color: var(--vt-color-chart-pv); }
          &.self-icon { background: rgba(82,196,26,0.15); color: var(--vt-color-success); }
          &.net-icon { background: rgba(255,105,0,0.15); color: var(--vt-color-primary); }
          &.weather-icon { background: rgba(167,139,250,0.15); color: #a78bfa; }
        }
        .summary-card__content { display: flex; flex-direction: column; gap: 0.02rem; min-width: 0;
          .summary-card__label { font-size: 0.11rem; color: var(--vt-text-secondary);
            white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
          .summary-card__value { font-size: var(--vt-font-size-lg); font-weight: var(--vt-font-weight-bold);
            color: var(--vt-text-primary); line-height: 1.2;
            .summary-card__unit { font-size: 0.1rem; color: var(--vt-text-secondary);
              margin-left: 0.03rem; font-weight: var(--vt-font-weight-normal); }
          }
          .summary-card__trend { font-size: 0.1rem; color: var(--vt-text-secondary);
            span { font-weight: var(--vt-font-weight-semibold); }
            &.up span { color: var(--vt-color-success); }
            &.down span { color: var(--vt-color-danger); }
          }
        }
      }
    }
  }

  // ── 图表 ──
  .forecast__chart-section {
    flex: 1;
    min-height: 0;
    background: var(--vt-bg-glass); border: 1px solid var(--vt-border-color);
    border-radius: var(--vt-radius-md); padding: 0.08rem 0.1rem;
    display: flex; flex-direction: column;

    .section-header { display: flex; align-items: center; gap: 0.08rem; margin-bottom: 0.04rem;
      flex-shrink: 0; }
    .section-title { font-size: var(--vt-font-size-sm); font-weight: var(--vt-font-weight-semibold);
      color: var(--vt-text-primary); }
    .section-subtitle { font-size: 0.1rem; color: var(--vt-text-secondary); }

    .forecast__chart-wrap { flex: 1; min-height: 0; position: relative; }
  }

  // ── 面板卡片 ──
  .panel-card {
    background: var(--vt-bg-glass); border: 1px solid var(--vt-border-color);
    border-radius: var(--vt-radius-md); padding: 0.1rem;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    .panel-card__header { display: flex; align-items: center; justify-content: space-between;
      margin-bottom: 0.08rem;
      flex-shrink: 0;
      .panel-card__title { display: flex; align-items: center; gap: 0.05rem;
        font-size: var(--vt-font-size-sm); font-weight: var(--vt-font-weight-semibold);
        color: var(--vt-text-primary);
        .ai-badge { background: var(--vt-color-primary); color: var(--vt-text-primary); font-size: 0.09rem;
          font-weight: var(--vt-font-weight-bold); padding: 0.01rem 0.04rem;
          border-radius: 0.02rem; line-height: 1.3; }
      }
    }
    .panel-card__body { display: flex; flex-direction: column; gap: 0.06rem; min-height: 0; }
  }

  .suggestion-scroll-body {
    flex: 1; overflow-y: auto;
    &::-webkit-scrollbar { width: 0.03rem; }
    &::-webkit-scrollbar-thumb { background: var(--vt-scrollbar-thumb); border-radius: 0.03rem; }
  }

  .suggestion-item {
    display: flex; align-items: flex-start; gap: 0.06rem; padding: 0.08rem;
    border-radius: var(--vt-radius-sm); background: var(--vt-bg-glass-strong);
    border-left: 0.02rem solid transparent;
    &.warning { border-left-color: var(--vt-color-level-warning); }
    &.info { border-left-color: var(--vt-color-warning); }
    .suggestion-item__icon { width: 0.24rem; height: 0.24rem; border-radius: 50%; display: flex;
      align-items: center; justify-content: center; font-size: 0.12rem; flex-shrink: 0; margin-top: 0.01rem;
      &.warning { background: rgba(255,110,8,0.15); color: var(--vt-color-level-warning); }
      &.info { background: rgba(250,173,20,0.15); color: var(--vt-color-warning); }
    }
    .suggestion-item__content { flex: 1; min-width: 0;
      .suggestion-item__title { font-size: 0.11rem; font-weight: var(--vt-font-weight-semibold);
        color: var(--vt-text-primary); line-height: 1.3; margin-bottom: 0.02rem; }
      .suggestion-item__desc { font-size: 0.1rem; color: var(--vt-text-secondary); line-height: 1.3; }
      .suggestion-item__impact { font-size: 0.1rem; color: var(--vt-text-secondary); margin-top: 0.02rem; }
    }
    .suggestion-item__meta-text { flex-shrink: 0;
      .suggestion-priority { font-size: 0.09rem; padding: 0.01rem 0.05rem; border-radius: 0.02rem;
        &.high { background: rgba(255,110,8,0.2); color: var(--vt-color-level-warning); }
        &.medium { background: rgba(250,173,20,0.2); color: var(--vt-color-warning); }
        &.low { background: rgba(82,196,26,0.15); color: var(--vt-color-success); }
      }
    }
  }

  .suggestion-empty { text-align: center; color: var(--vt-text-secondary);
    font-size: var(--vt-font-size-xs); padding: 0.15rem 0; }
}
</style>
