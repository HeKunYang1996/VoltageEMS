<template>
  <div class="forecast-line-chart">
    <div class="forecast-line-chart-container" ref="chartRef"></div>
    <div v-if="showToolbox" class="forecast-line-chart-toolbox">
      <div v-if="showFullScreen" class="forecast-line-chart-toolbox-item" @click="handleFullScreen">
        <el-icon>
          <ZoomIn />
        </el-icon>
      </div>
      <div v-if="showDownload" class="forecast-line-chart-toolbox-item" @click="handleExport">
        <el-icon>
          <Download />
        </el-icon>
      </div>
    </div>
    <FullSceenDialog
      ref="fullScreenDialogRef"
      :title="props.title || 'Forecast Chart Full Screen'"
      fullscreen
      :append-to-body="true"
      :modal-append-to-body="true"
      :close-on-click-modal="false"
    >
      <template #dialog-body>
        <div class="forecast-line-chart-full-screen">
          <div class="forecast-line-chart-full-screen__container" ref="fullScreenChartRef"></div>
        </div>
      </template>
    </FullSceenDialog>
  </div>
</template>

<script setup lang="ts">
import * as echarts from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import {
  TooltipComponent,
  GridComponent,
  LegendComponent,
  DataZoomComponent,
  ToolboxComponent,
  MarkLineComponent,
} from 'echarts/components'
import { SVGRenderer } from 'echarts/renderers'
import { useGlobalStore } from '@/stores/global'
import FullSceenDialog from '@/components/dialog/fullSceenDialog.vue'
import { ZoomIn, Download } from '@element-plus/icons-vue'
import * as XLSX from 'xlsx'
import { pxToResponsive } from '@/utils/responsive'

const fullScreenDialogRef = ref()
const fullScreenChartRef = ref<HTMLDivElement | null>(null)
const globalStore = useGlobalStore()
let fullScreenChartInstance: echarts.ECharts | null = null

echarts.use([
  LineChart,
  BarChart,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  SVGRenderer,
  DataZoomComponent,
  ToolboxComponent,
  MarkLineComponent,
])

/**
 * 分段数据接口
 * 支持多段数据，每段有独立的名称、数据和颜色
 */
export interface SegmentData {
  name: string // 段名称（如 "History", "Forecast"）
  data: number[] // 该段的数据
  color: string // 该段的颜色
  lineType?: 'solid' | 'dashed' | 'dotted' // 线条类型，默认 solid
}

/**
 * 系列数据接口
 * 一个系列可以有多个分段
 */
export interface ForecastSeriesData {
  name: string // 系列名称（如 "PV Power"）
  segments: SegmentData[] // 分段数据数组
}

export interface XAxisOption {
  xAxiosData: string[]
  xUnit?: string
}

export interface YAxisOption {
  yUnit?: string
}

export interface GridConfig {
  left?: number
  right?: number
  top?: number
  bottom?: number
}

/**
 * 分界线配置
 */
export interface SplitLineConfig {
  index: number // 分界点在 X 轴数据中的索引
  label?: string // 分界线标签
}

const props = withDefaults(
  defineProps<{
    xAxiosOption: XAxisOption
    yAxiosOption: YAxisOption
    series: ForecastSeriesData[]
    // 分界线配置数组（支持多个分界线）
    splitLines?: SplitLineConfig[]
    // Grid配置参数
    gridConfig?: GridConfig
    // 全屏模式Grid配置参数
    fullScreenGridConfig?: GridConfig
    // 按钮显示控制
    showToolbox?: boolean
    showFullScreen?: boolean
    showDownload?: boolean
    title?: string
    // 是否显示区域填充颜色
    showAreaStyle?: boolean
  }>(),
  {
    gridConfig: () => ({
      left: 0,
      right: 0,
      top: 45,
      bottom: 10,
    }),
    fullScreenGridConfig: () => ({
      left: 50,
      right: 50,
      top: 90,
      bottom: 50,
    }),
    showToolbox: true,
    showFullScreen: true,
    showDownload: true,
    showAreaStyle: true,
    splitLines: () => [],
  },
)

const chartRef = ref<HTMLDivElement | null>(null)
let chartInstance: echarts.ECharts | null = null

// 监听侧边栏折叠状态变化
watch(
  () => globalStore.isCollapse,
  () => {
    nextTick(() => {
      setTimeout(() => {
        chartInstance?.dispose()
        initChart()
      }, 300)
    })
  },
)

