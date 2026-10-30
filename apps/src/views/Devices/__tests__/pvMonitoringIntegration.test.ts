import { defineComponent, h } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import PVValueMonitoring from '../DevicesPV/PVValueMonitoring.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import type { StationTopology } from '@/types/stationTopology'
import wsManager from '@/utils/websocket'

const mocks = vi.hoisted(() => ({ points: vi.fn(), tables: vi.fn() }))
vi.mock('@/api/stationTopology', () => ({ getChannelBindings: vi.fn(), getStationTopology: vi.fn() }))
vi.mock('@/api/devicesManagement', () => ({ getAllInstances: vi.fn(), getInstancePoints: mocks.points }))
vi.mock('@/api/channelsManagement', () => ({ getPointsTables: mocks.tables }))
vi.mock('@/utils/websocket', async () => ({ default: {
  status: (await import('vue')).ref('connected'), subscribe: vi.fn(), unsubscribe: vi.fn(),
} }))
vi.mock('element-plus/es/components/base/style/css', () => ({}))
vi.mock('element-plus/es/components/tabs/style/css', () => ({}))
vi.mock('element-plus/es/components/tab-pane/style/css', () => ({}))
vi.mock('element-plus/es/components/select/style/css', () => ({}))
vi.mock('element-plus/es/components/option/style/css', () => ({}))
vi.mock('element-plus/es/components/empty/style/css', () => ({}))
vi.mock('@/components/device/DeviceMonitoringTable.vue', () => ({ default: defineComponent({
  name: 'DeviceMonitoringTable', props: ['leftTableData', 'rightTableData'],
  setup: props => () => h('div', [h('header', 'Name Value Unit Update Time Status'),
    ...props.leftTableData.map((item: { name: string; value: number | null }) => h('p', `${item.name}: ${item.value ?? '--'}`)),
  ]),
}) }))
const container = defineComponent({ setup: (_, { slots }) => () => h('div', slots.default?.()) })

describe('PV monitoring subscription integration', () => {
  beforeEach(() => { setActivePinia(createPinia()); vi.clearAllMocks() })

  it('owns only the active device subscription and leaves headers when a bound PV has no channel', async () => {
    const active = new Map<string, { config: any; handlers: any }>()
    let sequence = 0
    vi.mocked(wsManager.subscribe).mockImplementation((config, handlers) => {
      const id = `subscription-${++sequence}`
      active.set(id, { config, handlers })
      return id
    })
    vi.mocked(wsManager.unsubscribe).mockImplementation(id => { active.delete(id) })
    mocks.points.mockImplementation((id: number) => Promise.resolve({ success: true, data: {
      measurements: id === 20 ? [] : [{ measurement_id: 1, name: 'Power', unit: 'kW', routing: {
        enabled: true, channel_id: id * 10, channel_point_id: 1, channel_type: 'T',
      } }],
    } }))
    mocks.tables.mockImplementation((id: number) => Promise.resolve({ success: true, data: {
      telemetry: [{ point_id: 1, signal_name: `Channel ${id} power`, unit: 'kW' }], signal: [],
    } }))
    const store = useDeviceTopologyStore()
    store.topology = { flow_json: {
      nodes: [
        { id: 'h', data: { productName: 'Hybrid_Inverter', instanceId: 1 } },
        { id: 'p1', data: { productName: 'PV_Group', instanceId: 10 } },
        { id: 'p2', data: { productName: 'PV_Group', instanceId: 20 } },
      ], edges: [{ source: 'h', target: 'p1' }, { source: 'h', target: 'p2' }],
    } } as StationTopology
    const wrapper = mount(PVValueMonitoring, { global: { stubs: {
      LoadingBg: container,
      ElTabs: { name: 'ElTabs', props: ['modelValue'], template: '<div><slot /></div>' },
      ElTabPane: container, ElSelect: true, ElOption: true,
    } } })
    try {
      await flushPromises()
      expect([...active.values()].map(item => item.config.channels)).toEqual([[10]])
      expect(wrapper.text()).toContain('Channel 10 power')
      const tabs = wrapper.findComponent({ name: 'ElTabs' })
      tabs.vm.$emit('update:modelValue', 'pv:10')
      await flushPromises()
      expect([...active.values()].map(item => item.config.channels)).toEqual([[100]])
      expect(wrapper.text()).not.toContain('Channel 10 power')
      const handler = [...active.values()][0].handlers.onBatchDataUpdate
      handler({ updates: [{ source: 'comsrv', channel_id: 10, data_type: 'T', values: { 1: 99 }, ts: { 1: 1 } }] })
      await flushPromises()
      expect(wrapper.find('p').text()).toBe('Channel 100 power: --')
      handler({ updates: [{ source: 'comsrv', channel_id: 100, data_type: 'T', values: { 1: 0 }, ts: { 1: 1 } }] })
      await flushPromises()
      expect(wrapper.find('p').text()).toBe('Channel 100 power: 0')
      tabs.vm.$emit('update:modelValue', 'pv:20')
      await flushPromises()
      expect(active.size).toBe(0)
      expect(wrapper.find('header').exists()).toBe(true)
      expect(wrapper.findAll('p')).toHaveLength(0)
      expect(mocks.tables.mock.calls.map(call => call[0])).toEqual([10, 100])
    } finally { wrapper.unmount() }
    expect(active.size).toBe(0)
  })
})
