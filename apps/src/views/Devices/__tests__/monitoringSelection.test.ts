import { defineComponent, h, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import BatteryValue from '../DeviceBattery/BatteryValue.vue'
import DeviceMeter from '../DeviceMeter/index.vue'
import PVValueMonitoring from '../DevicesPV/PVValueMonitoring.vue'
import DieselValueMonitoring from '../DieselGenerator/DieselValueMonitoring.vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import type { StationTopology } from '@/types/stationTopology'

vi.mock('@/api/stationTopology', () => ({ getChannelBindings: vi.fn(), getStationTopology: vi.fn() }))
vi.mock('@/api/devicesManagement', () => ({ getAllInstances: vi.fn() }))
vi.mock('element-plus/es/components/base/style/css', () => ({}))
vi.mock('element-plus/es/components/tabs/style/css', () => ({}))
vi.mock('element-plus/es/components/tab-pane/style/css', () => ({}))
vi.mock('element-plus/es/components/select/style/css', () => ({}))
vi.mock('element-plus/es/components/option/style/css', () => ({}))
vi.mock('element-plus/es/components/empty/style/css', () => ({}))
vi.mock('@/components/device/InstanceValueMonitoring.vue', () => ({ default: defineComponent({
  name: 'InstanceValueMonitoring', props: ['instanceIds'],
  setup: (props) => () => h('div', { 'data-instances': props.instanceIds.join(',') }),
}) }))

const stubs = {
  ElTabs: { name: 'ElTabs', props: ['modelValue'], template: '<div><slot /></div>' },
  ElTabPane: defineComponent({ props: ['label', 'name'], setup: (props, { slots }) =>
    () => h('section', { 'data-tab': props.name }, [h('h2', props.label), slots.default?.()]) }),
  ElSelect: { name: 'ElSelect', props: ['modelValue'], template: '<select><slot /></select>' },
  ElOption: { props: ['label'], template: '<option>{{ label }}</option>' },
}

describe('device monitoring page selection', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('names converter tabs by product and binds one instance at a time', async () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 3, name: 'Battery-01', product_name: 'Battery' },
      { id: 1, name: 'Hybrid-01', product_name: 'Hybrid_Inverter' },
    ]
    store.topology = { flow_json: {
      nodes: [
        { id: 'b', data: { label: 'Battery', productName: 'Battery', instanceId: 3 } },
        { id: 'c', data: { label: 'Converter', productName: 'Hybrid_Inverter', instanceId: 1 } },
      ], edges: [{ source: 'b', target: 'c' }],
    } } as StationTopology
    const wrapper = mount(BatteryValue, { global: { stubs } })
    expect(wrapper.findAll('h2').map((item) => item.text())).toEqual(['Battery', 'Hybrid Inverter'])
    expect(wrapper.findAll('[data-instances]').map((item) => item.attributes('data-instances')))
      .toEqual(['3'])
    wrapper.findComponent({ name: 'ElTabs' }).vm.$emit('update:modelValue', 'Hybrid_Inverter')
    await nextTick()
    expect(wrapper.findAll('[data-instances]').map((item) => item.attributes('data-instances')))
      .toEqual(['1'])
    store.instances = [
      { id: 3, name: 'Battery-01', product_name: 'Battery' },
      { id: 2, name: 'PCS-01', product_name: 'PCS' },
    ]
    store.topology = { flow_json: {
      nodes: [
        { id: 'b', data: { label: 'Battery', productName: 'Battery', instanceId: 3 } },
        { id: 'c', data: { label: 'Converter', productName: 'PCS', instanceId: 2 } },
      ], edges: [{ source: 'b', target: 'c' }],
    } } as StationTopology
    await nextTick()
    expect(wrapper.findAll('h2').map((item) => item.text())).toEqual(['Battery', 'PCS'])
    wrapper.findComponent({ name: 'ElTabs' }).vm.$emit('update:modelValue', 'PCS')
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('2')
    wrapper.findComponent({ name: 'ElTabs' }).vm.$emit('update:modelValue', 'battery')
    await nextTick()
    expect(wrapper.findAll('[data-instances]').map((item) => item.attributes('data-instances')))
      .toEqual(['3'])
    wrapper.unmount()
  })

  it('shows PCS and Hybrid tabs with counts when multiple converters exist', async () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 3, name: 'Battery-01', product_name: 'Battery' },
      { id: 1, name: 'PCS-01', product_name: 'PCS' },
      { id: 2, name: 'PCS-02', product_name: 'PCS' },
      { id: 4, name: 'Hybrid-01', product_name: 'Hybrid_Inverter' },
    ]
    store.topology = { flow_json: {
      nodes: [
        { id: 'b', data: { productName: 'Battery', instanceId: 3 } },
        { id: 'p1', data: { productName: 'PCS', instanceId: 1 } },
        { id: 'p2', data: { productName: 'PCS', instanceId: 2 } },
        { id: 'h1', data: { productName: 'Hybrid_Inverter', instanceId: 4 } },
      ],
      edges: [
        { source: 'b', target: 'p1' },
        { source: 'b', target: 'p2' },
        { source: 'b', target: 'h1' },
      ],
    } } as StationTopology
    const wrapper = mount(BatteryValue, { global: { stubs } })
    expect(wrapper.findAll('h2').map((item) => item.text())).toEqual([
      'Battery',
      'PCS (2)',
      'Hybrid Inverter',
    ])
    wrapper.findComponent({ name: 'ElTabs' }).vm.$emit('update:modelValue', 'PCS')
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('1')
    wrapper.findComponent({ name: 'ElSelect' }).vm.$emit('update:modelValue', 2)
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('2')
    wrapper.unmount()
  })

  it('shows only topology Meters and drops removed nodes even if their instances still exist', async () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 201, name: 'Hybrid Meter', product_name: 'Meter' },
      { id: 202, name: 'Grid Meter', product_name: 'Meter' },
      { id: 203, name: 'Unused Meter', product_name: 'Meter' },
    ]
    store.topology = { flow_json: { nodes: [
      { id: 'm1', data: { productName: 'Meter', instanceId: 201 } },
      { id: 'm2', data: { productName: 'Meter', instanceId: 202 } },
    ], edges: [] } } as StationTopology
    const wrapper = mount(DeviceMeter, { global: { stubs } })
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('201')
    expect(wrapper.text()).toContain('Hybrid Meter')
    expect(wrapper.text()).not.toContain('Unused Meter')
    wrapper.findComponent({ name: 'ElSelect' }).vm.$emit('update:modelValue', 202)
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('202')
    store.topology!.flow_json.nodes = store.topology!.flow_json.nodes.slice(0, 1)
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('201')
    store.topology!.flow_json.nodes = []
    await nextTick()
    expect(wrapper.find('[data-instances]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('switches between the inverter and bound PV groups, and resets on system changes', async () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 1, name: 'Hybrid-01', product_name: 'Hybrid_Inverter' },
      { id: 2, name: 'AC-01', product_name: 'AC_Inverter' },
      { id: 10, name: 'PV-A', product_name: 'PV_Group' },
      { id: 11, name: 'PV-B', product_name: 'PV_Group' },
    ]
    store.topology = { flow_json: {
      nodes: [
        { id: 'h', data: { productName: 'Hybrid_Inverter', instanceId: 1 } },
        { id: 'ac', data: { productName: 'AC_Inverter', instanceId: 2 } },
        { id: 'p1', data: { productName: 'PV_Group', instanceId: 10 } },
        { id: 'p2', data: { productName: 'PV_Group', instanceId: 11 } },
        { id: 'p3', data: { productName: 'PV_Group', label: 'Display-only PV' } },
      ], edges: [
        { source: 'h', target: 'p1' }, { source: 'h', target: 'p2' }, { source: 'h', target: 'p3' },
      ],
    } } as StationTopology
    const wrapper = mount(PVValueMonitoring, { global: { stubs } })
    expect(wrapper.findAll('h2').map(item => item.text())).toEqual(['Hybrid Inverter', 'PV-A', 'PV-B'])
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('1')
    const tabs = wrapper.findComponent({ name: 'ElTabs' })
    tabs.vm.$emit('update:modelValue', 'pv:10')
    await nextTick()
    expect(wrapper.findAll('[data-instances]').map(item => item.attributes('data-instances'))).toEqual(['10'])
    tabs.vm.$emit('update:modelValue', 'pv:11')
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('11')
    store.topology!.flow_json.edges = store.topology!.flow_json.edges.filter(edge => edge.target !== 'p2')
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('1')
    tabs.vm.$emit('update:modelValue', 'pv:10')
    await nextTick()
    store.selectedPvGroupId = 'pv:inverter:2'
    await nextTick()
    expect(wrapper.findAll('h2').map(item => item.text())).toEqual(['AC Inverter'])
    expect(wrapper.findAll('[data-instances]').map(item => item.attributes('data-instances'))).toEqual(['2'])
    wrapper.unmount()
  })

  it('binds Diesel monitoring to exactly the selected group and clears a removed device', async () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 1, name: 'Diesel-01', product_name: 'Diesel' },
      { id: 2, name: 'Diesel-02', product_name: 'Diesel' },
    ]
    store.topology = { flow_json: { nodes: [
      { id: 'd1', data: { productName: 'Diesel', instanceId: 1 } },
      { id: 'd2', data: { productName: 'Diesel', instanceId: 2 } },
    ], edges: [] } } as StationTopology
    const wrapper = mount(DieselValueMonitoring, { global: { stubs } })
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('1')
    store.selectedDieselInstanceId = 2
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('2')
    store.topology!.flow_json.nodes = []
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('')
    wrapper.unmount()
  })
})
