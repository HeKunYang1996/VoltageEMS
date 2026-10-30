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
    })
  })

  describe('useHomepageDemo', () => {
    it('refreshes mock data on interval without WebSocket', () => {
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
            interval: 1000,
          })
          return () => h('div')
        },
      })

      mount(Host)
      expect(applyHomepageBatch).toHaveBeenCalledTimes(1)

      vi.advanceTimersByTime(1000)
      expect(applyHomepageBatch.mock.calls.length).toBeGreaterThan(1)

      const latestUpdates = applyHomepageBatch.mock.calls.at(-1)?.[0]?.updates ?? []
      expect(latestUpdates.map((item: { id: number }) => item.id)).toEqual(
        expect.arrayContaining([14, 15, 16, 17, 18, 19]),
      )
    })
  })
})
