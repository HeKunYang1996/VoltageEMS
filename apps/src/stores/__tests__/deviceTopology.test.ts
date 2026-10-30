import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useDeviceTopologyStore } from '../deviceTopology'
import type { StationTopology } from '@/types/stationTopology'

vi.mock('@/api/stationTopology', () => ({ getChannelBindings: vi.fn(), getStationTopology: vi.fn() }))
vi.mock('@/api/devicesManagement', () => ({ getAllInstances: vi.fn() }))

describe('device topology product selection', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('lists only topology Meters, excluding registry-only and stale bound instances', () => {
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
    expect(store.getProductInstances('Meter')).toEqual([])
    store.topology = { flow_json: { nodes: [
      { id: 'meter', data: { productName: 'Meter', instanceId: 201 } },
    ], edges: [] } } as StationTopology
    expect(store.getProductInstances('Meter')).toEqual(store.instances.slice(0, 1))
    expect(store.getAllInstanceIds()).toEqual([201])
    expect(store.getAllChannelIds()).toEqual([4])
    store.topology!.flow_json.nodes[0].data.instanceId = null
    expect(store.getProductInstances('Meter')).toEqual([])
    expect(store.getAllChannelIds()).toEqual([])
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

  it('groups PV by inverter instance for both AC and Hybrid, including topology-only PV nodes', () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 1, name: 'AC-01', product_name: 'AC_Inverter' },
      { id: 2, name: 'Hybrid-01', product_name: 'Hybrid_Inverter' },
      { id: 10, name: 'PV-A', product_name: 'PV_Group' },
    ]
    store.topology = { flow_json: {
      nodes: [
        { id: 'ac', data: { productName: 'AC_Inverter', instanceId: 1 } },
        { id: 'h', data: { productName: 'Hybrid_Inverter', instanceId: 2 } },
        { id: 'p1', data: { productName: 'PV_Group', instanceId: 10 } },
        { id: 'p2', data: { productName: 'PV_Group', label: 'PV-B' } },
        { id: 'p3', data: { productName: 'PV_Group', label: 'PV-C' } },
      ], edges: [
        { source: 'p1', target: 'h' }, { source: 'p2', target: 'h' },
        { source: 'p3', target: 'ac' }, { source: 'p1', target: 'h' },
      ],
    } } as StationTopology
    expect(store.pvGroups).toHaveLength(2)
    const hybrid = store.pvGroups.find(group => group.relatedProductName === 'Hybrid_Inverter')!
    expect(hybrid.displayName).toBe('Hybrid-01')
    expect(hybrid.relatedInstanceIds).toEqual([2])
    expect(hybrid.primaryInstanceIds).toEqual([10])
    expect(hybrid.pvNodes.map(node => node.displayName)).toEqual(['PV-A', 'PV-B'])
    expect(store.getLogicalDeviceInstanceIds('pv')).toEqual([1, 2])
    store.selectedPvGroupId = hybrid.id
    store.topology!.flow_json.edges.reverse()
    store.topology!.flow_json.edges = store.topology!.flow_json.edges.filter(edge => edge.source !== 'p1')
    expect(store.selectedPvGroup?.id).toBe(hybrid.id)
    expect(store.selectedPvGroup?.pvNodes.map(node => node.nodeId)).toEqual(['p2'])
  })

  it('does not duplicate a shared Hybrid across Battery and PV systems or repeated inverter bindings', () => {
    const store = useDeviceTopologyStore()
    store.topology = { flow_json: {
      nodes: [
        { id: 'b', data: { productName: 'Battery', instanceId: 3 } },
        { id: 'h1', data: { productName: 'Hybrid_Inverter', instanceId: 1 } },
        { id: 'h1-alias', data: { productName: 'Hybrid_Inverter', instanceId: 1 } },
        { id: 'h2', data: { productName: 'Hybrid_Inverter', instanceId: 2 } },
        { id: 'p1', data: { productName: 'PV_Group', instanceId: 10 } },
        { id: 'p2', data: { productName: 'PV_Group', instanceId: 11 } },
      ], edges: [
        { source: 'b', target: 'h1' }, { source: 'b', target: 'h2' },
        { source: 'p1', target: 'h1' }, { source: 'p2', target: 'h1-alias' },
      ],
    } } as StationTopology
    expect(store.batterySystems).toHaveLength(1)
    expect(store.pvGroups.map(group => group.relatedInstanceIds)).toEqual([[1], [2]])
    expect(store.pvGroups[0].primaryInstanceIds).toEqual([10, 11])
    expect(store.pvGroups[1].pvNodes).toEqual([])
    expect(store.pvGroups.every(group => !group.instanceIds.includes(3))).toBe(true)
  })

  it('resolves PV instances and names from existing channel bindings without requiring PV channels', () => {
    const store = useDeviceTopologyStore()
    store.topology = { flow_json: { nodes: [
      { id: 'h', data: { productName: 'Hybrid_Inverter' } },
      { id: 'p', data: { productName: 'PV_Group' } },
      { id: 'empty', data: { productName: 'AC_Inverter' } },
    ], edges: [{ source: 'h', target: 'p' }] } } as StationTopology
    store.bindings = [
      { nodeId: 'h', productName: 'Hybrid_Inverter', instances: [{ instanceId: 1, instanceName: 'Hybrid', channelIds: [5] }] },
      { nodeId: 'p', productName: 'PV_Group', instances: [{ instanceId: 2, instanceName: 'PV', channelIds: [] }] },
    ]
    expect(store.pvGroups).toHaveLength(1)
    expect(store.selectedPvGroup?.displayName).toBe('Hybrid')
    expect(store.selectedPvGroup?.pvNodes[0]).toMatchObject({ instanceIds: [2], channelIds: [], displayName: 'PV' })
  })

  it('creates one Diesel group per instance and resets a removed selection', () => {
    const store = useDeviceTopologyStore()
    store.instances = [
      { id: 1, name: 'Diesel-01', product_name: 'Diesel' },
      { id: 2, name: 'Diesel-02', product_name: 'Diesel' },
      { id: 3, name: 'Not used', product_name: 'Diesel' },
    ]
    store.topology = { flow_json: { nodes: [
      { id: 'd1', data: { productName: 'Diesel', instanceId: 1 } },
      { id: 'd2', data: { productName: 'Diesel', instanceId: 2 } },
    ], edges: [] } } as StationTopology
    expect(store.dieselGroups.map(group => group.displayName)).toEqual(['Diesel-01', 'Diesel-02'])
    expect(store.selectedDieselId).toBe(1)
    store.selectedDieselInstanceId = 2
    expect(store.selectedDieselId).toBe(2)
    store.topology!.flow_json.nodes = store.topology!.flow_json.nodes.slice(0, 1)
    expect(store.selectedDieselId).toBe(1)
    store.topology!.flow_json.nodes = []
    expect(store.selectedDieselId).toBeNull()
  })

  it('keeps current topology bindings authoritative across PV, Battery and legacy channel bindings', () => {
    const store = useDeviceTopologyStore()
    store.topology = { flow_json: { nodes: [
      { id: 'h', data: { productName: 'Hybrid_Inverter', instanceId: 1 } },
      { id: 'b', data: { productName: 'Battery', instanceId: 2 } },
      { id: 'legacy', data: { productName: 'Diesel' } },
      { id: 'legacy-meter', data: { label: 'Meter' } },
    ], edges: [{ source: 'h', target: 'b' }] } } as StationTopology
    store.bindings = [
      { nodeId: 'h', productName: 'Hybrid_Inverter', instances: [{ instanceId: 99, instanceName: 'Old Hybrid', channelIds: [99] }] },
      { nodeId: 'b', productName: 'Battery', instances: [{ instanceId: 88, instanceName: 'Old Battery', channelIds: [88] }] },
      { nodeId: 'legacy', productName: 'Diesel', instances: [{ instanceId: 3, instanceName: 'Diesel', channelIds: [3] }] },
      { nodeId: 'deleted', productName: 'Diesel', instances: [{ instanceId: 4, instanceName: 'Deleted Diesel', channelIds: [4] }] },
      { nodeId: 'legacy-meter', productName: 'Diesel', instances: [{ instanceId: 5, instanceName: 'Wrong product', channelIds: [5] }] },
    ]
    expect(store.getAllInstanceIds()).toEqual([1, 2, 3])
    expect(store.getAllChannelIds()).toEqual([3])
    expect(store.getProductInstances('Meter')).toEqual([])
    expect(store.pvGroups.map(group => group.relatedInstanceIds)).toEqual([[1]])
    expect(store.selectedBatterySystem?.primaryInstanceIds).toEqual([2])
    expect(store.selectedBatterySystem?.converters[0].instanceIds).toEqual([1])
    store.topology!.flow_json.nodes = store.topology!.flow_json.nodes.filter(node => node.id !== 'h')
    expect(store.pvGroups).toEqual([])
    expect(store.selectedBatterySystem?.converters).toEqual([])
  })
})
