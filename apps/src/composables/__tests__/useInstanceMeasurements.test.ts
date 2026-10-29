import { defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { getInstancePoints } from '@/api/devicesManagement'
import wsManager from '@/utils/websocket'
import { useInstanceMeasurements } from '../useInstanceMeasurements'

vi.mock('@/api/devicesManagement', () => ({ getInstancePoints: vi.fn() }))
vi.mock('@/utils/websocket', async () => ({ default: {
  status: (await import('vue')).ref('connected'),
  subscribe: vi.fn(() => 'subscription'), unsubscribe: vi.fn(),
} }))

describe('selected instance measurement values', () => {
  beforeEach(() => { vi.clearAllMocks(); wsManager.status.value = 'connected' })

  it('clears on selection/disconnect, ignores other sources, and drops stale definitions', async () => {
    const instanceId = ref<number | undefined>(1)
    let result!: ReturnType<typeof useInstanceMeasurements>
    let finishOld!: (value: any) => void
    vi.mocked(getInstancePoints).mockImplementation((id) => id === 1
      ? new Promise((resolve) => { finishOld = resolve })
      : Promise.resolve({ success: true, data: { measurements: [
        { measurement_id: 7, name: 'SOC', unit: '%' },
      ] } } as any))
    const wrapper = mount(defineComponent({ setup() {
      result = useInstanceMeasurements(() => instanceId.value)
      return () => h('div')
    } }))
    function push(source: string, id: number, values: Record<string, number>) {
      const calls = vi.mocked(wsManager.subscribe).mock.calls
      calls[calls.length - 1][1].onBatchDataUpdate!({ updates: [
        { source, channel_id: id, data_type: 'M', values },
      ] } as any)
    }
    push('inst', 1, { 3: 42 })
    expect(result.values.value).toEqual({ 3: 42 })
    instanceId.value = 2
    expect(result.values.value).toEqual({})
    await flushPromises()
    finishOld({ success: true, data: { measurements: [{ measurement_id: 3, name: 'Old' }] } })
    await flushPromises()
    expect(result.measurements.value[0].name).toBe('SOC')
    push('comsrv', 2, { 7: 99 })
    push('inst', 1, { 7: 88 })
    push(undefined as any, 2, { 7: 77 })
    expect(result.values.value).toEqual({})
    push('inst', 2, { 7: 0 })
    expect(result.values.value).toEqual({ 7: 0 })
    wsManager.status.value = 'disconnected'
    expect(result.values.value).toEqual({})
    wrapper.unmount()
    expect(wsManager.unsubscribe).toHaveBeenCalled()
  })

  it('reports a contract error for legacy object point tables', async () => {
    vi.mocked(getInstancePoints).mockResolvedValue({ success: true, data: {
      measurements: { p: { measurement_id: 7, name: 'SOC', unit: '%' } },
    } } as any)
    let result!: ReturnType<typeof useInstanceMeasurements>
    const wrapper = mount(defineComponent({ setup() {
      result = useInstanceMeasurements(() => 1)
      return () => h('div')
    } }))
    await flushPromises()
    expect(result.error.value).toBe(true)
    expect(result.measurements.value).toEqual([])
    wrapper.unmount()
  })
})