/**
 * 调整颜色透明度
 */
function adjustColorOpacity(color: string, opacity: number): string {
  if (color.startsWith('#')) {
    const hex = color.replace('#', '')
    const r = parseInt(hex.substring(0, 2), 16)
    const g = parseInt(hex.substring(2, 4), 16)
    const b = parseInt(hex.substring(4, 6), 16)
    return `rgba(${r}, ${g}, ${b}, ${opacity})`
  }
  if (color.startsWith('rgb(')) {
    const rgb = color.replace('rgb(', '').replace(')', '').split(',')
    return `rgba(${rgb[0].trim()}, ${rgb[1].trim()}, ${rgb[2].trim()}, ${opacity})`
  }
  if (color.startsWith('rgba(')) {
    const parts = color.replace('rgba(', '').replace(')', '').split(',')
    return `rgba(${parts[0].trim()}, ${parts[1].trim()}, ${parts[2].trim()}, ${opacity})`
  }
  return color
}

/**
 * 自定义 Tooltip formatter
 * 只显示当前悬停点所属分段的数据
 */
function customTooltipFormatter(
  params: any,
  sizeConfig: {
    width: number
    fontSize: number
    itemFontSize: number
    itemLineHeight: number
    dotSize: number
    gap: number
  },
  yUnit: string,
) {
  const { width, fontSize, itemFontSize, itemLineHeight, dotSize, gap } = sizeConfig
  const name = params[0]?.axisValueLabel || params[0]?.name || ''

  // 过滤掉 null 值和背景系列
  const validParams = params.filter(
    (item: any) =>
      item.value !== null && item.value !== undefined && item.seriesName !== 'background',
  )

  if (validParams.length === 0) return ''

  let html = `
    <div style="
      max-width:${width}px;
      display:flex;
      flex-direction:column;
      gap:${gap}px;
    ">
      <div style="
        color:rgba(255,255,255,0.85);
        font-size:${fontSize}px;
        font-family:Arimo;
        font-weight:600;
        width:100%;
        margin-bottom:${gap / 2}px;
      ">${name}</div>
  `
  validParams.forEach((item: any) => {
    html += `
      <div style="
        display:flex;
        align-items:center;
        justify-content:space-between;
        font-size:${itemFontSize}px;
        font-family:Arimo;
        color:rgba(255,255,255,0.85);
        line-height:${itemLineHeight}px;
        margin-bottom:${gap / 4}px;
        gap:${gap * 2}px;
      ">
        <div style="display:flex;align-items:center;gap:${gap / 2}px;">
          <span style="
            display:inline-block;
            width:${dotSize}px;
            height:${dotSize}px;
            border-radius:50%;
            background:${item.color};
            margin-right:${dotSize / 2}px;
          "></span>
          <span>${item.seriesName}</span>
        </div>
        <div style="font-weight:600;">${item.value}${yUnit ? ' ' + yUnit : ''}</div>
      </div>
    `
  })
  html += '</div>'
  return html
}

function getGridConfig(isFullScreen: boolean) {
  return isFullScreen
    ? {
        left: pxToResponsive(props.fullScreenGridConfig.left || 50),
        right: pxToResponsive(props.fullScreenGridConfig.right || 50),
        top: pxToResponsive(props.fullScreenGridConfig.top || 90),
        bottom: pxToResponsive(props.fullScreenGridConfig.bottom || 50),
      }
    : {
        left: pxToResponsive(props.gridConfig.left || 0),
        right: pxToResponsive(props.gridConfig.right || 0),
        top: pxToResponsive(props.gridConfig.top || 45),
        bottom: pxToResponsive(props.gridConfig.bottom || 10),
      }
}

