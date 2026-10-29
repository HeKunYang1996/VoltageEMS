import { describe, expect, it } from 'vitest'
import { essState, flowDirection, powerInKw } from '../homeEnergyFlow'

describe('homepage power interpretation', () => {
  it('uses raw telemetry before the rounded display value', () => {
    expect(powerInKw({ rawValue: -0.0004, value: '-0.000', unit: 'kW' })).toBe(-0.0004)
    expect(essState({ rawValue: -0.0004, value: '-0.000' })).toBe('charge')
  })

  it('accepts finite numeric strings and normalizes supported power units', () => {
    expect(powerInKw({ value: ' 1200 ', unit: 'W' })).toBe(1.2)
    expect(powerInKw({ value: '-0.02', unit: 'MW' })).toBe(-20)
    expect(powerInKw({ value: 0, unit: ' kW ' })).toBe(0)
  })

  it.each([undefined, null, '', ' ', '-', 'NaN', '20 kW', Infinity, NaN, true])(
    'does not treat missing or invalid telemetry (%s) as an idle measurement',
    (value) => {
      expect(flowDirection({ value })).toBe('unknown')
      expect(essState({ value })).toBe('unknown')
    },
  )

  it('does not fall back to displayed data when raw telemetry is missing', () => {
    expect(powerInKw({ rawValue: undefined, value: '32' })).toBeNull()
    expect(powerInKw(undefined)).toBeNull()
    expect(powerInKw({ value: 20, unit: '%' })).toBeNull()
  })

  it('distinguishes charging, discharging and zero-power idle', () => {
    expect(essState({ value: -32 })).toBe('charge')
    expect(essState({ value: 32 })).toBe('discharge')
    expect(essState({ value: -0 })).toBe('idle')
  })

  it('maps signed power onto the orientation of each branch', () => {
    expect(flowDirection({ value: 32 })).toBe('forward')
    expect(flowDirection({ value: -32 })).toBe('reverse')
    expect(flowDirection({ value: 32 }, false)).toBe('reverse')
    expect(flowDirection({ value: -32 }, false)).toBe('forward')
    expect(flowDirection({ value: 0 }, false)).toBe('idle')
  })
})
