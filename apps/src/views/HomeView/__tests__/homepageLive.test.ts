import { computed, defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Home from '../index.vue'
import wsManager from '@/utils/websocket'

const mocks = vi.hoisted(() => ({ websocket: vi.fn(), statistics: vi.fn(), alarms: vi.fn(), history: vi.fn() }))
vi.mock('@/composables/useWebSocket', () => ({ default: mocks.websocket }))
vi.mock('@/composables/useEnergyStatistics', () => ({ useEnergyStatistics: mocks.statistics }))
vi.mock('@/utils/websocket', async () => ({ default: { status: (await import('vue')).ref('connected') } }))
vi.mock('@/api/alarm', () => ({ getCurrentAlarms: mocks.alarms }))
vi.mock('@/api/Statistic/overview', () => ({ batchQueryHistory: mocks.history }))
vi.mock('@/components/charts/lineChart.vue', () => ({ default: { template: '<div />' } }))
vi.mock('@/components/charts/StackedBarChart.vue', () => ({ default: { template: '<div />' } }))
vi.mock('../HomeBg.vue', () => ({ default: { props: ['data'], template: '<div />' } }))
vi.mock('element-plus/es/components/base/style/css', () => ({}))
vi.mock('element-plus/es/components/carousel/style/css', () => ({}))
vi.mock('element-plus/es/components/carousel-item/style/css', () => ({}))
const container = defineComponent({ setup: (_, { slots }) => () => h('div', slots.default?.()) })
const energyCard = defineComponent({ props: ['title', 'value', 'unit'], setup: (props) =>
  () => h('div', { 'data-title': props.title }, `${props.value} ${props.unit}`) })
const options = { global: { stubs: {
  ModuleCard: container, EnergyCard: energyCard, ElCarousel: container, ElCarouselItem: container,
} } }
const update = (id: number, name: string, values: number | null, unit = 'kWh') => ({ id, name, values, unit, ts: 1_700_000_000_000 })

describe('homepage custom live data', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    wsManager.status.value = 'connected'
    mocks.history.mockResolvedValue({ success: true, data: { series: [] } })
    mocks.alarms.mockResolvedValue({ success: true, data: { list: [] } })
    mocks.statistics.mockReturnValue({ metrics: computed(() => ({})), live: ref({}), loading: ref(false) })
  })

  it('subscribes to custom homepage points and preserves backend labels, values, zero and null', async () => {
    const wrapper = mount(Home, options)
    await flushPromises()
    expect(mocks.statistics).toHaveBeenCalledWith(expect.any(Function), { realtime: false })
    const [config, listeners] = mocks.websocket.mock.calls[0]
    expect(config).toEqual({ source: 'homepage', interval: 1000 })
    listeners.onBatchDataUpdate({ updates: [
      update(1, 'Custom PV Energy', 12.5), update(2, 'Generator Energy', 0),
      update(3, 'Load Energy', null), update(19, 'Battery SOC', 75, '%'),
      update(14, 'PV Power', 12.34567, 'kW'),
    ] })
    await flushPromises()
    expect(wrapper.find('[data-title="Custom PV Energy"]').text()).toBe('12.5 kWh')
    expect(wrapper.find('[data-title="Generator Energy"]').text()).toBe('0 kWh')
    expect(wrapper.find('[data-title="Load Energy"]').text()).toBe('- kWh')
    expect(wrapper.findComponent({ name: 'HomeBg' }).props('data').ess.soc.value).toBe('75')
    expect(wrapper.findComponent({ name: 'HomeBg' }).props('data').pv.P).toMatchObject({
      value: '12.346', rawValue: 12.34567,
    })
    wrapper.unmount()
  })

  it('drops removed snapshot points and clears old values on disconnect', async () => {
    const wrapper = mount(Home, options)
    await flushPromises()
    const listeners = mocks.websocket.mock.calls[0][1]
    listeners.onBatchDataUpdate({ updates: [update(1, 'Old PV', 99), update(2, 'DG', 12)] })
    await flushPromises()
    listeners.onBatchDataUpdate({ updates: [update(2, 'DG', 13)] })
    await flushPromises()
    expect(wrapper.find('[data-title="Old PV"]').exists()).toBe(false)
    expect(wrapper.find('[data-title="PV Energy"]').text()).toBe('- kWh')
    listeners.onBatchDataUpdate({ updates: [] })
    await flushPromises()
    expect(wrapper.find('[data-title="DG"]').exists()).toBe(false)
    listeners.onBatchDataUpdate({ updates: [update(2, 'DG', 13)] })
    await flushPromises()
    wsManager.status.value = 'disconnected'
    await flushPromises()
    expect(wrapper.find('[data-title="DG"]').exists()).toBe(false)
    expect(wrapper.find('[data-title="Diesel Energy"]').text()).toBe('- kWh')
    expect(wrapper.findComponent({ name: 'HomeBg' }).props('data').pv.P.rawValue).toBeNull()
    wrapper.unmount()
  })
  it('rejects initialized zeros and values without a valid sample timestamp', async () => {
    const wrapper = mount(Home, options)
    await flushPromises()
    const listeners = mocks.websocket.mock.calls[0][1]
    listeners.onBatchDataUpdate({ updates: [
      { ...update(1, 'PV', 0), ts: 0 },
      { ...update(2, 'DG', 12), ts: undefined },
      update(3, 'Load', NaN),
    ] })
    await flushPromises()
    expect(wrapper.find('[data-title="PV"]').text()).toBe('- kWh')
    expect(wrapper.find('[data-title="DG"]').text()).toBe('- kWh')
    expect(wrapper.find('[data-title="Load"]').text()).toBe('- kWh')
    wrapper.unmount()
  })

  it('keeps a newer alarm response when the startup request finishes late', async () => {
    let finishOld!: (value: any) => void
    let finishNew!: (value: any) => void
    mocks.alarms.mockImplementationOnce(() => new Promise(resolve => { finishOld = resolve }))
      .mockImplementationOnce(() => new Promise(resolve => { finishNew = resolve }))
    wsManager.status.value = 'connecting'
    const wrapper = mount(Home, options)
    wsManager.status.value = 'connected'
    await flushPromises()
    const response = (id: number, name: string) => ({ success: true, data: { list: [
      { id, warning_level: 1, device_name: name, channel_id: 1, point_id: 1 },
    ] } })
    finishNew(response(2, 'Newest alarm'))
    await flushPromises()
    finishOld(response(1, 'Old alarm'))
    await flushPromises()
    expect(wrapper.text()).toContain('Newest alarm')
    expect(wrapper.text()).not.toContain('Old alarm')
    wrapper.unmount()
  })

  it('shows an unavailable state when the real history API fails', async () => {
    const log = vi.spyOn(console, 'error').mockImplementation(() => {})
    mocks.statistics.mockReturnValue({ metrics: computed(() => ({
      pvPower: { instanceId: 6, terms: [{ pointId: '1', factor: 1 }], counter: false },
    })), live: ref({}), loading: ref(false) })
    mocks.history.mockRejectedValue(new Error('database unavailable'))
    const wrapper = mount(Home, options)
    await flushPromises()
    expect(mocks.history).toHaveBeenCalled()
    expect(wrapper.findAll('.home-chart-empty').map(item => item.text()))
      .toEqual(['Historical data unavailable', 'Historical data unavailable'])
    wrapper.unmount()
    log.mockRestore()
  })

  it('queries history once after four device point tables finish, and keeps real errors visible', async () => {
    const log = vi.spyOn(console, 'error').mockImplementation(() => {})
    const loading = ref(true)
    const metrics = ref({})
    mocks.statistics.mockReturnValue({ metrics, live: ref({}), loading })
    mocks.history.mockRejectedValue(new Error('database unavailable'))
    const wrapper = mount(Home, options)
    try {
      await flushPromises()
      const names = ['pvPower', 'essEnergy', 'dgPower', 'loadEnergy']
      for (const [index, name] of names.entries()) {
        metrics.value = { ...metrics.value, [name]: {
          instanceId: index + 1, terms: [{ pointId: '1', factor: 1 }], counter: false,
        } }
        await flushPromises()
        expect(mocks.history).not.toHaveBeenCalled()
      }
      loading.value = false
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(1)
      expect(mocks.history.mock.calls[0][0].series).toHaveLength(4)
      expect(wrapper.findAll('.home-chart-empty').map(item => item.text()))
        .toEqual(['Historical data unavailable', 'Historical data unavailable'])
    } finally {
      wrapper.unmount()
      log.mockRestore()
    }
  })

  it('does not cancel a pending history query on the next polling tick', async () => {
    vi.useFakeTimers()
    let finish!: (value: any) => void
    mocks.statistics.mockReturnValue({ metrics: computed(() => ({
      pvPower: { instanceId: 1, terms: [{ pointId: '1', factor: 1 }], counter: false },
    })), live: ref({}), loading: ref(false) })
    mocks.history.mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const wrapper = mount(Home, options)
    try {
      await vi.advanceTimersByTimeAsync(0)
      const signal = mocks.history.mock.calls[0][1]
      await vi.advanceTimersByTimeAsync(60_000)
      expect(mocks.history).toHaveBeenCalledTimes(1)
      expect(signal.aborted).toBe(false)
      finish({ success: true, data: { series: [] } })
      await vi.advanceTimersByTimeAsync(0)
      await vi.advanceTimersByTimeAsync(60_000)
      expect(mocks.history).toHaveBeenCalledTimes(2)
    } finally {
      wrapper.unmount()
      vi.useRealTimers()
    }
  })

})
