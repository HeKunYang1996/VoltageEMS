import { defineComponent, h, ref } from 'vue'
import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  isHomepageDemoMode,
  seedHomepageDemo,
  shouldSubscribeHomepageWebSocket,
  useHomepageDemo,
} from '../useHomepageDemo'
import { setActivePinia, createPinia } from 'pinia'
import { useGlobalStore } from '@/stores/global'

describe('composables/useHomepageDemo.ts', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('demo mode guards', () => {
    it('detects demo mode from VITE_DEMO_MODE', () => {
      expect(isHomepageDemoMode({ VITE_DEMO_MODE: 'true' } as ImportMetaEnv)).toBe(true)
      expect(isHomepageDemoMode({ VITE_DEMO_MODE: 'false' } as ImportMetaEnv)).toBe(false)
      expect(isHomepageDemoMode({ VITE_DEMO_MODE: undefined } as ImportMetaEnv)).toBe(false)
    })

    it('skips homepage WebSocket subscription in demo mode', () => {
      expect(shouldSubscribeHomepageWebSocket({ VITE_DEMO_MODE: 'true' } as ImportMetaEnv)).toBe(
        false,
      )
      expect(shouldSubscribeHomepageWebSocket({ VITE_DEMO_MODE: 'false' } as ImportMetaEnv)).toBe(
        true,
      )
    })
  })

  describe('seedHomepageDemo', () => {
    it('seeds homepage batch updates including topology IDs 14–19', () => {
      const applyHomepageBatch = vi.fn()
      const alarmInfoList = ref([])
      const chartXAxisData = ref<string[]>([])
      const lineChartSeries = ref([])
      const exampleSeries = ref([])

      seedHomepageDemo({
        applyHomepageBatch,
        alarmInfoList,
        chartXAxisData,
        lineChartSeries,
        exampleSeries,
      })

      expect(applyHomepageBatch).toHaveBeenCalledTimes(1)
      const updates = applyHomepageBatch.mock.calls[0][0].updates ?? []
      const ids = updates.map((item: { id: number }) => item.id)
      expect(ids).toEqual(expect.arrayContaining([14, 15, 16, 17, 18, 19]))
      expect(chartXAxisData.value.length).toBeGreaterThan(0)
      expect(lineChartSeries.value.length).toBe(3)
      expect(exampleSeries.value.length).toBe(3)
      expect(alarmInfoList.value.length).toBeGreaterThan(0)
      const byId = Object.fromEntries(
        updates.map((point: { id: number; values: number }) => [point.id, point.values]),
      )
      expect(lineChartSeries.value.map((series: { data: number[] }) => series.data.at(-1))).toEqual(
        [byId[5], byId[6], byId[7]],
      )
      expect(byId[5] + byId[6] + byId[7]).toBeCloseTo(byId[15], 1)
      expect(useGlobalStore().alarmNum).toBe(alarmInfoList.value.length)
    })
  })

  describe('useHomepageDemo', () => {
    it('keeps cards and charts stable across idle time and remounts without a timer', () => {
      const applyHomepageBatch = vi.fn()
      const alarmInfoList = ref([])
      const chartXAxisData = ref<string[]>([])
      const lineChartSeries = ref([])
      const exampleSeries = ref([])

      const Host = defineComponent({
        setup() {
          useHomepageDemo({
            applyHomepageBatch,
            alarmInfoList,
            chartXAxisData,
            lineChartSeries,
            exampleSeries,
          })
          return () => h('div')
        },
      })

      const wrapper = mount(Host)
      const original = structuredClone(applyHomepageBatch.mock.calls[0][0])
      const originalCharts = JSON.stringify([
        chartXAxisData.value,
        lineChartSeries.value,
        exampleSeries.value,
      ])
      expect(applyHomepageBatch).toHaveBeenCalledTimes(1)
      expect(vi.getTimerCount()).toBe(0)

      vi.advanceTimersByTime(24 * 60 * 60 * 1000)
      expect(applyHomepageBatch).toHaveBeenCalledTimes(1)
      expect(
        JSON.stringify([chartXAxisData.value, lineChartSeries.value, exampleSeries.value]),
      ).toBe(originalCharts)
      wrapper.unmount()
      const remounted = mount(Host)
      expect(applyHomepageBatch.mock.calls.at(-1)?.[0]).toEqual(original)
      remounted.unmount()

      const latestUpdates = applyHomepageBatch.mock.calls.at(-1)?.[0]?.updates ?? []
      expect(latestUpdates.map((item: { id: number }) => item.id)).toEqual(
        expect.arrayContaining([14, 15, 16, 17, 18, 19]),
      )
    })
  })
})
