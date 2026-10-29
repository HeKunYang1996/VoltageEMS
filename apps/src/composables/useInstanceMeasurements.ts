import { onScopeDispose, ref, watch } from 'vue'
import { getInstancePoints } from '@/api/devicesManagement'
import type { InstanceMeasurementItem } from '@/types/deviceConfiguration'
import useTopologySubscribe from '@/composables/useTopologySubscribe'
import wsManager from '@/utils/websocket'

/** Product measurement definitions and values for exactly the selected instance. */
export function useInstanceMeasurements(getInstanceId: () => number | undefined) {
  const measurements = ref<InstanceMeasurementItem[]>([])
  const values = ref<Record<string, number | string | null>>({})
  const loading = ref(false)
  const error = ref(false)
  let version = 0

  watch(getInstanceId, async (instanceId) => {
    const current = ++version
    measurements.value = []
    values.value = {}
    error.value = false
    loading.value = instanceId !== undefined
    if (instanceId === undefined) return
    try {
      const response = await getInstancePoints(instanceId)
      if (current !== version) return
      if (!response.success || !Array.isArray(response.data?.measurements))
        throw new Error('Invalid instance points')
      measurements.value = response.data.measurements
    } catch {
      if (current === version) error.value = true
    } finally {
      if (current === version) loading.value = false
    }
  }, { immediate: true, flush: 'sync' })

  function applyUpdate(update: {
    source?: string
    channel_id: number
    data_type: string
    values: Record<string, number | string | null>
  }) {
    if (wsManager.status.value !== 'connected' || update.channel_id !== getInstanceId() ||
      update.data_type !== 'M' || update.source !== 'inst') return
    values.value = update.values
  }

  useTopologySubscribe(
    () => {
      const id = getInstanceId()
      return id === undefined ? [] : [id]
    },
    { source: 'inst', dataTypes: ['M'], interval: 1000 },
    {
      onDataUpdate: applyUpdate,
      onBatchDataUpdate: (data) => { if ('updates' in data) data.updates.forEach(applyUpdate) },
    },
  )
  watch(wsManager.status, (status) => {
    if (status !== 'connected') values.value = {}
  }, { flush: 'sync' })
  onScopeDispose(() => { version++ })

  return { measurements, values, loading, error }
}
