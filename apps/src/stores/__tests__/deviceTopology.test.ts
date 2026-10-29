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

  it('merges multiple converters under one Battery system', () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 3, name: 'Battery-01', product_name: 'Battery' },
      { id: 1, name: 'PCS-01', product_name: 'PCS' },
      { id: 2, name: 'PCS-02', product_name: 'PCS' },
      { id: 4, name: 'Hybrid-01', product_name: 'Hybrid_Inverter' },
    ]
    store.topology = { flow_json: {
      nodes: [
        { id: 'battery', data: { productName: 'Battery', label: 'Battery', instanceId: 3 } },
        { id: 'pcs1', data: { productName: 'PCS', label: 'PCS', instanceId: 1 } },
        { id: 'pcs2', data: { productName: 'PCS', label: 'PCS', instanceId: 2 } },
        { id: 'hybrid', data: { productName: 'Hybrid_Inverter', label: 'Hybrid', instanceId: 4 } },
      ],
      edges: [
        { source: 'battery', target: 'pcs1' },
        { source: 'battery', target: 'pcs2' },
        { source: 'battery', target: 'hybrid' },
      ],
    } } as StationTopology
    expect(store.batterySystems).toHaveLength(1)
    expect(store.selectedBatterySystem?.displayName).toBe('Battery-01')
    expect(store.selectedBatterySystem?.primaryInstanceIds).toEqual([3])
    expect(store.selectedBatterySystem?.converters).toHaveLength(3)
    expect(store.selectedBatterySystem?.converters.map((item) => item.productName).sort()).toEqual([
      'Hybrid_Inverter',
      'PCS',
      'PCS',
    ])
  })

  it('includes Battery-only systems when the battery instance exists', () => {
    const store = useDeviceTopologyStore()
    store.instances = [{ id: 3, name: 'Battery-01', product_name: 'Battery' }]
    store.topology = { flow_json: {
      nodes: [{ id: 'battery', data: { productName: 'Battery', label: 'Battery', instanceId: 3 } }],
      edges: [],
    } } as StationTopology
    expect(store.batterySystems).toHaveLength(1)
    expect(store.selectedBatterySystem?.converters).toEqual([])
  })
})
