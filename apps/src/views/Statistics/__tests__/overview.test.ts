import { computed, defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { StatisticMetrics } from '@/utils/energyStatistics'
import { batchQueryHistory } from '@/api/Statistic/overview'
import Overview from '../Overview.vue'

const mock = vi.hoisted(() => ({ statistics: vi.fn() }))
vi.mock('@/composables/useEnergyStatistics', () => ({ useEnergyStatistics: mock.statistics }))
vi.mock('@/api/Statistic/overview', () => ({ batchQueryHistory: vi.fn() }))
vi.mock('element-plus/es/components/base/style/css', () => ({}))
vi.mock('element-plus/es/components/date-picker/style/css', () => ({}))
vi.mock('@/components/charts/StackedBarChart.vue', () => ({ default: { name: 'StackedBarChart', props: ['series'], template: '<div />' } }))
vi.mock('@/components/charts/lineChart.vue', () => ({ default: { name: 'lineChart', props: ['series'], template: '<div />' } }))
vi.mock('@/components/charts/DoughnutChart.vue', () => ({ default: { template: '<div />' } }))

const metrics = ref<StatisticMetrics>({})
const live = ref({ pvPower: null as number | null, essPower: null as number | null, dgPower: null as number | null })
const loadMetric = (instanceId: number) => ({ instanceId, terms: [{ pointId: '14', factor: 1 }], counter: true })
const response = (id: number, start: number, end: number) => ({ success: true, data: { series: [{
  redis_key: `inst:${id}:M`, point_id: '14', count: 2, data: [
    { timestamp: '2026-10-29T00:00:00.000Z', value: start },
    { timestamp: '2026-10-29T01:00:00.000Z', value: end },
  ],
}] } })
const container = defineComponent({ setup: (_, { slots }) => () => h('div', slots.default?.()) })
const options = { global: { stubs: { ModuleCard: container, LoadingBg: container, EnergyCard: true, ElDatePicker: true } } }

describe('statistics overview consumption', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    metrics.value = { loadEnergy: loadMetric(1) }
    live.value = { pvPower: null, essPower: null, dgPower: null }
    mock.statistics.mockReturnValue({ metrics: computed(() => metrics.value), live })
  })

  it('loads a meter without PV/ESS/Diesel and keeps realtime cards from replacing range energy', async () => {
    vi.mocked(batchQueryHistory).mockResolvedValue(response(1, 100, 107) as any)
    const wrapper = mount(Overview, options)
    await flushPromises()
    expect(batchQueryHistory).toHaveBeenCalledWith(expect.objectContaining({
      series: [{ redis_key: 'inst:1:M', point_id: '14' }],
    }), expect.any(AbortSignal))
    expect(wrapper.find('.chart__review-header-value').text().replace(/\s+/g, ' ')).toBe('7 kWh')
    const bars = wrapper.findAllComponents({ name: 'StackedBarChart' })
    expect(bars[0].props('series')[0].data).toEqual([null, 7])
    live.value = { pvPower: 999, essPower: -2, dgPower: 0 }
    await flushPromises()
    expect(wrapper.find('.chart__review-header-value').text().replace(/\s+/g, ' ')).toBe('7 kWh')
    wrapper.unmount()
  })

  it('discards a previous meter response after switching devices', async () => {
    let finishOld!: (value: any) => void
    vi.mocked(batchQueryHistory).mockImplementationOnce(() => new Promise((resolve) => { finishOld = resolve }))
      .mockResolvedValue(response(2, 50, 52) as any)
    const wrapper = mount(Overview, options)
    metrics.value = { loadEnergy: loadMetric(2) }
    await flushPromises()
    expect(wrapper.find('.chart__review-header-value').text().replace(/\s+/g, ' ')).toBe('2 kWh')
    finishOld(response(1, 100, 199))
    await flushPromises()
    expect(wrapper.find('.chart__review-header-value').text().replace(/\s+/g, ' ')).toBe('2 kWh')
    metrics.value = {}
    await flushPromises()
    expect(wrapper.find('.chart__review-header-value').text().replace(/\s+/g, ' ')).toBe('-- kWh')
    wrapper.unmount()
  })

  it('clears preset results and waits for valid dates when Custom is selected', async () => {
    let finish!: (value: any) => void
    vi.mocked(batchQueryHistory).mockImplementation(() => new Promise((resolve) => { finish = resolve }))
    const wrapper = mount(Overview, options)
    await wrapper.find('[data-value="custom"]').trigger('click')
    finish(response(1, 10, 99))
    await flushPromises()
    expect(wrapper.find('.chart__review-header-value').text().replace(/\s+/g, ' ')).toBe('-- kWh')
    metrics.value = { loadEnergy: loadMetric(2) }
    await flushPromises()
    expect(batchQueryHistory).toHaveBeenCalledTimes(1)
    const picker = wrapper.findComponent({ name: 'ElDatePicker' })
    picker.vm.$emit('update:modelValue', null)
    picker.vm.$emit('change', null)
    await flushPromises()
    expect(batchQueryHistory).toHaveBeenCalledTimes(1)
    wrapper.unmount()
  })
})
