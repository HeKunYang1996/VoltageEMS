export interface PowerPoint {
  value?: unknown
  /** Unrounded telemetry, independent of the displayed value. */
  rawValue?: unknown
  unit?: string
}

export type FlowDirection = 'forward' | 'reverse' | 'idle' | 'unknown'
export type EssState = 'charge' | 'discharge' | 'idle' | 'unknown'

export function powerInKw(point: PowerPoint | undefined): number | null {
  if (!point) return null
  const value = 'rawValue' in point ? point.rawValue : point.value
  if (typeof value !== 'number' && typeof value !== 'string') return null
  if (typeof value === 'string' && !value.trim()) return null
  const power = Number(value)
  if (!Number.isFinite(power)) return null

  switch ((point.unit ?? 'kW').trim().toLowerCase()) {
    case 'w':
      return power / 1000
    case 'kw':
      return power
    case 'mw':
      return power * 1000
    default:
      return null
  }
}

/** Each branch follows its own measured power; ESS is the shared junction in the artwork. */
export function flowDirection(
  point: PowerPoint | undefined,
  positiveIsForward = true,
): FlowDirection {
  const power = powerInKw(point)
  if (power === null) return 'unknown'
  if (power === 0) return 'idle'
  return power > 0 === positiveIsForward ? 'forward' : 'reverse'
}

/** Project convention: negative ESS power charges, positive ESS power discharges. */
export function essState(point: PowerPoint | undefined): EssState {
  const power = powerInKw(point)
  if (power === null) return 'unknown'
  if (power === 0) return 'idle'
  return power < 0 ? 'charge' : 'discharge'
}
