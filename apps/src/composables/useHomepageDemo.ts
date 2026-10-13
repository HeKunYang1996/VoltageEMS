import { onMounted, onUnmounted, type Ref } from 'vue'
import {
  buildDemoAlarms,
  buildDemoChartData,
  DEMO_ALARM_NUM,
  jitterDemoUpdates,
  snapshotToPointUpdates,
  type DemoAlarmItem,
  type MicrogridSnapshot,
} from '@/mocks/homepageDemo'
import { useGlobalStore } from '@/stores/global'

/** 是否处于首页营销演示模式（VITE_DEMO_MODE=true）。 */
export function isHomepageDemoMode(
  env: Pick<ImportMetaEnv, 'VITE_DEMO_MODE'> = import.meta.env,
): boolean {
  return env.VITE_DEMO_MODE === 'true'
}

/** 演示模式下不订阅 homepage WebSocket，改由 mock 定时刷新。 */
export function shouldSubscribeHomepageWebSocket(
  env: Pick<ImportMetaEnv, 'VITE_DEMO_MODE'> = import.meta.env,
): boolean {
  return !isHomepageDemoMode(env)
}

interface ChartSeriesItem {
  name: string
  data: number[]
  color: string
}

interface HomepageDemoOptions {
  applyHomepageBatch: (data: {
    updates?: Array<{ id: number; name: string; values?: number; unit: string; imgurl?: string }>
  }) => void
  alarmInfoList: Ref<DemoAlarmItem[]>
  chartXAxisData: Ref<string[]>
  lineChartSeries: Ref<ChartSeriesItem[]>
  exampleSeries: Ref<ChartSeriesItem[]>
  /** 刷新间隔 ms，默认 3s */
  interval?: number
}

/** 同步写入首页 mock 数据，首屏即可展示 */
export function seedHomepageDemo(options: Omit<HomepageDemoOptions, 'interval'>) {
  const { applyHomepageBatch, alarmInfoList, chartXAxisData, lineChartSeries, exampleSeries } =
    options

  const globalStore = useGlobalStore()
  const chart = buildDemoChartData()
  applyHomepageBatch({ updates: snapshotToPointUpdates(chart.snapshot) })
  alarmInfoList.value = buildDemoAlarms(chart.snapshot)
  globalStore.alarmNum = DEMO_ALARM_NUM
  chartXAxisData.value = chart.xAxisData
  lineChartSeries.value = chart.lineChartSeries
  exampleSeries.value = chart.energyChartSeries
}

/** 营销演示模式：优先展示 mock 数据并模拟实时波动 */
export function useHomepageDemo(options: HomepageDemoOptions) {
  const { applyHomepageBatch, alarmInfoList, chartXAxisData, lineChartSeries, exampleSeries } =
    options
  const intervalMs = options.interval ?? 3000

  let timer: ReturnType<typeof setInterval> | null = null
  let lastSnapshot: MicrogridSnapshot = buildDemoChartData().snapshot

  // 同步注入，避免 onMounted 前出现 '-' 占位
  seedHomepageDemo({
    applyHomepageBatch,
    alarmInfoList,
    chartXAxisData,
    lineChartSeries,
    exampleSeries,
  })
  lastSnapshot = buildDemoChartData().snapshot

  onMounted(() => {
    timer = setInterval(() => {
      const { updates, snapshot } = jitterDemoUpdates(snapshotToPointUpdates(lastSnapshot))
      lastSnapshot = snapshot
      applyHomepageBatch({ updates })
      alarmInfoList.value = buildDemoAlarms(snapshot)
    }, intervalMs)
  })

  onUnmounted(() => {
    if (timer) clearInterval(timer)
  })

  return {
    refreshAlarms: () => {
      alarmInfoList.value = buildDemoAlarms(lastSnapshot)
    },
  }
}