// 统一生成option的方法
function getChartOption({ isFullScreen = false }: { isFullScreen?: boolean }) {
  const xUnit = props.xAxiosOption.xUnit ?? ''
  const yUnit = props.yAxiosOption.yUnit ?? ''

  const tooltipSize = isFullScreen
    ? {
        width: pxToResponsive(300),
        fontSize: pxToResponsive(32),
        itemFontSize: pxToResponsive(24),
        itemLineHeight: pxToResponsive(32),
        dotSize: pxToResponsive(20),
        gap: pxToResponsive(16),
      }
    : {
        width: pxToResponsive(220),
        fontSize: pxToResponsive(14),
        itemFontSize: pxToResponsive(12),
        itemLineHeight: pxToResponsive(18),
        dotSize: pxToResponsive(8),
        gap: pxToResponsive(8),
      }

  // 生成图例数据
  const legendData: string[] = []
  props.series.forEach((s) => {
    s.segments.forEach((seg) => {
      const legendName = `${s.name} (${seg.name})`
      if (!legendData.includes(legendName)) {
        legendData.push(legendName)
      }
    })
  })

  // legend样式参数
  const legend = isFullScreen
    ? {
        icon: 'circle',
        show: true,
        type: 'plain',
        orient: 'horizontal',
        right: pxToResponsive(50),
        top: pxToResponsive(40),
        itemWidth: pxToResponsive(20),
        itemHeight: pxToResponsive(20),
        itemGap: pxToResponsive(40),
        textStyle: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontSize: pxToResponsive(18),
          fontFamily: 'Arimo',
          fontWeight: 400,
        },
        data: legendData,
      }
    : {
        icon: 'circle',
        show: true,
        type: 'plain',
        orient: 'horizontal',
        right: 0,
        top: pxToResponsive(10),
        itemWidth: pxToResponsive(12),
        itemHeight: pxToResponsive(12),
        itemGap: pxToResponsive(25),
        textStyle: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontSize: pxToResponsive(12),
          fontFamily: 'Arimo',
          fontWeight: 400,
        },
        data: legendData,
      }

  const grid = getGridConfig(isFullScreen)

  const xAxis = isFullScreen
    ? {
        type: 'category',
        name: xUnit,
        nameTextStyle: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(16),
          padding: [pxToResponsive(15), 0, 0, 0],
        },
        data: props.xAxiosOption.xAxiosData,
        axisTick: {
          alignWithLabel: true,
          lineStyle: { color: '#fff' },
        },
        axisLine: { show: false },
        axisLabel: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(16),
        },
        splitLine: { show: false },
        boundaryGap: true,
      }
    : {
        type: 'category',
        name: xUnit,
        nameTextStyle: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(12),
          padding: [pxToResponsive(10), 0, 0, 0],
        },
        data: props.xAxiosOption.xAxiosData,
        axisTick: {
          alignWithLabel: true,
          lineStyle: { color: '#fff' },
        },
        axisLine: { show: false },
        axisLabel: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(12),
        },
        splitLine: { show: false },
        boundaryGap: true,
      }

  const yAxis = isFullScreen
    ? {
        type: 'value',
        name: yUnit,
        nameTextStyle: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(16),
          align: 'right',
          padding: [0, pxToResponsive(12), 0, 0],
        },
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(16),
        },
        splitLine: {
          show: true,
          lineStyle: {
            color: '#fff',
            type: 'dashed',
            opacity: 0.2,
          },
        },
      }
    : {
        type: 'value',
        name: yUnit,
        nameTextStyle: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(12),
          align: 'right',
          padding: [0, pxToResponsive(8), 0, 0],
        },
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          color: 'rgba(255, 255, 255, 0.6)',
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(12),
        },
        splitLine: {
          show: true,
          lineStyle: {
            color: '#fff',
            type: 'dashed',
            opacity: 0.2,
            width: pxToResponsive(1),
          },
        },
      }

  // 生成系列数据
  const seriesData: any[] = [
    // {
    //   name: 'background',
    //   type: 'bar',
    //   barWidth: '70%',
    //   barGap: '-100%',
    //   itemStyle: {
    //     color: 'rgba(255,255,255,0)',
    //   },
    //   data: totalData,
    //   showBackground: true,
    //   backgroundStyle: {
    //     color: 'rgba(252, 252, 253, 0.04)',
    //   },
    //   silent: true,
    //   emphasis: { disabled: true },
    //   tooltip: { show: false },
    //   label: { show: false },
    //   z: 0,
    // },
  ]

  // 是否已添加分界线标记
  let markLineAdded = false

  // 为每个系列的每个分段创建独立的线条
  props.series.forEach((s, seriesIndex) => {
    s.segments.forEach((seg, segIndex) => {
      const seriesItem: any = {
        name: `${s.name} (${seg.name})`,
        type: 'line',
        data: seg.data,
        smooth: true,
        symbol: 'circle',
        symbolSize: isFullScreen ? pxToResponsive(6) : pxToResponsive(0),
        lineStyle: {
          color: seg.color,
          width: isFullScreen ? pxToResponsive(6) : pxToResponsive(4),
          type: seg.lineType || 'solid',
        },
        itemStyle: {
          color: seg.color,
          borderColor: seg.color,
          borderWidth: isFullScreen ? pxToResponsive(3) : pxToResponsive(2),
        },
        emphasis: {
          focus: 'series',
          scale: false,
        },
        z: seriesIndex + segIndex + 1,
      }

      // 添加区域样式（可选）
      if (props.showAreaStyle) {
        seriesItem.areaStyle = {
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: adjustColorOpacity(seg.color, 0.3) },
              { offset: 1, color: adjustColorOpacity(seg.color, 0.05) },
            ],
          },
        }
      }

      // 只在第一个系列上添加分界线标记
      if (!markLineAdded && props.splitLines && props.splitLines.length > 0) {
        seriesItem.markLine = {
          silent: true,
          symbol: 'none',
          lineStyle: {
            color: 'rgba(255, 255, 255, 0.8)',
            type: 'dashed',
            width: isFullScreen ? pxToResponsive(2) : pxToResponsive(1),
          },
          label: {
            show: true,
            position: 'start',
            color: 'rgba(255, 255, 255, 0.85)',
            fontSize: isFullScreen ? pxToResponsive(14) : pxToResponsive(12),
            fontFamily: 'Arimo',
            fontWeight: 600,
            backgroundColor: 'rgba(63, 79, 117, 0.9)',
            padding: isFullScreen
              ? [pxToResponsive(6), pxToResponsive(10)]
              : [pxToResponsive(4), pxToResponsive(8)],
            borderRadius: 4,
          },
          data: props.splitLines.map((sl) => ({
            xAxis: sl.index,
            label: {
              formatter: sl.label ?? '',
            },
          })),
        }
        markLineAdded = true
      }

      seriesData.push(seriesItem)
    })
  })

  // tooltip
  const tooltip = isFullScreen
    ? {
        trigger: 'axis',
        confine: true,
        backgroundColor: '#3f4f75',
        borderColor: 'rgba(255,255,255,0.12)',
        borderWidth: pxToResponsive(2),
        padding: [pxToResponsive(30), pxToResponsive(40), pxToResponsive(30), pxToResponsive(40)],
        extraCssText: `
          border-radius: ${pxToResponsive(24)}px;
          box-shadow: 0 ${pxToResponsive(16)}px ${pxToResponsive(32)}px 0 rgba(0,0,0,0.15);
          max-width: ${pxToResponsive(500)}px;
        `,
        textStyle: {
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(24),
          color: 'rgba(255,255,255,0.85)',
          lineHeight: pxToResponsive(32),
        },
        axisPointer: {
          type: 'shadow',
          shadowStyle: {
            color: 'rgba(79, 173, 247, 0.08)',
          },
          lineStyle: {
            width: pxToResponsive(2),
          },
        },
        formatter: (params: any) => customTooltipFormatter(params, tooltipSize, yUnit),
      }
    : {
        trigger: 'axis',
        confine: true,
        backgroundColor: '#3f4f75',
        borderColor: 'rgba(255,255,255,0.12)',
        borderWidth: pxToResponsive(1),
        padding: [pxToResponsive(10), pxToResponsive(16), pxToResponsive(10), pxToResponsive(16)],
        extraCssText: `
          border-radius: ${pxToResponsive(8)}px;
          box-shadow: 0 ${pxToResponsive(4)}px ${pxToResponsive(16)}px 0 rgba(0,0,0,0.12);
          max-width: ${pxToResponsive(220)}px;
        `,
        textStyle: {
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: pxToResponsive(12),
          color: 'rgba(255,255,255,0.85)',
          lineHeight: pxToResponsive(18),
        },
        axisPointer: {
          type: 'shadow',
          shadowStyle: {
            color: 'rgba(79, 173, 247, 0.08)',
          },
          lineStyle: {
            width: pxToResponsive(1),
          },
        },
        formatter: (params: any) => customTooltipFormatter(params, tooltipSize, yUnit),
      }

  const dataZoom = [
    {
      type: 'inside',
      show: true,
    },
  ]

  const toolbox = isFullScreen
    ? {
        itemSize: 20,
        itemGap: 26,
        top: -10,
        right: 40,
        iconStyle: {
          borderColor: '#fff',
          borderWidth: 1,
        },
        emphasis: {
          iconStyle: {
            borderColor: '#fff',
            borderWidth: 1,
          },
        },
        textStyle: {
          fontFamily: 'Arimo',
          fontWeight: 400,
          fontSize: 12,
          color: 'rgba(255,255,255,1)',
        },
        feature: {
          myDownload: {
            show: true,
            title: '',
            icon: 'path:// M160 832h704a32 32 0 1 1 0 64H160a32 32 0 1 1 0-64m384-253.696 236.288-236.352 45.248 45.248L508.8 704 192 387.2l45.248-45.248L480 584.704V128h64z',
            onclick: handleExport,
            iconStyle: {
              color: '#fff',
            },
          },
        },
      }
    : {}

  return {
    legend,
    grid,
    tooltip,
    dataZoom,
    xAxis,
    yAxis,
    toolbox,
    series: seriesData,
  }
}

