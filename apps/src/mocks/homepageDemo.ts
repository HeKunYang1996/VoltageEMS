/** homepage_batch 推送的单条点位 */
export interface DemoPointUpdate {
  id: number
  name: string
  values: number
  unit: string
  imgurl?: string
}

/** 首页告警展示项 */
export interface DemoAlarmItem {
  id: number | string
  deviceName: string
  alarmLevel: 'Critical Alarm' | 'Warning Alarm' | 'Info Alarm'
  alarmMsg: string
}

/** 微电网某一时刻的快照（离网自洽：P_pv + P_dg + P_ess = P_load） */
export interface MicrogridSnapshot {
  pvKw: number
  loadKw: number
  dgKw: number
  essKw: number
  soc: number
  oilPct: number
  pvVoltage: number
  essVoltage: number
  dgVoltage: number
  pvEnergyKwh: number
  dgEnergyKwh: number
  usedEnergyKwh: number
  savingUsd: number
}

/** Fixed, idealized six-hour off-grid scene for repeatable marketing screenshots. */
export const DEMO_SCENARIO = {
  startHour: 10,
  endHour: 16,
  intervalMinutes: 10,
  batteryCapacityKwh: 500,
  initialSoc: 64,
  batteryMaxKw: 100,
  dieselMaxKw: 180,
  avoidedDieselCostPerKwh: 0.24,
} as const

// Totals share the six-hour scenario. Savings use an illustrative diesel-only
// baseline ($0.24/kWh). The idealized 500 kWh battery starts at 64% SOC;
// conversion losses are excluded. Positive BESS power discharges, negative charges.
const round1 = (v: number) => Number(v.toFixed(1))
const round2 = (v: number) => Number(v.toFixed(2))

/** 演示用提前补油提示阈值，不代表设备故障阈值。 */
const DEMO_FUEL_NOTICE_THRESHOLD = 40

/** 离网功率平衡校验：P_pv + P_dg + P_ess = P_load */
export function isOffGridBalanced(pv: number, load: number, dg: number, ess: number): boolean {
  return Math.abs(pv + dg + ess - load) < 0.15
}

/** 根据剩余油量生成 L3 阈值提示，与拓扑中的 Fuel 数值保持一致。 */
export function buildDemoAlarms(snapshot: MicrogridSnapshot): DemoAlarmItem[] {
  const oil = snapshot.oilPct
  if (!Number.isFinite(oil) || oil < 0 || oil >= DEMO_FUEL_NOTICE_THRESHOLD) return []

  return [
    {
      id: 'demo-1',
      deviceName: 'Diesel',
      alarmLevel: 'Info Alarm',
      alarmMsg: `Fuel level low (${oil}% < ${DEMO_FUEL_NOTICE_THRESHOLD}%)`,
    },
  ]
}

interface DemoSample {
  time: string
  snapshot: MicrogridSnapshot
  intervalEnergy: { pv: number; diesel: number; bess: number; load: number }
}

/** Deterministic daylight profile; never feeds curtailed output back into solar availability. */
function demoPower(hour: number) {
  const progress = (hour - DEMO_SCENARIO.startHour) / 6
  const pvAvailable = round1(
    80 +
      50 * progress +
      140 * Math.sin(Math.PI * progress) +
      10 * Math.sin(4 * Math.PI * progress) * Math.sin(Math.PI * progress),
  )
  const load = round1(
    175 +
      55 * Math.exp(-Math.pow((hour - 10.1) / 0.6, 2)) +
      20 * Math.exp(-Math.pow((hour - 15.5) / 0.85, 2)),
  )
  const bess = round1(
    Math.max(-DEMO_SCENARIO.batteryMaxKw, Math.min(DEMO_SCENARIO.batteryMaxKw, load - pvAvailable)),
  )
  const diesel = round1(Math.max(0, load - pvAvailable - bess))
  // Surplus beyond the battery rating is curtailed rather than inventing extra demand.
  const pv = round1(load - diesel - bess)
  return { pv, load, bess, diesel }
}

