import { describe, expect, it } from 'vitest'
import {
  buildDemoChartData,
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
  oilPct: 68.5,
  pvVoltage: 398.2,
  essVoltage: 768.5,
  dgVoltage: 400.1,
  pvEnergyKwh: 12800,
  dgEnergyKwh: 3240,
  usedEnergyKwh: 9480,
  savingUsd: 1800.5,
}

describe('mocks/homepageDemo.ts', () => {
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
      expect(byId[17]).toMatchObject({ name: 'Oil', values: 68.5, unit: '%' })
      expect(byId[18]).toMatchObject({ name: 'P', values: 30, unit: 'kW' })
      expect(byId[19]).toMatchObject({ name: 'SOC', values: 65, unit: '%' })
    })

    it('exports exactly one update per homepage point id 1–19', () => {
      const updates = snapshotToPointUpdates(sampleSnapshot)
      const ids = updates.map((item) => item.id).sort((a, b) => a - b)
      expect(ids).toEqual(Array.from({ length: 19 }, (_, i) => i + 1))
    })
  })

  describe('buildDemoChartData', () => {
    it('produces an off-grid balanced snapshot at the latest timestep', () => {
      const { snapshot } = buildDemoChartData()
      expect(
        isOffGridBalanced(snapshot.pvKw, snapshot.loadKw, snapshot.dgKw, snapshot.essKw),
      ).toBe(true)
    })
  })
})
