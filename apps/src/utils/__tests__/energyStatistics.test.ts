import { describe, expect, it } from 'vitest'
import type { BatchQueryResponse } from '@/types/Statistics/OverView'
import {
  buildStatisticChartData, counterIntervals, resolveStatisticMetric, statisticHistory,
  statisticLiveValue, statisticRequests, STATISTIC_HISTORY_LIMIT,
} from '../energyStatistics'

const meter = resolveStatisticMetric(1, [{ measurement_id: 14, name: 'Total Energy', unit: 'kWh' }], ['Total Energy'], 'energy')!
function series(id: string, readings: (number | null)[], instance = 1): BatchQueryResponse {
  return { redis_key: `inst:${instance}:M`, point_id: id, count: readings.length,
    data: readings.map((value, i) => ({ timestamp: new Date(Date.UTC(2026, 9, 29, i)).toISOString(), value })) }
}

describe('energy and power statistics', () => {
  it('calculates interval consumption rather than summing lifetime readings', () => {
    const history = statisticHistory(meter, [series('14', [1000, 1003, 1008])])
    expect([...history.values.values()]).toEqual([null, 3, 5])
    expect(history.total).toBe(8)
  })

  it('does not fabricate energy on reset, missing data, single reading or truncation', () => {
    expect(counterIntervals([10, 12, 1, 4])).toEqual([null, 2, null, 3])
    for (const readings of [[10], [10, null, 15], [10, 12, 1, 4]]) {
      expect(statisticHistory(meter, [series('14', readings)]).total).toBeNull()
    }
    expect(statisticHistory(meter, [{ ...series('14', [10, 12]), count: STATISTIC_HISTORY_LIMIT }]).total).toBeNull()
    expect(statisticHistory(meter, [series('14', [10, 10])]).total).toBe(0)
  })

  it('combines charge and discharge with discharge positive and missing points unknown', () => {
    const power = resolveStatisticMetric(1, [
      { measurement_id: 5, name: 'Charge Power', unit: 'kW' },
      { measurement_id: 6, name: 'Discharge Power', unit: 'kW' },
    ], ['Discharge Power', 'Charge Power'], 'power', [1, -1])!
    expect(statisticLiveValue(power, { 5: 8, 6: 0 })).toBe(-8)
    expect(statisticLiveValue(power, { 5: 0, 6: 12 })).toBe(12)
    expect(statisticLiveValue(power, { 5: 0 })).toBeNull()
    const history = statisticHistory(power, [series('5', [8, 0]), series('6', [0, 12])])
    expect([...history.values.values()]).toEqual([-8, 12])
  })

  it('honors actual IDs and unit scaling; rejects missing or incompatible units', () => {
    const power = resolveStatisticMetric(2, [{ measurement_id: 212, name: 'PV Input Power', unit: 'W' }], ['PV Input Power'], 'power')!
    expect(statisticLiveValue(power, { 212: 1500 })).toBe(1.5)
    expect(statisticRequests({ pvPower: power })).toEqual([{ redis_key: 'inst:2:M', point_id: '212' }])
    expect(resolveStatisticMetric(2, [{ measurement_id: 212, name: 'PV Input Power', unit: 'V' }], ['PV Input Power'], 'power')).toBeUndefined()
    expect(resolveStatisticMetric(2, [], ['PV Input Power'], 'power')).toBeUndefined()
  })

  it('keeps asynchronous/missing samples as gaps and computes each counter independently', () => {
    const power = resolveStatisticMetric(2, [{ measurement_id: 1, name: 'Total Power', unit: 'kW' }], ['Total Power'], 'power')!
    const chart = buildStatisticChartData({ loadEnergy: meter, pvPower: power }, [
      series('14', [100, 103]), { ...series('1', [4], 2), data: [
        { timestamp: '2026-10-29T00:30:00.000Z', value: 4 },
      ] },
    ])
    expect(chart.values('loadEnergy')).toEqual([null, null, 3])
    expect(chart.values('pvPower')).toEqual([null, 4, null])
    expect(chart.loadEnergy).toBe(3)
    expect(chart.values('soc')).toEqual([null, null, null])
  })
})
