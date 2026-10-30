import { defineComponent, h, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DieselOverview from '../DieselGenerator/DieselOverview.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import type { StationTopology } from '@/types/stationTopology'
import wsManager from '@/utils/websocket'

const mocks = vi.hoisted(() => ({ subscribe: vi.fn() }))
vi.mock('@/api/stationTopology', () => ({ getChannelBindings: vi.fn(), getStationTopology: vi.fn() }))
vi.mock('@/api/devicesManagement', () => ({ getAllInstances: vi.fn() }))
vi.mock('@/composables/useTopologySubscribe', () => ({ default: mocks.subscribe }))
vi.mock('@/utils/websocket', async () => ({ default: { status: (await import('vue')).ref('connected') } }))
const card = defineComponent({ props: ['title', 'value'], setup: props =>
  () => h('div', { 'data-title': props.title }, props.value) })

describe('Diesel overview selected group', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    wsManager.status.value = 'connected'
  })

  it('subscribes to one instance, clears on switching and ignores old device samples', async () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 1, name: 'Diesel-01', product_name: 'Diesel' },
      { id: 2, name: 'Diesel-02', product_name: 'Diesel' },
    ]
    store.topology = { flow_json: { nodes: [
      { id: 'd1', data: { productName: 'Diesel', instanceId: 1 } },
      { id: 'd2', data: { productName: 'Diesel', instanceId: 2 } },
    ], edges: [] } } as StationTopology
    const wrapper = mount(DieselOverview, { global: { stubs: { PVCard: card } } })
    try {
      const [ids, , handlers] = mocks.subscribe.mock.calls[0]
      const push = (id: number, value: number) => handlers.onBatchDataUpdate({ updates: [{
        source: 'inst', channel_id: id, data_type: 'M', values: { 1: value }, ts: { 1: 1 },
      }] })
      expect(ids()).toEqual([1])
      push(1, 10)
      await nextTick()
      expect(wrapper.find('[data-title="Power"]').text()).toBe('10')
      store.selectedDieselInstanceId = 2
      await nextTick()
      expect(ids()).toEqual([2])
      expect(wrapper.find('[data-title="Power"]').text()).toBe('-')
      push(1, 99)
      await nextTick()
      expect(wrapper.find('[data-title="Power"]').text()).toBe('-')
      push(2, 20)
      await nextTick()
      expect(wrapper.find('[data-title="Power"]').text()).toBe('20')
      store.topology!.flow_json.nodes = []
      await nextTick()
      expect(ids()).toEqual([])
      expect(wrapper.find('[data-title="Power"]').text()).toBe('-')
    } finally { wrapper.unmount() }
  })

  it.each(['disconnected', 'error', 'connecting'] as const)('clears all cards on %s and waits for data after reconnect', async (status) => {
    const store = useDeviceTopologyStore()
    store.topology = { flow_json: { nodes: [
      { id: 'd1', data: { productName: 'Diesel', instanceId: 1 } },
    ], edges: [] } } as StationTopology
    const wrapper = mount(DieselOverview, { global: { stubs: { PVCard: card } } })
    try {
      const handlers = mocks.subscribe.mock.calls[0][2]
      const push = (power: number) => handlers.onBatchDataUpdate({ updates: [{
        source: 'inst', channel_id: 1, data_type: 'M',
        values: { 1: power, 13: 40, 3: 230, 14: 35 }, ts: { 1: 1, 13: 1, 3: 1, 14: 1 },
      }] })
      const values = () => wrapper.findAll('[data-title]').map(item => item.text())
      push(10)
      await nextTick()
      expect(values()).toEqual(['10', '40', '230', '35'])
      wsManager.status.value = status
      await nextTick()
      expect(values()).toEqual(['-', '-', '-', '-'])
      push(99)
      await nextTick()
      expect(values()).toEqual(['-', '-', '-', '-'])
      push(99)
      wsManager.status.value = 'connected'
      await nextTick()
      expect(values()).toEqual(['-', '-', '-', '-'])
      push(0)
      await nextTick()
      expect(values()).toEqual(['0', '40', '230', '35'])
    } finally { wrapper.unmount() }
  })
})