// 初始化echarts
const initChart = () => {
  if (!chartRef.value) return
  if (chartInstance) {
    chartInstance.dispose()
  }
  chartInstance = echarts.init(chartRef.value, undefined, { renderer: 'svg' })
  chartInstance.setOption(getChartOption({ isFullScreen: false }))
}

// 初始化全屏图表
const initFullScreenChart = () => {
  if (!fullScreenChartRef.value) return
  if (fullScreenChartInstance) {
    fullScreenChartInstance.dispose()
  }
  fullScreenChartInstance = echarts.init(fullScreenChartRef.value, undefined, { renderer: 'svg' })
  fullScreenChartInstance.setOption(getChartOption({ isFullScreen: true }))
}

const handleFullScreen = () => {
  fullScreenDialogRef.value.dialogVisible = true
  nextTick(() => {
    setTimeout(() => {
      initFullScreenChart()
    }, 100)
  })
}

const handleExport = () => {
  const exportData: (string | number)[][] = []

  // 添加表头
  const headers: (string | number)[] = ['time']
  props.series.forEach((s) => {
    s.segments.forEach((seg) => {
      headers.push(
        `${s.name} (${seg.name})${props.yAxiosOption.yUnit ? ' (' + props.yAxiosOption.yUnit + ')' : ''}`,
      )
    })
  })
  exportData.push(headers)

  // 添加数据
  props.xAxiosOption.xAxiosData.forEach((time: string, index: number) => {
    const row: (string | number)[] = [time]
    props.series.forEach((s) => {
      s.segments.forEach((seg) => {
        row.push(seg.data[index] ?? '')
      })
    })
    exportData.push(row)
  })

  const wb = XLSX.utils.book_new()
  const ws = XLSX.utils.aoa_to_sheet(exportData)
  XLSX.utils.book_append_sheet(wb, ws, 'forecast_chart_data')
  const fileName = `forecast_chart_data_${new Date().toISOString().slice(0, 10)}.xlsx`
  XLSX.writeFile(wb, fileName)
}

