import { describe, expect, it, vi } from 'vitest'
import {
  buildDemoChartData,
  buildDemoAlarms,
  DEMO_ALARM_NUM,
  DEMO_SCENARIO,
  isOffGridBalanced,
  snapshotToPointUpdates,
  type MicrogridSnapshot,
} from '../homepageDemo'

const sampleSnapshot: MicrogridSnapshot = {
  pvKw: 120,
  loadKw: 200,
  dgKw: 50,
  essKw: 30,
  soc: 65,
  oilPct: 38.5,
  pvVoltage: 398.2,
  essVoltage: 768.5,
  dgVoltage: 400.1,
  pvEnergyKwh: 12800,
  dgEnergyKwh: 3240,
  usedEnergyKwh: 9480,
  savingUsd: 1800.5,
}

describe('mocks/homepageDemo.ts', () => {
  describe('buildDemoAlarms', () => {
    it('exports an initial alarm count for App consistent with the initial snapshot', () => {
      expect(DEMO_ALARM_NUM).toBe(buildDemoAlarms(buildDemoChartData().snapshot).length)
    })

    it.each([0, 50])('shows an L3 fuel threshold notice at diesel power %s kW', (dgKw) => {
      const snapshot = { ...sampleSnapshot, dgKw }
      expect(buildDemoAlarms(snapshot)).toEqual([
        {
          id: 'demo-1',
          deviceName: 'Diesel',
          alarmLevel: 'Info Alarm',
          alarmMsg: 'Fuel level low (38.5% < 40%)',
        },
      ])
      expect(snapshotToPointUpdates(snapshot).find((point) => point.id === 17)?.values).toBe(38.5)
    })

    it.each([40, 85, NaN, -1])('does not trigger for oil level %s', (oilPct) => {
      expect(buildDemoAlarms({ ...sampleSnapshot, oilPct })).toEqual([])
    })
  })

  describe('isOffGridBalanced', () => {
    it('returns true when P_pv + P_dg + P_ess equals P_load within tolerance', () => {
      expect(isOffGridBalanced(120, 200, 50, 30)).toBe(true)
      expect(isOffGridBalanced(100, 100, 0, 0)).toBe(true)
    })

    it('returns false when power balance deviates beyond tolerance', () => {
      expect(isOffGridBalanced(120, 200, 50, 20)).toBe(false)
      expect(isOffGridBalanced(0, 100, 0, 0)).toBe(false)
    })
  })

  describe('snapshotToPointUpdates', () => {
    it('includes topology point IDs 14–19 with values from snapshot', () => {
      const updates = snapshotToPointUpdates(sampleSnapshot)
      const byId = Object.fromEntries(updates.map((item) => [item.id, item]))

      expect(updates.map((item) => item.id)).toEqual(
        expect.arrayContaining([14, 15, 16, 17, 18, 19]),
      )

      expect(byId[14]).toMatchObject({ name: 'P', values: 120, unit: 'kW' })
      expect(byId[15]).toMatchObject({ name: 'P', values: 200, unit: 'kW' })
      expect(byId[16]).toMatchObject({ name: 'P', values: 50, unit: 'kW' })
      expect(byId[17]).toMatchObject({ name: 'Fuel', values: 38.5, unit: '%' })
      expect(byId[18]).toMatchObject({ name: 'P', values: 30, unit: 'kW' })
      expect(byId[19]).toMatchObject({ name: 'SOC', values: 65, unit: '%' })
    })

    it('exports exactly one update per homepage point id 1–19', () => {
      const updates = snapshotToPointUpdates(sampleSnapshot)
      const ids = updates.map((item) => item.id).sort((a, b) => a - b)
      expect(ids).toEqual(Array.from({ length: 19 }, (_, i) => i + 1))
    })
  })

  describe('marketing scenario consistency', () => {
    it('has exactly six hours of common chart boundaries and a matching endpoint', () => {
      const chart = buildDemoChartData()
      expect(chart.xAxisData).toHaveLength(37)
      expect(chart.xAxisData[0]).toBe('10:00')
      expect(chart.xAxisData.at(-1)).toBe('16:00')
      const endpoint = [chart.snapshot.pvKw, chart.snapshot.dgKw, chart.snapshot.essKw]
      chart.lineChartSeries.forEach((series, i) => {
        expect(series.data).toHaveLength(chart.xAxisData.length)
        expect(series.data.at(-1)).toBe(endpoint[i])
      })
      chart.energyChartSeries.forEach((series) => {
        expect(series.data).toHaveLength(chart.xAxisData.length)
        expect(series.data[0]).toBe(0)
      })
    })

    it('balances power and energy at every step while respecting equipment and SOC limits', () => {
      const chart = buildDemoChartData()
      const sum = (values: number[]) => values.reduce((a, b) => a + b, 0)
      let batteryEnergy = 0
      for (const { snapshot: s, intervalEnergy: e } of chart.samples) {
        expect(isOffGridBalanced(s.pvKw, s.loadKw, s.dgKw, s.essKw)).toBe(true)
        expect(e.pv + e.diesel + e.bess).toBeCloseTo(e.load, 10)
        expect(s.soc).toBeGreaterThanOrEqual(22)
        expect(s.soc).toBeLessThanOrEqual(92)
        expect(Math.abs(s.essKw)).toBeLessThanOrEqual(DEMO_SCENARIO.batteryMaxKw)
        expect(s.dgKw).toBeGreaterThanOrEqual(0)
        expect(s.dgKw).toBeLessThanOrEqual(DEMO_SCENARIO.dieselMaxKw)
        expect(s.pvKw).toBeGreaterThanOrEqual(0)
        batteryEnergy += e.bess
        expect(
          ((DEMO_SCENARIO.initialSoc - s.soc) / 100) * DEMO_SCENARIO.batteryCapacityKwh,
        ).toBeCloseTo(batteryEnergy, 9)
      }
      expect(sum(chart.energyChartSeries[0].data)).toBeCloseTo(chart.snapshot.pvEnergyKwh, 10)
      expect(sum(chart.energyChartSeries[1].data)).toBeCloseTo(chart.snapshot.dgEnergyKwh, 10)
      expect(chart.snapshot.pvEnergyKwh + chart.snapshot.dgEnergyKwh + batteryEnergy).toBeCloseTo(
        chart.snapshot.usedEnergyKwh,
        9,
      )
      // Charging must remain negative in the energy chart, rather than disappearing.
      expect(chart.energyChartSeries[2].data.some((value) => value < 0)).toBe(true)
      expect(chart.energyChartSeries[2].data.some((value) => value > 0)).toBe(true)
    })

    it('integrates the actual ten-minute interval and derives savings from the same totals', () => {
      const { samples } = buildDemoChartData()
      for (let i = 1; i < samples.length; i++) {
        const previous = samples[i - 1].snapshot
        const { snapshot, intervalEnergy } = samples[i]
        expect(intervalEnergy.pv).toBeCloseTo((previous.pvKw + snapshot.pvKw) / 2 / 6, 10)
        expect(intervalEnergy.diesel).toBeCloseTo((previous.dgKw + snapshot.dgKw) / 2 / 6, 10)
        expect(snapshot.savingUsd).toBeCloseTo(
          (snapshot.usedEnergyKwh - snapshot.dgEnergyKwh) * DEMO_SCENARIO.avoidedDieselCostPerKwh,
          10,
        )
      }
    })

    it('is identical after idle time or a reload on a different day', () => {
      vi.useFakeTimers()
      try {
        vi.setSystemTime(new Date('2026-10-22T03:00:00Z'))
        const first = buildDemoChartData()
        vi.setSystemTime(new Date('2027-03-02T20:00:00Z'))
        expect(buildDemoChartData()).toEqual(first)
      } finally {
        vi.useRealTimers()
      }
    })
  })
})
