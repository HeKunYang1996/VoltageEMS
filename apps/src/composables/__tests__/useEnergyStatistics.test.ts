import { beforeEach, describe, expect, it, vi } from 'vitest'
import { computed, reactive } from 'vue'
import { readFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { InstanceMeasurementItem } from '@/types/deviceConfiguration'
import { useEnergyStatistics } from '../useEnergyStatistics'
import { statisticRequests } from '@/utils/energyStatistics'

const mocks = vi.hoisted(() => ({ topology: vi.fn(), measurements: vi.fn() }))
vi.mock('@/stores/deviceTopology', () => ({ useDeviceTopologyStore: mocks.topology }))
vi.mock('@/composables/useInstanceMeasurements', () => ({ useInstanceMeasurements: mocks.measurements }))
function productPoints(name: string): InstanceMeasurementItem[] {
  const data = JSON.parse(readFileSync(resolve(dirname(fileURLToPath(import.meta.url)),
    '../../../../libs/voltage-model/src/products', `${name}.json`), 'utf8'))
  return data.M.map((point: { id: number; name: string; unit?: string }) => ({
    measurement_id: point.id, name: point.name, unit: point.unit ?? '',
  }))
}

describe('selected product statistics contract', () => {
  const state = reactive({
    selectedPvGroup: { relatedInstanceIds: [1], relatedProductName: 'AC_Inverter' },
    selectedBatteryGroup: { primaryInstanceIds: [2] }, selectedDieselId: 3,
    getLogicalDeviceInstanceIds: () => [4],
  })
  beforeEach(() => {
    state.selectedPvGroup.relatedProductName = 'AC_Inverter'
    mocks.topology.mockReturnValue(state)
    mocks.measurements.mockImplementation((getId: () => number | undefined) => ({
      measurements: computed(() => {
        const id = getId()
        return id === undefined ? [] : productPoints(id === 1 ? state.selectedPvGroup.relatedProductName
          : id === 2 ? 'Battery' : id === 3 ? 'Diesel' : 'Meter')
      }),
      values: computed(() => ({ 1: 10, 2: 22, 5: 4, 6: 0, 9: 999 })),
    }))
  })

  it('uses AC total power/energy, Battery net power/SOC and Meter cumulative energy', () => {
    const result = useEnergyStatistics()
    const ids = (name: keyof typeof result.metrics.value) => result.metrics.value[name]?.terms.map((p) => p.pointId)
    expect(ids('pvPower')).toEqual(['1'])
    expect(ids('pvEnergy')).toEqual(['17'])
    expect(ids('essPower')).toEqual(['6', '5'])
    expect(ids('essEnergy')).toEqual(['10'])
    expect(ids('soc')).toEqual(['7'])
    expect(ids('loadEnergy')).toEqual(['14'])
    expect(ids('dgPower')).toEqual(['1'])
    expect(ids('dgEnergy')).toEqual(['2'])
    expect(result.live.value).toEqual({ pvPower: 10, essPower: -4, dgPower: 10 })
  })

  it('switches Hybrid PV power/total energy with the selected product', () => {
    const result = useEnergyStatistics()
    state.selectedPvGroup.relatedProductName = 'Hybrid_Inverter'
    expect(result.metrics.value.pvPower?.terms[0].pointId).toBe('2')
    expect(result.metrics.value.pvEnergy?.terms[0].pointId).toBe('16')
    expect(result.live.value.pvPower).toBe(22)
  })

  it('makes no requests or live values when disabled for homepage demo mode', () => {
    const result = useEnergyStatistics(() => false)
    expect(statisticRequests(result.metrics.value)).toEqual([])
    expect(result.live.value).toEqual({ pvPower: null, essPower: null, dgPower: null })
  })
})
