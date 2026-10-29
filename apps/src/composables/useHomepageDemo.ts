import type { Ref } from 'vue'
import {
  buildDemoAlarms,
  buildDemoChartData,
  snapshotToPointUpdates,
  type DemoAlarmItem,
} from '@/mocks/homepageDemo'
import { homepageChartColor } from '@/utils/theme'
import { useGlobalStore } from '@/stores/global'

/** 是否处于首页营销演示模式（VITE_DEMO_MODE=true）。 */
export function isHomepageDemoMode(
  env: Pick<ImportMetaEnv, 'VITE_DEMO_MODE'> = import.meta.env,
): boolean {
  return env.VITE_DEMO_MODE === 'true'
}

/** 演示模式下不订阅 homepage WebSocket，统一使用固定场景。 */
export function shouldSubscribeHomepageWebSocket(
  env: Pick<ImportMetaEnv, 'VITE_DEMO_MODE'> = import.meta.env,
): boolean {
  return !isHomepageDemoMode(env)
}

interface ChartSeriesItem {
  name: string
  data: (number | null)[]
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
}

/** 同步写入首页 mock 数据，首屏即可展示 */
export function seedHomepageDemo(options: HomepageDemoOptions) {
  const { applyHomepageBatch, alarmInfoList, chartXAxisData, lineChartSeries, exampleSeries } =
    options

  const globalStore = useGlobalStore()
  const chart = buildDemoChartData()
  applyHomepageBatch({ updates: snapshotToPointUpdates(chart.snapshot) })
  alarmInfoList.value = buildDemoAlarms(chart.snapshot)
  globalStore.alarmNum = alarmInfoList.value.length
  chartXAxisData.value = chart.xAxisData
  lineChartSeries.value = chart.lineChartSeries.map((series) => ({
    ...series,
    color: homepageChartColor(series.name),
  }))
  exampleSeries.value = chart.energyChartSeries.map((series) => ({
    ...series,
    color: homepageChartColor(series.name),
  }))
  return chart.snapshot
}

/** Fixed marketing snapshot: idle time cannot drift cards away from the chart endpoint. */
export function useHomepageDemo(options: HomepageDemoOptions) {
  const snapshot = seedHomepageDemo(options)
  const globalStore = useGlobalStore()
  return {
    refreshAlarms: () => {
      options.alarmInfoList.value = buildDemoAlarms(snapshot)
      globalStore.alarmNum = options.alarmInfoList.value.length
    },
  }
}
