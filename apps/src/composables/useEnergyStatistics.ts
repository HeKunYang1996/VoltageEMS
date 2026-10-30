import { computed } from 'vue'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import { useInstanceMeasurements } from '@/composables/useInstanceMeasurements'
import useTopologySubscribe from '@/composables/useTopologySubscribe'
import { resolveStatisticMetric, statisticLiveValue, type StatisticMetrics } from '@/utils/energyStatistics'

/** Shared bindings for the homepage history and statistics overview. */
export function useEnergyStatistics(
  enabled: () => boolean = () => true,
  options: { realtime?: boolean } = {},
) {
  const topology = useDeviceTopologyStore()
  const pvId = () => enabled() ? topology.selectedPvGroup?.relatedInstanceIds[0] : undefined
  const essId = () => enabled() ? topology.selectedBatterySystem?.primaryInstanceIds[0] : undefined
  const dgId = () => enabled() ? topology.selectedDieselId ?? undefined : undefined
  const loadId = () => enabled() ? topology.getLogicalDeviceInstanceIds('meterLoad')[0] : undefined
  const pv = useInstanceMeasurements(pvId, { realtime: false })
  const ess = useInstanceMeasurements(essId, { realtime: false })
  const dg = useInstanceMeasurements(dgId, { realtime: false })
  const load = useInstanceMeasurements(loadId, { realtime: false })

  const sources = [pv, ess, dg, load]
  const loading = computed(() => topology.loading || sources.some((source) => source.loading.value))
  const applyUpdate = (update: Parameters<typeof pv.applyUpdate>[0]) => {
    sources.forEach((source) => source.applyUpdate(update))
  }
  // The gateway has one subscription per socket. Homepage history must not
  // replace its homepage subscription with device subscriptions.
  useTopologySubscribe(
    () => options.realtime === false ? [] : [...new Set(
      [pvId(), essId(), dgId(), loadId()].filter((id): id is number => id !== undefined),
    )],
    { source: 'inst', dataTypes: ['M'], interval: 1000 },
    {
      onDataUpdate: applyUpdate,
      onBatchDataUpdate: (data) => { if ('updates' in data) data.updates.forEach(applyUpdate) },
    },
  )

  const metrics = computed<StatisticMetrics>(() => {
    const hybrid = topology.selectedPvGroup?.relatedProductName === 'Hybrid_Inverter'
    const pvMeasurements = pv.measurements.value
    const pvEnergyName = hybrid
      ? (pvMeasurements.some((p) => p.name === 'PV Energy Total') ? 'PV Energy Total' : 'PV Energy Today')
      : 'Energy Today'
    return {
      pvPower: resolveStatisticMetric(pvId(), pvMeasurements, [hybrid ? 'PV Input Power' : 'Total Power'], 'power'),
      pvEnergy: resolveStatisticMetric(pvId(), pvMeasurements, [pvEnergyName], 'energy'),
      // Project convention: positive discharges, negative charges.
      essPower: resolveStatisticMetric(essId(), ess.measurements.value, ['Discharge Power', 'Charge Power'], 'power', [1, -1]),
      essEnergy: resolveStatisticMetric(essId(), ess.measurements.value, ['Discharge Energy'], 'energy'),
      soc: resolveStatisticMetric(essId(), ess.measurements.value, ['SOC'], 'soc'),
      dgPower: resolveStatisticMetric(dgId(), dg.measurements.value, ['Diesel Power'], 'power'),
      dgEnergy: resolveStatisticMetric(dgId(), dg.measurements.value, ['Diesel Energy'], 'energy'),
      loadEnergy: resolveStatisticMetric(loadId(), load.measurements.value, ['Total Energy'], 'energy'),
    }
  })
  const live = computed(() => ({
    pvPower: statisticLiveValue(metrics.value.pvPower, pv.values.value),
    essPower: statisticLiveValue(metrics.value.essPower, ess.values.value),
    dgPower: statisticLiveValue(metrics.value.dgPower, dg.values.value),
  }))
  return { metrics, live, loading }
}
