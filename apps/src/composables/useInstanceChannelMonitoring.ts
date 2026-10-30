import { computed, onScopeDispose, ref, watch } from 'vue'
import { getInstancePoints } from '@/api/devicesManagement'
import { getPointsTables } from '@/api/channelsManagement'
import type { LeftTableItem, RightTableItem } from '@/types/deviceMonitoring'
import useTopologySubscribe from '@/composables/useTopologySubscribe'
import wsManager from '@/utils/websocket'
import { parseSampleTimestamp } from '@/utils/common'

/** Full native T/S point tables, resolved from the selected device's active routing. */
export function useInstanceChannelMonitoring(getInstanceIds: () => number[]) {
  const channelIds = ref<number[]>([])
  const selectedChannelId = ref<number>()
  const leftTableData = ref<LeftTableItem[]>([])
  const rightTableData = ref<RightTableItem[]>([])
  const routingLoading = ref(false)
  const tableLoading = ref(false)
  const routingError = ref(false)
  const tableError = ref(false)
  let routingVersion = 0
  let tableVersion = 0

  watch(() => getInstanceIds().join(','), async () => {
    const current = ++routingVersion
    channelIds.value = []
    selectedChannelId.value = undefined
    routingError.value = false
    const ids = getInstanceIds()
    routingLoading.value = ids.length > 0
    if (!ids.length) return
    try {
      const responses = await Promise.all(ids.map(getInstancePoints))
      if (current !== routingVersion) return
      if (responses.some((response) => !response.success || !Array.isArray(response.data?.measurements)))
        throw new Error('Invalid device routing')
      const channels = responses.flatMap((response) =>
        response.data.measurements.flatMap(({ routing }) =>
          routing?.enabled && routing.channel_id != null && routing.channel_point_id != null &&
          (routing.channel_type === 'T' || routing.channel_type === 'S')
            ? [routing.channel_id] : [],
        ),
      )
      channelIds.value = [...new Set(channels)]
      selectedChannelId.value = channelIds.value[0]
    } catch {
      if (current === routingVersion) routingError.value = true
    } finally {
      if (current === routingVersion) routingLoading.value = false
    }
  }, { immediate: true, flush: 'sync' })

  watch(selectedChannelId, async (channelId) => {
    const current = ++tableVersion
    leftTableData.value = []
    rightTableData.value = []
    tableError.value = false
    tableLoading.value = channelId !== undefined
    if (channelId === undefined) return
    try {
      const response = await getPointsTables(channelId)
      if (current !== tableVersion) return
      if (!response.success || !response.data) throw new Error('Invalid channel points')
      leftTableData.value = (response.data.telemetry ?? []).map((point) => ({
        pointId: point.point_id, name: point.signal_name || '', unit: point.unit || '',
        value: null, updateTime: null,
      }))
      rightTableData.value = (response.data.signal ?? []).map((point) => ({
        pointId: point.point_id, name: point.signal_name || '', status: null, updateTime: null,
      }))
    } catch {
      if (current === tableVersion) tableError.value = true
    } finally {
      if (current === tableVersion) tableLoading.value = false
    }
  }, { immediate: true, flush: 'sync' })

  function applyUpdate(update: {
    source?: string
    channel_id: number
    data_type: string
    values: Record<string, number | string | null>
    ts?: Record<string, number | string | null>
  }) {
    if (wsManager.status.value !== 'connected' || update.channel_id !== selectedChannelId.value ||
      update.source !== 'comsrv') return
    if (update.data_type === 'T') leftTableData.value.forEach((item) => {
      const field = String(item.pointId)
      const sampleTs = parseSampleTimestamp(update.ts?.[field])
      item.updateTime = sampleTs
      item.value = sampleTs === null ? null : (update.values[field] ?? null)
    })
    if (update.data_type === 'S') rightTableData.value.forEach((item) => {
      const field = String(item.pointId)
      const sampleTs = parseSampleTimestamp(update.ts?.[field])
      item.updateTime = sampleTs
      item.status = sampleTs === null ? null : (update.values[field] ?? null)
    })
  }

  useTopologySubscribe(
    () => selectedChannelId.value === undefined ? [] : [selectedChannelId.value],
    { source: 'comsrv', dataTypes: ['T', 'S'], interval: 1000 },
    {
      onDataUpdate: applyUpdate,
      onBatchDataUpdate: (data) => { if ('updates' in data) data.updates.forEach(applyUpdate) },
    },
  )
  watch(wsManager.status, (status) => {
    if (status === 'connected') return
    leftTableData.value.forEach((item) => { item.value = null; item.updateTime = null })
    rightTableData.value.forEach((item) => { item.status = null; item.updateTime = null })
  }, { flush: 'sync' })
  onScopeDispose(() => { routingVersion++; tableVersion++ })

  return {
    channelIds, selectedChannelId, leftTableData, rightTableData,
    loading: computed(() => routingLoading.value || tableLoading.value),
    error: computed(() => routingError.value || tableError.value),
  }
}
