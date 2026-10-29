import { defineComponent, h, nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import BatteryValue from '../DeviceBattery/BatteryValue.vue'
import DeviceMeter from '../DeviceMeter/index.vue'
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
  ElTabPane: defineComponent({ props: ['label'], setup: (props, { slots }) =>
    () => h('section', [h('h2', props.label), slots.default?.()]) }),
  ElSelect: { name: 'ElSelect', props: ['modelValue'], template: '<select><slot /></select>' },
  ElOption: { props: ['label'], template: '<option>{{ label }}</option>' },
}

describe('device monitoring page selection', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('names and binds the converter tab to the actual Hybrid or PCS device', async () => {
    const store = useDeviceTopologyStore()
    const topology = (productName: string, instanceId: number) => ({ flow_json: {
      nodes: [
        { id: 'b', data: { label: 'Battery', productName: 'Battery', instanceId: 3 } },
        { id: 'c', data: { label: 'Converter', productName, instanceId } },
      ], edges: [{ source: 'b', target: 'c' }],
    } }) as StationTopology
    store.topology = topology('Hybrid_Inverter', 1)
    const wrapper = mount(BatteryValue, { global: { stubs } })
    expect(wrapper.findAll('h2').map((item) => item.text())).toEqual(['Battery', 'Hybrid Inverter'])
    expect(wrapper.findAll('[data-instances]').map((item) => item.attributes('data-instances')))
      .toEqual(['3'])
    wrapper.findComponent({ name: 'ElTabs' }).vm.$emit('update:modelValue', 'converter')
    await nextTick()
    expect(wrapper.findAll('[data-instances]').map((item) => item.attributes('data-instances')))
      .toEqual(['1'])
    store.topology = topology('PCS', 2)
    await nextTick()
    expect(wrapper.findAll('h2').map((item) => item.text())).toEqual(['Battery', 'PCS'])
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('2')
    wrapper.findComponent({ name: 'ElTabs' }).vm.$emit('update:modelValue', 'battery')
    await nextTick()
    expect(wrapper.findAll('[data-instances]').map((item) => item.attributes('data-instances')))
      .toEqual(['3'])
    wrapper.unmount()
  })

  it('shows and switches actual Meters even when no Load topology exists', async () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 201, name: 'Hybrid Meter', product_name: 'Meter' },
      { id: 202, name: 'Grid Meter', product_name: 'Meter' },
    ]
    const wrapper = mount(DeviceMeter, { global: { stubs } })
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('201')
    expect(wrapper.text()).toContain('Hybrid Meter')
    wrapper.findComponent({ name: 'ElSelect' }).vm.$emit('update:modelValue', 202)
    await nextTick()
    expect(wrapper.find('[data-instances]').attributes('data-instances')).toBe('202')
    store.instances = []
    await nextTick()
    expect(wrapper.find('[data-instances]').exists()).toBe(false)
    wrapper.unmount()
  })
})
