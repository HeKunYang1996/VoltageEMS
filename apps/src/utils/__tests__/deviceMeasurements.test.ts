import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { InstanceMeasurementItem } from '@/types/deviceConfiguration'
import {
  BATTERY_OVERVIEW_MEASUREMENTS, productDisplayName, pvOverviewMeasurements, selectMeasurements,
} from '../deviceMeasurements'

function productMeasurements(name: string): InstanceMeasurementItem[] {
  const product = JSON.parse(readFileSync(resolve(dirname(fileURLToPath(import.meta.url)),
    '../../../../libs/voltage-model/src/products', `${name}.json`), 'utf8'))
  return product.M.map((point: { id: number; name: string; unit?: string }) => ({
    measurement_id: point.id, name: point.name, unit: point.unit ?? '',
  }))
}

describe('current device measurement definitions', () => {
  it('uses Hybrid points instead of AC/PCS point numbers', () => {
    const points = pvOverviewMeasurements('Hybrid_Inverter', productMeasurements('Hybrid_Inverter'))
    expect(points.map((point) => point.measurement_id)).toEqual([2, 1, 6, 15])
    expect(points.map((point) => point.name)).not.toContain('Battery Current')
  })

  it('uses AC Inverter total power and daily energy', () => {
    expect(pvOverviewMeasurements('AC_Inverter', productMeasurements('AC_Inverter'))
      .map((point) => point.measurement_id)).toEqual([1, 3, 4, 17])
  })

  it('resolves Battery SOC, SOH, status and separate charge/discharge power', () => {
    const points = selectMeasurements(productMeasurements('Battery'), BATTERY_OVERVIEW_MEASUREMENTS)
    expect(points.map((point) => point.measurement_id)).toEqual([11, 7, 8, 1, 2, 5, 6, 12, 13, 14, 15, 16])
  })

  it('uses actual metadata IDs and omits unavailable measurements', () => {
    const points = [{ measurement_id: 212, name: 'PV Input Power', unit: 'kW' }]
    expect(pvOverviewMeasurements('Hybrid_Inverter', points)).toEqual(points)
    expect(pvOverviewMeasurements('PCS', points)).toEqual([])
    expect(productDisplayName('Hybrid_Inverter')).toBe('Hybrid Inverter')
    expect(productDisplayName('PCS')).toBe('PCS')
  })
})
