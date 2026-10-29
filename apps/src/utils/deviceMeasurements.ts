import type { InstanceMeasurementItem } from '@/types/deviceConfiguration'

export function productDisplayName(productName: string): string {
  return productName.replace(/_/g, ' ')
}

// Resolve IDs from the actual instance definition instead of sharing point numbers
// between different products. Missing measurements stay absent, never substituted.
export const BATTERY_OVERVIEW_MEASUREMENTS = [
  'Charge Discharge Status', 'SOC', 'SOH', 'Total Voltage', 'Total Current',
  'Charge Power', 'Discharge Power', 'Max Cell Voltage', 'Min Cell Voltage',
  'Avg Cell Voltage', 'Cell Voltage Difference', 'Avg Cell Temperature',
] as const

const PV_OVERVIEW_MEASUREMENTS: Record<string, readonly string[]> = {
  AC_Inverter: ['Total Power', 'DC Voltage', 'DC Current', 'Energy Today'],
  Hybrid_Inverter: ['PV Input Power', 'Inverter AC Output Power', 'Battery SOC', 'PV Energy Today'],
}

export function selectMeasurements(
  measurements: InstanceMeasurementItem[],
  names: readonly string[],
): InstanceMeasurementItem[] {
  return names.flatMap((name) => {
    const point = measurements.find((item) => item.name === name)
    return point ? [point] : []
  })
}

export function pvOverviewMeasurements(productName: string, measurements: InstanceMeasurementItem[]) {
  return selectMeasurements(measurements, PV_OVERVIEW_MEASUREMENTS[productName] ?? [])
}
