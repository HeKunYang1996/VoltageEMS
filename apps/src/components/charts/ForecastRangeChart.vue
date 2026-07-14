<template>
  <div class="forecast-range-chart">
    <div class="forecast-range-chart-container" ref="chartRef"></div>
    <div v-if="showToolbox" class="forecast-range-chart-toolbox">
      <div v-if="showFullScreen" class="toolbox-item" @click="handleFullScreen">
        <el-icon><ZoomIn /></el-icon>
      </div>
      <div v-if="showDownload" class="toolbox-item" @click="handleExport">
        <el-icon><Download /></el-icon>
      </div>
    </div>
    <FullSceenDialog
      ref="fullScreenDialogRef"
      :title="title || 'Forecast Chart'"
      fullscreen
      :append-to-body="true"
      :modal-append-to-body="true"
      :close-on-click-modal="false"
    >
      <template #dialog-body>
        <div class="forecast-range-chart-full-screen">
          <div class="forecast-range-chart-full-screen__container" ref="fullScreenChartRef"></div>
        </div>
      </template>
    </FullSceenDialog>
  </div>
</template>

<script setup lang="ts">
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import {
  TooltipComponent,
  GridComponent,
  LegendComponent,
  DataZoomComponent,
  ToolboxComponent,
  MarkLineComponent,
} from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { useGlobalStore } from '@/stores/global'
import { pxToResponsive as px } from '@/utils/responsive'
import FullSceenDialog from '@/components/dialog/fullSceenDialog.vue'
import { ZoomIn, Download } from '@element-plus/icons-vue'
import * as XLSX from 'xlsx'
import type { WeatherData } from '@/types/forecast'

echarts.use([
  LineChart,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  CanvasRenderer,
  DataZoomComponent,
  ToolboxComponent,
  MarkLineComponent,
])

export interface RangeChartData {
  label: string
  timestamp?: string
  actual: number | null
  p50: number | null
  p10: number | null
  p90: number | null
  weather?: WeatherData | null
}

export interface SplitLineConfig {
  index: number
  label?: string
}

const props = withDefaults(
  defineProps<{
    data: RangeChartData[]
    yUnit?: string
    splitLines?: SplitLineConfig[]
    forecastColor?: string
    actualColor?: string
    bandColor?: string
    showToolbox?: boolean
    showFullScreen?: boolean
    showDownload?: boolean
    title?: string
  }>(),
  {
    yUnit: 'kW',
    forecastColor: '#ff6900',
    actualColor: '#69cbff',
    bandColor: '#69cbff',
    showToolbox: true,
    showFullScreen: true,
    showDownload: true,
    splitLines: () => [],
  },
)

const fullScreenDialogRef = ref()
const fullScreenChartRef = ref<HTMLDivElement | null>(null)
const chartRef = ref<HTMLDivElement | null>(null)
const globalStore = useGlobalStore()
let chartInstance: echarts.ECharts | null = null
let fullScreenChartInstance: echarts.ECharts | null = null

watch(
  () => globalStore.isCollapse,
  () => {
    nextTick(() => {
      setTimeout(() => { chartInstance?.dispose(); initChart() }, 300)
    })
  },
)

function alpha(c: string, a: number): string {
  const h = c.replace('#', '')
  const r = parseInt(h.substring(0, 2), 16)
  const g = parseInt(h.substring(2, 4), 16)
  const b = parseInt(h.substring(4, 6), 16)
  return `rgba(${r},${g},${b},${a})`
}

