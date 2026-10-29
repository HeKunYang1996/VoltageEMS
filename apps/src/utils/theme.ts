/** Resolve CSS theme colors before passing them to ECharts. */
export function themeColor(variable: string, fallback: string): string {
  if (typeof document === 'undefined') return fallback
  return getComputedStyle(document.documentElement).getPropertyValue(variable).trim() || fallback
}

export function homepageChartColor(name: string): string {
  const colors: Record<string, [string, string]> = {
    PV: ['--vt-color-chart-pv', '#6786b3'],
    ESS: ['--vt-color-chart-ess', '#005ee8'],
    BESS: ['--vt-color-chart-ess', '#005ee8'],
    DG: ['--vt-color-chart-dg', '#0035a2'],
    Diesel: ['--vt-color-chart-dg', '#0035a2'],
  }
  const [variable, fallback] = colors[name] || colors.PV
  return themeColor(variable, fallback)
}
