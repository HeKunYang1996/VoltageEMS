import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useDeviceTopologyStore } from '../deviceTopology'
import type { StationTopology } from '@/types/stationTopology'

vi.mock('@/api/stationTopology', () => ({ getChannelBindings: vi.fn(), getStationTopology: vi.fn() }))
vi.mock('@/api/devicesManagement', () => ({ getAllInstances: vi.fn() }))

describe('device topology product selection', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('lists actual Meters without requiring a Load connection or a topology node', () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 201, name: 'Inverter Meter', product_name: 'Meter' },
      { id: 202, name: 'Unconnected Meter', product_name: 'Meter' },
      { id: 101, name: 'Hybrid', product_name: 'Hybrid_Inverter' },
    ]
    store.bindings = [{ nodeId: 'meter', productName: 'Meter', instances: [
      { instanceId: 201, instanceName: 'Old Name', channelIds: [4] },
      { instanceId: 999, instanceName: 'Removed Meter', channelIds: [9] },
    ] }]
    expect(store.getProductInstances('Meter')).toEqual(store.instances.slice(0, 2))
  })

  it.each(['Hybrid_Inverter', 'PCS'])('keeps the actual %s product on a Battery group', (product) => {
    const store = useDeviceTopologyStore()
    store.topology = { flow_json: {
      nodes: [
        { id: 'battery', data: { productName: 'Battery', label: 'Battery', instanceId: 3 } },
        { id: 'converter', data: { productName: product, label: 'Converter', instanceId: 1 } },
      ], edges: [{ source: 'battery', target: 'converter' }],
    } } as StationTopology
    expect(store.selectedBatteryGroup?.relatedProductName).toBe(product)
    expect(store.selectedBatteryGroup?.relatedInstanceIds).toEqual([1])
    expect(store.selectedBatteryGroup?.primaryInstanceIds).toEqual([3])
  })
})