function buildOption(isFull: boolean) {
  const d = props.data
  const labels = d.map(x => x.label)
  const actuals = d.map(x => x.actual)
  const p50s = d.map(x => x.p50)
  const p10s = d.map(x => x.p10)
  const p90s = d.map(x => x.p90)
  const rangeBand = d.map(x => (x.p10 !== null && x.p90 !== null ? x.p90 - x.p10 : null))

  const hasP10 = p10s.some(v => v !== null)
  const hasP90 = p90s.some(v => v !== null)
  const hasActual = actuals.some(v => v !== null)
  const hasP50 = p50s.some(v => v !== null)
  const hasBand = d.some(x => x.p10 !== null && x.p90 !== null)

  const R = isFull ? px : (v: number) => v

  const grid = {
    left: R(55), right: R(20), top: R(40), bottom: R(25),
  }

  const axisLabel = {
    color: 'rgba(255,255,255,0.6)', fontFamily: 'Arimo',
    fontSize: isFull ? px(16) : px(12),
  }

  const series: any[] = []

  if (hasBand) {
    series.push(
      {
        name: '__P10_BASE__',
        type: 'line',
        data: p10s,
        stack: 'confidence-band',
        smooth: true,
        symbol: 'none',
        lineStyle: { opacity: 0 },
        itemStyle: { opacity: 0 },
        areaStyle: { opacity: 0 },
        silent: true,
        tooltip: { show: false },
        emphasis: { disabled: true },
        z: 1,
      },
      {
        name: '__P10_P90_BAND__',
        type: 'line',
        data: rangeBand,
        stack: 'confidence-band',
        smooth: true,
        symbol: 'none',
        lineStyle: { opacity: 0 },
        itemStyle: { opacity: 0 },
        areaStyle: { color: alpha(props.bandColor, 0.16) },
        silent: true,
        tooltip: { show: false },
        emphasis: { disabled: true },
        z: 1,
      },
    )
  }

  if (hasP90) {
    series.push({
      name: 'P90',
      type: 'line',
      data: p90s,
      smooth: true,
      symbol: 'none',
      lineStyle: { color: alpha(props.bandColor, 0.86), width: R(1), type: 'dashed' },
      itemStyle: { color: alpha(props.bandColor, 0.86) },
      emphasis: { focus: 'series' },
      z: 2,
    })
  }

  if (hasP10) {
    series.push({
      name: 'P10',
      type: 'line',
      data: p10s,
      smooth: true,
      symbol: 'none',
      lineStyle: { color: alpha(props.bandColor, 0.55), width: R(1), type: 'dashed' },
      itemStyle: { color: alpha(props.bandColor, 0.55) },
      emphasis: { focus: 'series' },
      z: 2,
    })
  }

  // P50 forecast line
  if (hasP50) {
    series.push({
      name: 'Forecast (P50)',
      type: 'line',
      data: p50s,
      smooth: true,
      symbol: 'none',
      lineStyle: { color: props.forecastColor, width: R(2), type: 'dashed' },
      itemStyle: { color: props.forecastColor },
      emphasis: { focus: 'series' },
      z: 3,
    })
  }

  // Actual line
  if (hasActual) {
    series.push({
      name: 'Actual',
      type: 'line',
      data: actuals,
      smooth: true,
      symbol: 'circle',
      symbolSize: R(4),
      lineStyle: { color: props.actualColor, width: R(2) },
      itemStyle: { color: props.actualColor },
      emphasis: { focus: 'series' },
      z: 4,
    })
  }

  // MarkLine on first non-silent series
  if (props.splitLines.length > 0) {
    const first = series.find(s => !s.silent)
    if (first) {
      first.markLine = {
        silent: true,
        symbol: 'none',
        lineStyle: { color: 'rgba(255,255,255,0.8)', type: 'dashed', width: R(1) },
        label: {
          show: true,
          position: 'start',
          color: 'rgba(255,255,255,0.85)',
          fontSize: R(12),
          backgroundColor: 'rgba(63,79,117,0.9)',
          padding: [R(4), R(8)],
          borderRadius: R(4),
        },
        data: props.splitLines.map(sl => ({
          xAxis: sl.index,
          label: { formatter: sl.label ?? '' },
        })),
      }
    }
  }

  // Tooltip
  const tooltip: any = {
    trigger: 'axis',
    confine: true,
    backgroundColor: '#3f4f75',
    borderColor: 'rgba(255,255,255,0.12)',
    borderWidth: isFull ? px(2) : 1,
    padding: isFull ? [px(30), px(40), px(30), px(40)] : [px(10), px(16), px(10), px(16)],
    extraCssText: `border-radius:${R(8)}px;box-shadow:0 ${R(4)}px ${R(16)}px 0 rgba(0,0,0,0.12);`,
    axisPointer: { type: 'cross', crossStyle: { color: 'rgba(255,255,255,0.2)' } },
    formatter: (params: any) => {
      const idx = params[0]?.dataIndex ?? -1
      if (idx < 0 || idx >= d.length) return ''
      const pt = d[idx]
      const fs1 = R(14), fs2 = R(12), fs3 = R(11)

      let html = `<div style="max-width:2.4rem;font-family:Arimo;">
        <div style="color:rgba(255,255,255,0.85);font-size:${fs1}px;font-weight:600;margin-bottom:${R(4)}px;">${pt.label}</div>`
      if (pt.actual !== null) {
        html += `<div style="display:flex;justify-content:space-between;font-size:${fs2}px;color:#69cbff;margin-bottom:${R(2)}px;">
          <span>Actual</span><span style="font-weight:600;">${pt.actual.toFixed(2)} ${props.yUnit}</span>
        </div>`
      }
      if (pt.p50 !== null) {
        html += `<div style="display:flex;justify-content:space-between;font-size:${fs2}px;color:${props.forecastColor};margin-bottom:${R(2)}px;">
          <span>Forecast (P50)</span><span style="font-weight:600;">${pt.p50.toFixed(2)} ${props.yUnit}</span>
        </div>`
      }
      if (pt.p10 !== null && pt.p90 !== null) {
        html += `<div style="display:flex;justify-content:space-between;font-size:${fs3}px;color:rgba(255,255,255,0.5);margin-bottom:${R(2)}px;">
          <span>P10 - P90</span><span>${pt.p10.toFixed(2)} ~ ${pt.p90.toFixed(2)} ${props.yUnit}</span>
        </div>`
      }
      if (pt.weather) {
        const w = pt.weather
        html += `<div style="border-top:1px solid rgba(255,255,255,0.15);margin:${R(4)}px 0 ${R(2)}px;"></div>`
        if (w.temperature !== null) html += `<div style="font-size:${fs3}px;color:rgba(255,255,255,0.6);">🌡️ Temp: ${w.temperature.toFixed(1)}°C</div>`
        if (w.humidity !== null) html += `<div style="font-size:${fs3}px;color:rgba(255,255,255,0.6);">💧 Humidity: ${w.humidity.toFixed(1)}%</div>`
        if (w.ghi !== null) html += `<div style="font-size:${fs3}px;color:rgba(255,255,255,0.6);">☀️ GHI: ${w.ghi.toFixed(1)} W/m²</div>`
        if (w.cloud_cover !== null) html += `<div style="font-size:${fs3}px;color:rgba(255,255,255,0.6);">☁️ Cloud: ${w.cloud_cover.toFixed(1)}%</div>`
        if (w.wind_speed !== null) html += `<div style="font-size:${fs3}px;color:rgba(255,255,255,0.6);">💨 Wind: ${w.wind_speed.toFixed(1)} m/s</div>`
        if (w.weather_text) html += `<div style="font-size:${fs3}px;color:rgba(255,255,255,0.6);">${w.weather_text}</div>`
      }
      html += '</div>'
      return html
    },
  }

  const legendData = [
    ...(hasActual ? [{ name: 'Actual', icon: 'circle' }] : []),
    ...(hasP50 ? [{ name: 'Forecast (P50)', icon: 'circle' }] : []),
    ...(hasP10 ? [{ name: 'P10', icon: 'circle' }] : []),
    ...(hasP90 ? [{ name: 'P90', icon: 'circle' }] : []),
  ]

  return {
    tooltip,
    legend: {
      icon: 'circle',
      show: true,
      orient: 'horizontal',
      right: 0,
      top: px(10),
      itemWidth: px(12),
      itemHeight: px(12),
      itemGap: px(25),
      textStyle: { color: 'rgba(255,255,255,0.6)', fontSize: px(12) },
      data: legendData,
    },
    grid,
    xAxis: {
      type: 'category', data: labels,
      axisTick: { alignWithLabel: true, lineStyle: { color: '#fff' } },
      axisLine: { show: false }, axisLabel, splitLine: { show: false },
    },
    yAxis: {
      type: 'value', name: props.yUnit, nameTextStyle: axisLabel,
      axisLine: { show: false }, axisTick: { show: false }, axisLabel,
      splitLine: { show: true, lineStyle: { color: '#fff', type: 'dashed', opacity: 0.2 } },
    },
    dataZoom: [{ type: 'inside', show: true }],
    series,
  }
}

