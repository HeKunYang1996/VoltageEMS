import { defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getInstancePoints } from '@/api/devicesManagement'
import { getPointsTables } from '@/api/channelsManagement'
import wsManager from '@/utils/websocket'
import { useInstanceChannelMonitoring } from '../useInstanceChannelMonitoring'

vi.mock('@/api/devicesManagement', () => ({ getInstancePoints: vi.fn() }))
vi.mock('@/api/channelsManagement', () => ({ getPointsTables: vi.fn() }))
vi.mock('@/utils/websocket', async () => ({ default: {
  status: (await import('vue')).ref('connected'),
  subscribe: vi.fn(() => 'subscription'), unsubscribe: vi.fn(),
} }))

const wrappers: ReturnType<typeof mount>[] = []
function setup(ids = [1]) {
  const instanceIds = ref(ids)
  let result!: ReturnType<typeof useInstanceChannelMonitoring>
  wrappers.push(mount(defineComponent({ setup() {
    result = useInstanceChannelMonitoring(() => instanceIds.value)
    return () => h('div')
  } })))
  return { instanceIds, get state() { return result } }
}
function routing(channelId: number, enabled = true) {
  return { measurement_id: 2, name: 'PV Input Power', unit: 'kW', routing: {
    channel_id: channelId, channel_type: 'T', channel_point_id: 42, enabled,
  } }
}
function response(measurements: unknown) {
  return { success: true, data: { measurements, actions: [], properties: [] } } as any
}
function table(name: string) {
  return { success: true, data: {
    telemetry: [{ point_id: 42, signal_name: name, unit: 'kW' },
      { point_id: 99, signal_name: 'Native device detail', unit: 'V' }],
    signal: [{ point_id: 3, signal_name: 'Operating Status' }],
  } } as any
}
function push(updates: unknown[]) {
  const calls = vi.mocked(wsManager.subscribe).mock.calls
  calls[calls.length - 1][1].onBatchDataUpdate!({ updates } as any)
}

describe('actual device channel monitoring', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    wsManager.status.value = 'connected'
    vi.mocked(getInstancePoints).mockResolvedValue(response([routing(7), routing(8, false)]))
    vi.mocked(getPointsTables).mockResolvedValue(table('Hybrid Power'))
  })
  afterEach(() => wrappers.splice(0).forEach((wrapper) => wrapper.unmount()))

  it('loads the whole actual channel, including native points outside the product M list', async () => {
    const { state } = setup()
    await flushPromises()
    expect(state.channelIds.value).toEqual([7])
    expect(getPointsTables).toHaveBeenCalledWith(7)
    expect(state.leftTableData.value.map((point) => point.pointId)).toEqual([42, 99])
    push([
      { source: 'comsrv', channel_id: 7, data_type: 'T', values: { 42: 0, 99: 230 }, ts: { 42: 1 } },
      { source: 'comsrv', channel_id: 7, data_type: 'S', values: { 3: 0 } },
    ])
    expect(state.leftTableData.value.map((point) => point.value)).toEqual([0, 230])
    expect(state.rightTableData.value[0].status).toBe(0)
    push([{ source: 'inst', channel_id: 7, data_type: 'T', values: { 42: 999 } }])
    expect(state.leftTableData.value[0].value).toBe(0)
    push([{ channel_id: 7, data_type: 'T', values: { 42: 999 } }])
    expect(state.leftTableData.value[0].value).toBe(0)
    push([{ source: 'comsrv', channel_id: 7, data_type: 'T', values: { 42: 10 } }])
    expect(state.leftTableData.value[1].value).toBeNull()
    wsManager.status.value = 'disconnected'
    expect(state.leftTableData.value[0].value).toBeNull()
    expect(state.rightTableData.value[0].status).toBeNull()
  })

  it('rejects the retired object point-table format', async () => {
    vi.mocked(getInstancePoints).mockResolvedValue(response({ m1: routing(7) }))
    const { state } = setup()
    await flushPromises()
    expect(state.error.value).toBe(true)
    expect(state.channelIds.value).toEqual([])
    expect(getPointsTables).not.toHaveBeenCalled()
  })

  it('switches between a device’s mapped channels and clears old values immediately', async () => {
    vi.mocked(getInstancePoints).mockResolvedValue(response([routing(7), routing(9)]))
    const { state } = setup()
    await flushPromises()
    push([{ source: 'comsrv', channel_id: 7, data_type: 'T', values: { 42: 123 } }])
    state.selectedChannelId.value = 9
    expect(state.leftTableData.value).toEqual([])
    await flushPromises()
    expect(state.leftTableData.value[0].value).toBeNull()
    expect(vi.mocked(wsManager.subscribe).mock.calls.slice(-1)[0][0].channels).toEqual([9])
  })

  it('does not let a late point-table response overwrite a newly selected device', async () => {
    let finishOld!: (value: any) => void
    vi.mocked(getPointsTables).mockImplementation((id) => id === 7
      ? new Promise((resolve) => { finishOld = resolve }) : Promise.resolve(table('Meter Power')))
    const host = setup()
    await flushPromises()
    vi.mocked(getInstancePoints).mockResolvedValue(response([routing(9)]))
    host.instanceIds.value = [2]
    await flushPromises()
    expect(host.state.selectedChannelId.value).toBe(9)
    expect(host.state.leftTableData.value[0].name).toBe('Meter Power')
    finishOld(table('Old Hybrid'))
    await flushPromises()
    expect(host.state.leftTableData.value[0].name).toBe('Meter Power')
    push([{ source: 'comsrv', channel_id: 7, data_type: 'T', values: { 42: 999 } }])
    expect(host.state.leftTableData.value[0].value).toBeNull()
  })

  it('clears the previous device when the next device has no active routing', async () => {
    const host = setup()
    await flushPromises()
    vi.mocked(getInstancePoints).mockResolvedValue(response([routing(7, false)]))
    host.instanceIds.value = [2]
    await flushPromises()
    expect(host.state.channelIds.value).toEqual([])
    expect(host.state.leftTableData.value).toEqual([])
    expect(host.state.rightTableData.value).toEqual([])
  })

  it('discards late routing responses and reports a loading failure', async () => {
    let finishOld!: (value: any) => void
    vi.mocked(getInstancePoints).mockImplementation((id) => id === 1
      ? new Promise((resolve) => { finishOld = resolve }) : Promise.reject(new Error('Offline')))
    const host = setup()
    host.instanceIds.value = [2]
    await flushPromises()
    finishOld(response([routing(7)]))
    await flushPromises()
    expect(host.state.error.value).toBe(true)
    expect(host.state.channelIds.value).toEqual([])
    expect(getPointsTables).not.toHaveBeenCalled()
  })
})
