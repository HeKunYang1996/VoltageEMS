import { defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Curves from '../Curves.vue'

const mocks = vi.hoisted(() => ({ history: vi.fn(), points: vi.fn(), topology: vi.fn() }))
vi.mock('@/api/Statistic/overview', () => ({ batchQueryHistory: mocks.history }))
vi.mock('@/api/devicesManagement', () => ({ getInstancePoints: mocks.points }))
vi.mock('@/stores/deviceTopology', () => ({ useDeviceTopologyStore: mocks.topology }))
vi.mock('element-plus/es/components/base/style/css', () => ({}))
vi.mock('element-plus/es/components/select/style/css', () => ({}))
vi.mock('element-plus/es/components/option/style/css', () => ({}))
vi.mock('element-plus/es/components/date-picker/style/css', () => ({}))
vi.mock('@/components/charts/lineChart.vue', () => ({ default: {
  name: 'lineChart', props: ['series', 'xAxiosOption'], template: '<div />',
} }))

const container = defineComponent({ setup: (_, { slots }) => () => h('div', slots.default?.()) })
const loading = defineComponent({ name: 'LoadingBg', props: ['loading'],
  setup: (_, { slots }) => () => h('div', slots.default?.()) })
const options = { global: { stubs: {
  ModuleCard: container, LoadingBg: loading, ElSelect: true, ElOption: true, ElDatePicker: true,
} } }
const response = (id: number, value: number) => ({ success: true, data: { series: [{
  redis_key: `inst:${id}:M`, point_id: '1', count: 1,
  data: [{ timestamp: '2026-10-30T00:00:00Z', value }],
}] } })
const pending: Array<{ resolve: (value: any) => void; reject: (reason: Error) => void; signal: AbortSignal }> = []

describe('statistics curves request ownership', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    pending.length = 0
    mocks.topology.mockReturnValue({
      getTopologyDevices: () => [1, 2].map(id => ({ id, name: `Battery ${id}`, product_name: 'Battery' })),
      loaded: true, topology: { flow_json: { nodes: [{ id: 'a' }, { id: 'b' }] } },
      bindings: [1, 2].map((id) => ({ nodeId: id === 1 ? 'a' : 'b', productName: 'Battery',
        instances: [{ instanceId: id, instanceName: `Battery ${id}` }],
      })), instances: [],
    })
    mocks.points.mockResolvedValue({ success: true, data: { measurements: [
      { measurement_id: 1, name: 'Voltage', unit: 'V' },
    ] } })
    mocks.history.mockImplementation((_, signal: AbortSignal) => new Promise((resolve, reject) => {
      pending.push({ resolve, reject, signal })
    }))
  })

  it('selects a device once, keeps other devices pending, and reuses a pending query', async () => {
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(1)
      const select = wrapper.findComponent({ name: 'ElSelect' })
      select.vm.$emit('update:modelValue', '2')
      select.vm.$emit('change', '2')
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(2)
      expect(pending[0].signal.aborted).toBe(false)
      select.vm.$emit('update:modelValue', 'all')
      select.vm.$emit('change', 'all')
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(2)
      pending[0].resolve(response(1, 11))
      pending[1].resolve(response(2, 22))
      await flushPromises()
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(2)
      expect(wrapper.findAllComponents({ name: 'lineChart' }).map(chart => chart.props('series')[0].data))
        .toEqual([[11], [22]])
    } finally { wrapper.unmount() }
  })

  it('allows concurrent expansions and ignores old responses after the time range changes', async () => {
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      expect(mocks.history).toHaveBeenCalledTimes(2)
      expect(pending[0].signal.aborted).toBe(false)
      await wrapper.find('[data-value="1d"]').trigger('click')
      expect(mocks.history).toHaveBeenCalledTimes(4)
      expect(pending.slice(0, 2).every(item => item.signal.aborted)).toBe(true)
      pending[0].resolve(response(1, 99))
      pending[1].resolve(response(2, 99))
      await flushPromises()
      expect(wrapper.findComponent({ name: 'LoadingBg' }).props('loading')).toBe(false)
      expect(wrapper.findAll('.curves__device-section').every(section => section.attributes('aria-busy') === 'true')).toBe(true)
      expect(wrapper.findAllComponents({ name: 'lineChart' }).map(chart => chart.props('series')[0].data))
        .toEqual([[], []])
      pending[2].resolve(response(1, 12))
      pending[3].resolve(response(2, 23))
      await flushPromises()
      expect(wrapper.findComponent({ name: 'LoadingBg' }).props('loading')).toBe(false)
      expect(wrapper.findAllComponents({ name: 'lineChart' }).map(chart => chart.props('series')[0].data))
        .toEqual([[12], [23]])
      await wrapper.find('[data-value="1d"]').trigger('click')
      expect(mocks.history).toHaveBeenCalledTimes(4)
    } finally { wrapper.unmount() }
  })

  it('invalidates preset queries and waits for a valid Custom range', async () => {
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      await wrapper.find('[data-value="custom"]').trigger('click')
      expect(pending[0].signal.aborted).toBe(true)
      expect(mocks.history).toHaveBeenCalledTimes(1)
      pending[0].resolve(response(1, 99))
      await flushPromises()
      const picker = wrapper.findComponent({ name: 'ElDatePicker' })
      picker.vm.$emit('update:modelValue', null)
      picker.vm.$emit('change', null)
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(1)
      picker.vm.$emit('update:modelValue', ['2026-10-29 00:00:00', '2026-10-30 00:00:00'])
      picker.vm.$emit('change')
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(2)
      picker.vm.$emit('change')
      await flushPromises()
      expect(mocks.history).toHaveBeenCalledTimes(2)
      expect(pending[1].signal.aborted).toBe(false)
    } finally { wrapper.unmount() }
  })

  it('shows a real device query failure without cancelling another device', async () => {
    const log = vi.spyOn(console, 'error').mockImplementation(() => {})
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      pending[0].reject(new Error('database unavailable'))
      await flushPromises()
      expect(wrapper.find('[role="alert"]').text()).toBe('Historical data unavailable')
      expect(pending[1].signal.aborted).toBe(false)
      pending[1].resolve(response(2, 22))
      await flushPromises()
      expect(wrapper.findAllComponents({ name: 'lineChart' })[1].props('series')[0].data).toEqual([22])
    } finally {
      wrapper.unmount()
      log.mockRestore()
    }
  })

  it('does not start history queries after unmount while point tables are still loading', async () => {
    let finish!: (value: any) => void
    const responsePromise = new Promise(resolve => { finish = resolve })
    mocks.points.mockReturnValue(responsePromise)
    const wrapper = mount(Curves, options)
    await flushPromises()
    wrapper.unmount()
    finish({ success: true, data: { measurements: [{ measurement_id: 1, name: 'Voltage', unit: 'V' }] } })
    await flushPromises()
    expect(mocks.history).not.toHaveBeenCalled()
  })

  it('keeps legitimate point chunks for different devices running in parallel', async () => {
    mocks.points.mockResolvedValue({ success: true, data: { measurements:
      Array.from({ length: 21 }, (_, index) => ({ measurement_id: index + 1, name: `Point ${index + 1}`, unit: 'V' })),
    } })
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      expect(mocks.history.mock.calls.map(call => call[0].series.length)).toEqual([20, 1, 20, 1])
      expect(pending.every(item => !item.signal.aborted)).toBe(true)
      pending.forEach(item => item.resolve({ success: true, data: { series: [] } }))
      await flushPromises()
      expect(wrapper.findComponent({ name: 'LoadingBg' }).props('loading')).toBe(false)
    } finally { wrapper.unmount() }
  })

  it('expands a panel without a global resize, whole-page mask or changing existing chart props', async () => {
    const resize = vi.fn()
    window.addEventListener('resize', resize)
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      pending[0].resolve(response(1, 11))
      await flushPromises()
      const chart = wrapper.findComponent({ name: 'lineChart' })
      const series = chart.props('series')
      const xAxis = chart.props('xAxiosOption')
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      await new Promise(resolve => setTimeout(resolve, 80))
      expect(resize).not.toHaveBeenCalled()
      expect(wrapper.findComponent({ name: 'LoadingBg' }).props('loading')).toBe(false)
      expect(wrapper.findAll('[role="status"]')).toHaveLength(1)
      expect(wrapper.findAll('.curves__device-section')[0].attributes('aria-busy')).toBe('false')
      expect(chart.props('series')).toBe(series)
      expect(chart.props('xAxiosOption')).toBe(xAxis)
      pending[1].resolve(response(2, 22))
      await flushPromises()
      expect(chart.props('series')).toBe(series)
      expect(chart.props('xAxiosOption')).toBe(xAxis)
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      expect(chart.props('series')).toBe(series)
    } finally {
      wrapper.unmount()
      window.removeEventListener('resize', resize)
    }
  })

  it('keeps missing and non-numeric samples empty while preserving real zero', async () => {
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      pending[0].resolve({ success: true, data: { series: [{
        redis_key: 'inst:1:M', point_id: '1', count: 7,
        data: [null, undefined, NaN, Infinity, 'running', 0, 3.12345].map((value, index) => ({
          timestamp: `2026-10-30T00:00:0${index}Z`, value,
        })),
      }] } })
      await flushPromises()
      expect(wrapper.findComponent({ name: 'lineChart' }).props('series')[0].data)
        .toEqual([null, null, null, null, null, 0, 3.123])
    } finally { wrapper.unmount() }
  })

  it('removes devices and pending requests when they leave the topology while the page is open', async () => {
    const topology = mocks.topology()
    const devices = ref(topology.getTopologyDevices())
    mocks.topology.mockReturnValue({ ...topology, getTopologyDevices: () => devices.value })
    const wrapper = mount(Curves, options)
    try {
      await flushPromises()
      await wrapper.findAll('.curves__device-header')[1].trigger('click')
      devices.value = devices.value.slice(1)
      await flushPromises()
      expect(pending[0].signal.aborted).toBe(true)
      expect(wrapper.findAll('.curves__device-label').map(label => label.text())).toEqual(['Battery 2'])
      expect(mocks.history.mock.calls.slice(-1)[0][0].series[0].redis_key).toBe('inst:2:M')
      pending[0].resolve(response(1, 99))
      await flushPromises()
      expect(wrapper.findAll('.curves__device-label').map(label => label.text())).toEqual(['Battery 2'])
      devices.value = []
      await flushPromises()
      expect(wrapper.findAll('.curves__device-section')).toHaveLength(0)
      expect(pending.slice(1).every(item => item.signal.aborted)).toBe(true)
    } finally { wrapper.unmount() }
  })
})