function initChart() {
  if (!chartRef.value) return
  chartInstance?.dispose()
  chartInstance = echarts.init(chartRef.value)
  chartInstance.setOption(buildOption(false))
}

function initFullScreenChart() {
  if (!fullScreenChartRef.value) return
  fullScreenChartInstance?.dispose()
  fullScreenChartInstance = echarts.init(fullScreenChartRef.value)
  fullScreenChartInstance.setOption(buildOption(true))
}

const handleFullScreen = () => {
  fullScreenDialogRef.value.dialogVisible = true
  nextTick(() => setTimeout(initFullScreenChart, 100))
}

const handleExport = () => {
  const headers = ['time', 'actual', 'forecast_p50', 'p10', 'p90']
  const rows = props.data.map(d => [d.label, d.actual ?? '', d.p50 ?? '', d.p10 ?? '', d.p90 ?? ''])
  const wb = XLSX.utils.book_new()
  const ws = XLSX.utils.aoa_to_sheet([headers, ...rows])
  XLSX.utils.book_append_sheet(wb, ws, 'forecast')
  XLSX.writeFile(wb, `forecast_${new Date().toISOString().slice(0, 10)}.xlsx`)
}

watch(() => props.data, () => nextTick(initChart), { deep: true })

onMounted(() => {
  initChart()
  window.addEventListener('resize', () => chartInstance?.resize())
})
onBeforeUnmount(() => {
  chartInstance?.dispose()
  fullScreenChartInstance?.dispose()
})
</script>

<style scoped lang="scss">
.forecast-range-chart {
  width: 100%; height: 100%; position: relative;
  .forecast-range-chart-container { width: 100%; height: 100%; }
  .forecast-range-chart-toolbox {
    position: absolute; top: -0.2rem; right: 0;
    display: flex; gap: 0.2rem;
    .toolbox-item { width: 0.14rem; height: 0.14rem; cursor: pointer; color: rgba(255,255,255,0.6); }
  }
}
.forecast-range-chart-full-screen {
  width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;
  background: #212c49;
  &__container { width: 100%; height: calc(100vh - 1.1rem); }
}
</style>