// 监听窗口大小变化
const resizeFullScreenChart = () => {
  if (fullScreenChartInstance && fullScreenDialogRef.value.dialogVisible) {
    setTimeout(() => {
      fullScreenChartInstance?.setOption(getChartOption({ isFullScreen: true }), true)
      fullScreenChartInstance?.resize()
    }, 300)
  }
}

const resizeChart = () => {
  setTimeout(() => {
    if (chartInstance) {
      chartInstance.setOption(getChartOption({ isFullScreen: false }), true)
      chartInstance.resize()
    }
  }, 300)
}

watch(
  () => [props.xAxiosOption.xAxiosData, props.series],
  () => {
    initChart()
  },
  { deep: true },
)

onMounted(() => {
  initChart()
  window.addEventListener('resize', resizeChart)
  window.addEventListener('resize', resizeFullScreenChart)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', resizeChart)
  window.removeEventListener('resize', resizeFullScreenChart)
  chartInstance?.dispose()
  fullScreenChartInstance?.dispose()
})
</script>

<style scoped lang="scss">
.forecast-line-chart {
  width: 100%;
  height: 100%;
  position: relative;

  .forecast-line-chart-container {
    width: 100%;
    height: 100%;
  }

  .forecast-line-chart-toolbox {
    position: absolute;
    top: -0.2rem;
    right: 0;
    display: flex;
    align-items: center;
    gap: 0.2rem;

    .forecast-line-chart-toolbox-item {
      width: 0.14rem;
      height: 0.14rem;
      cursor: pointer;
    }
  }
}

.forecast-line-chart-full-screen {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #212c49;
  overflow: hidden;

  .forecast-line-chart-full-screen__container {
    width: 100%;
    height: calc(100vh - 1.1rem);
    overflow: hidden;
  }
}
</style>