/** Power samples are boundaries; each bar integrates the preceding ten-minute interval. */
function simulateMicrogridSeries(): DemoSample[] {
  const samples: DemoSample[] = []
  const intervalHours = DEMO_SCENARIO.intervalMinutes / 60
  const intervals = (DEMO_SCENARIO.endHour - DEMO_SCENARIO.startHour) / intervalHours
  let pvEnergy = 0
  let dieselEnergy = 0
  let usedEnergy = 0
  let soc = DEMO_SCENARIO.initialSoc as number

  for (let i = 0; i <= intervals; i++) {
    const hour = DEMO_SCENARIO.startHour + i * intervalHours
    const power = demoPower(hour)
    const previous = samples.at(-1)?.snapshot
    const integrate = (previousPower: number, currentPower: number) =>
      previous ? ((previousPower + currentPower) / 2) * intervalHours : 0
    const intervalEnergy = {
      pv: integrate(previous?.pvKw ?? 0, power.pv),
      diesel: integrate(previous?.dgKw ?? 0, power.diesel),
      bess: integrate(previous?.essKw ?? 0, power.bess),
      load: integrate(previous?.loadKw ?? 0, power.load),
    }
    pvEnergy += intervalEnergy.pv
    dieselEnergy += intervalEnergy.diesel
    usedEnergy += intervalEnergy.load
    soc -= (intervalEnergy.bess / DEMO_SCENARIO.batteryCapacityKwh) * 100
    const minuteOfDay = Math.round(hour * 60)

    samples.push({
      time: `${String(Math.floor(minuteOfDay / 60)).padStart(2, '0')}:${String(minuteOfDay % 60).padStart(2, '0')}`,
      intervalEnergy,
      snapshot: {
        pvKw: power.pv,
        loadKw: power.load,
        dgKw: power.diesel,
        essKw: power.bess,
        soc,
        oilPct: 38.5,
        pvVoltage: 398.2,
        essVoltage: 768.5,
        dgVoltage: 400.1,
        pvEnergyKwh: pvEnergy,
        dgEnergyKwh: dieselEnergy,
        usedEnergyKwh: usedEnergy,
        savingUsd: (usedEnergy - dieselEnergy) * DEMO_SCENARIO.avoidedDieselCostPerKwh,
      },
    })
  }
  return samples
}

/** 快照 → homepage_batch 点位（拓扑/站点/设备数值一致） */
export function snapshotToPointUpdates(s: MicrogridSnapshot): DemoPointUpdate[] {
  const updates: DemoPointUpdate[] = [
    { id: 1, name: 'PV Energy', values: s.pvEnergyKwh, unit: 'kWh', imgurl: 'icon-pv-energy' },
    {
      id: 2,
      name: 'Diesel Energy',
      values: s.dgEnergyKwh,
      unit: 'kWh',
      imgurl: 'icon-diesel-energy',
    },
    {
      id: 3,
      name: 'Energy Used',
      values: s.usedEnergyKwh,
      unit: 'kWh',
      imgurl: 'icon-energy-used',
    },
    {
      id: 4,
      name: 'Saving Billing',
      values: s.savingUsd,
      unit: '$',
      imgurl: 'icon-saving-billing',
    },
    { id: 5, name: 'PV Power', values: s.pvKw, unit: 'kW', imgurl: 'icon-pv-energy' },
    { id: 6, name: 'Diesel Power', values: s.dgKw, unit: 'kW', imgurl: 'icon-diesel-energy' },
    { id: 7, name: 'BESS Power', values: s.essKw, unit: 'kW', imgurl: 'icon-ess-energy' },
    { id: 8, name: 'P', values: s.pvKw, unit: 'kW' },
    { id: 9, name: 'U', values: s.pvVoltage, unit: 'V' },
    { id: 10, name: 'P', values: s.dgKw, unit: 'kW' },
    { id: 11, name: 'U', values: s.dgVoltage, unit: 'V' },
    { id: 12, name: 'P', values: s.essKw, unit: 'kW' },
    { id: 13, name: 'U', values: s.essVoltage, unit: 'V' },
    { id: 14, name: 'P', values: s.pvKw, unit: 'kW' },
    { id: 15, name: 'P', values: s.loadKw, unit: 'kW' },
    { id: 16, name: 'P', values: s.dgKw, unit: 'kW' },
    { id: 17, name: 'Fuel', values: s.oilPct, unit: '%' },
    { id: 18, name: 'P', values: s.essKw, unit: 'kW' },
    { id: 19, name: 'SOC', values: s.soc, unit: '%' },
  ]
  return updates.map((point) => ({
    ...point,
    values: point.id === 4 ? round2(point.values) : round1(point.values),
  }))
}

/** One source for chart history, period totals and the exact current snapshot. */
export function buildDemoChartData() {
  const samples = simulateMicrogridSeries()
  return {
    samples,
    xAxisData: samples.map((sample) => sample.time),
    snapshot: samples.at(-1)!.snapshot,
    lineChartSeries: [
      { name: 'PV', data: samples.map((s) => s.snapshot.pvKw), color: 'rgba(105, 203, 255, 1)' },
      { name: 'Diesel', data: samples.map((s) => s.snapshot.dgKw), color: 'rgba(246, 200, 95, 1)' },
      { name: 'BESS', data: samples.map((s) => s.snapshot.essKw), color: 'rgba(29, 134, 255, 1)' },
    ],
    energyChartSeries: [
      {
        name: 'PV',
        data: samples.map((s) => s.intervalEnergy.pv),
        color: 'rgba(105, 203, 255, 1)',
      },
      {
        name: 'Diesel',
        data: samples.map((s) => s.intervalEnergy.diesel),
        color: 'rgba(246, 200, 95, 1)',
      },
      {
        name: 'BESS',
        data: samples.map((s) => s.intervalEnergy.bess),
        color: 'rgba(29, 134, 255, 1)',
      },
    ],
  }
}

/** Keep the header alarm count consistent with the displayed fuel notice. */
export const DEMO_ALARM_NUM = buildDemoAlarms(buildDemoChartData().snapshot).length
export const DEMO_HOMEPAGE_UPDATES = snapshotToPointUpdates(buildDemoChartData().snapshot)
