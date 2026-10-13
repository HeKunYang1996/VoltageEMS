import dayjs from 'dayjs'

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

const INTERVAL_HOURS = 10 / 60
const ESS_MAX_KW = 100
const DG_MAX_KW = 180
const PV_PEAK_KW = 420
const ESS_CAPACITY_KWH = 500

const round1 = (v: number) => Number(v.toFixed(1))
const round2 = (v: number) => Number(v.toFixed(2))

/** 演示模式 Header 铃铛告警数 */
export const DEMO_ALARM_NUM = 1

/** 离网功率平衡校验：P_pv + P_dg + P_ess = P_load */
export function isOffGridBalanced(pv: number, load: number, dg: number, ess: number): boolean {
  return Math.abs(pv + dg + ess - load) < 0.15
}

/** 根据当前快照生成 1 条 ESS 演示告警 */
export function buildDemoAlarms(s: MicrogridSnapshot): DemoAlarmItem[] {
  if (s.soc >= 88) {
    return [
      { id: 'demo-1', deviceName: 'ESS', alarmLevel: 'Info Alarm', alarmMsg: 'SOC near full' },
    ]
  }
  if (s.soc <= 28) {
    return [
      {
        id: 'demo-1',
        deviceName: 'ESS',
        alarmLevel: 'Warning Alarm',
        alarmMsg: 'SOC below 25%',
      },
    ]
  }
  return [
    { id: 'demo-1', deviceName: 'ESS', alarmLevel: 'Info Alarm', alarmMsg: 'Battery normal' },
  ]
}

/** 确定性云遮波动 */
function cloudFactor(hour: number): number {
  const w1 = Math.sin(hour * 2.7) * 0.06
  const w2 = Math.sin(hour * 5.3 + 1.2) * 0.04
  const w3 = Math.sin(hour * 11.1 + 0.5) * 0.025
  return Math.max(0.72, Math.min(1, 1 + w1 + w2 + w3))
}

/** 负载曲线 */
function loadKw(hour: number): number {
  const base = 125
  const morning = 85 * Math.exp(-Math.pow((hour - 8.3) / 1.1, 2))
  const evening = 95 * Math.exp(-Math.pow((hour - 18.8) / 2, 2))
  const hvac = 30 * Math.exp(-Math.pow((hour - 14.2) / 2.8, 2))
  return base + morning + evening + hvac
}

/** 光伏可发潜力 */
function pvAvailableKw(hour: number): number {
  if (hour < 6.5 || hour > 19) return 0
  const x = (hour - 6.5) / 12.5
  const bell = Math.sin(x * Math.PI)
  return PV_PEAK_KW * Math.pow(bell, 0.9) * cloudFactor(hour)
}

/** 按 SOC 限制 ESS 充放功率（负=充电，正=放电） */
function essPowerLimit(soc: number, desired: number): number {
  if (desired < 0) {
    if (soc >= 92) return 0
    const headroom = Math.max(0, (92 - soc) / 40)
    return -Math.min(ESS_MAX_KW, Math.abs(desired) * headroom)
  }
  if (desired > 0) {
    if (soc <= 22) return 0
    const reserve = Math.max(0, (soc - 22) / 50)
    return Math.min(ESS_MAX_KW, desired * reserve)
  }
  return 0
}

/**
 * 离网功率平衡
 * - 富余：储能充电 + 弃光（降低 PV 显示值）
 * - 缺口：储能放电 → 柴发补位
 * 恒满足：P_pv + P_dg + P_ess = P_load
 */
function balanceOffGrid(
  soc: number,
  pvAvailable: number,
  loadTarget: number,
): { pv: number; load: number; ess: number; dg: number } {
  const load = round1(loadTarget)
  const available = round1(Math.max(0, pvAvailable))

  if (available >= load) {
    const surplus = available - load
    const ess = round1(essPowerLimit(soc, -Math.min(ESS_MAX_KW, surplus)))
    const pv = round1(load - ess)
    return { pv, load, ess, dg: 0 }
  }

  const deficit = load - available
  const ess = round1(essPowerLimit(soc, Math.min(ESS_MAX_KW, deficit)))
  const dg = round1(Math.min(DG_MAX_KW, Math.max(0, load - available - ess)))
  const pv = round1(load - dg - ess)

  return { pv: Math.max(0, pv), load, ess, dg }
}

/** 离网场景逐步仿真 6 小时 */
function simulateMicrogridSeries(points = 36) {
  const now = dayjs()
  let soc = 58
  let cumPv = 12800
  let cumDg = 3240
  let cumUsed = 9480

  const xAxisData: string[] = []
  const pvSeries: number[] = []
  const dgSeries: number[] = []
  const essSeries: number[] = []
  const pvEnergy: number[] = []
  const dgEnergy: number[] = []
  const essEnergy: number[] = []
  let lastSnapshot: MicrogridSnapshot | null = null

  for (let i = points - 1; i >= 0; i--) {
    const t = now.subtract(i * 10, 'minute')
    const hour = t.hour() + t.minute() / 60
    xAxisData.push(t.format('HH:mm'))

    const { pv, load, ess, dg } = balanceOffGrid(soc, pvAvailableKw(hour), loadKw(hour))

    const deltaSoc = (-ess * INTERVAL_HOURS / ESS_CAPACITY_KWH) * 100
    soc = round1(Math.max(20, Math.min(95, soc + deltaSoc)))

    cumPv += pv * INTERVAL_HOURS
    cumDg += dg * INTERVAL_HOURS
    cumUsed += load * INTERVAL_HOURS

    pvSeries.push(pv)
    dgSeries.push(dg)
    essSeries.push(ess)
    pvEnergy.push(round2(pv * INTERVAL_HOURS))
    dgEnergy.push(round2(dg * INTERVAL_HOURS))
    essEnergy.push(round2(Math.max(0, ess) * INTERVAL_HOURS))

    if (i === 0) {
      lastSnapshot = {
        pvKw: pv,
        loadKw: load,
        dgKw: dg,
        essKw: ess,
        soc,
        oilPct: 68.5,
        pvVoltage: 398.2,
        essVoltage: 768.5,
        dgVoltage: 400.1,
        pvEnergyKwh: round1(cumPv),
        dgEnergyKwh: round1(cumDg),
        usedEnergyKwh: round1(cumUsed),
        savingUsd: round2(cumPv * 0.12 + cumDg * 0.08),
      }
    }
  }

  return {
    xAxisData,
    pvSeries,
    dgSeries,
    essSeries,
    pvEnergy,
    dgEnergy,
    essEnergy,
    snapshot: lastSnapshot!,
  }
}

/** 快照 → homepage_batch 点位（拓扑/站点/设备数值一致） */
export function snapshotToPointUpdates(s: MicrogridSnapshot): DemoPointUpdate[] {
  return [
    { id: 1, name: 'PV Energy', values: s.pvEnergyKwh, unit: 'kWh', imgurl: 'icon-pv-energy' },
    { id: 2, name: 'Diesel Energy', values: s.dgEnergyKwh, unit: 'kWh', imgurl: 'icon-diesel-energy' },
    { id: 3, name: 'Energy Used', values: s.usedEnergyKwh, unit: 'kWh', imgurl: 'icon-energy-used' },
    { id: 4, name: 'Saving Billing', values: s.savingUsd, unit: '$', imgurl: 'icon-saving-billing' },
    { id: 5, name: 'PV Power', values: s.pvKw, unit: 'kW', imgurl: 'icon-pv-energy' },
    { id: 6, name: 'Diesel Power', values: s.dgKw, unit: 'kW', imgurl: 'icon-diesel-energy' },
    { id: 7, name: 'ESS Power', values: s.essKw, unit: 'kW', imgurl: 'icon-ess-energy' },
    { id: 8, name: 'P', values: s.pvKw, unit: 'kW' },
    { id: 9, name: 'U', values: s.pvVoltage, unit: 'V' },
    { id: 10, name: 'P', values: s.dgKw, unit: 'kW' },
    { id: 11, name: 'U', values: s.dgVoltage, unit: 'V' },
    { id: 12, name: 'P', values: s.essKw, unit: 'kW' },
    { id: 13, name: 'U', values: s.essVoltage, unit: 'V' },
    { id: 14, name: 'P', values: s.pvKw, unit: 'kW' },
    { id: 15, name: 'P', values: s.loadKw, unit: 'kW' },
    { id: 16, name: 'P', values: s.dgKw, unit: 'kW' },
    { id: 17, name: 'Oil', values: s.oilPct, unit: '%' },
    { id: 18, name: 'P', values: s.essKw, unit: 'kW' },
    { id: 19, name: 'SOC', values: s.soc, unit: '%' },
  ]
}

/** 生成近 6 小时图表 + 当前时刻一致快照 */
export function buildDemoChartData() {
  const sim = simulateMicrogridSeries()

  return {
    xAxisData: sim.xAxisData,
    snapshot: sim.snapshot,
    lineChartSeries: [
      { name: 'PV', data: sim.pvSeries, color: 'rgba(105, 203, 255, 1)' },
      { name: 'DG', data: sim.dgSeries, color: 'rgba(246, 200, 95, 1)' },
      { name: 'ESS', data: sim.essSeries, color: 'rgba(29, 134, 255, 1)' },
    ],
    energyChartSeries: [
      { name: 'PV', data: sim.pvEnergy, color: 'rgba(105, 203, 255, 1)' },
      { name: 'DG', data: sim.dgEnergy, color: 'rgba(246, 200, 95, 1)' },
      { name: 'ESS', data: sim.essEnergy, color: 'rgba(29, 134, 255, 1)' },
    ],
  }
}

/** 初始点位（由仿真末帧导出） */
export const DEMO_HOMEPAGE_UPDATES: DemoPointUpdate[] = snapshotToPointUpdates(
  buildDemoChartData().snapshot,
)

/** 实时刷新：扰动负载后重新离网平衡 */
export function jitterDemoUpdates(base: DemoPointUpdate[]): {
  updates: DemoPointUpdate[]
  snapshot: MicrogridSnapshot
} {
  const get = (id: number) => Number(base.find((p) => p.id === id)?.values ?? 0)
  const soc = round1(Math.max(20, Math.min(95, get(19) + (Math.random() - 0.5) * 0.3)))
  const loadTarget = get(15) + (Math.random() - 0.5) * 6
  const pvAvailable = get(5) + (Math.random() - 0.5) * 12

  const { pv, load, ess, dg } = balanceOffGrid(soc, pvAvailable, loadTarget)

  const snap: MicrogridSnapshot = {
    pvKw: pv,
    loadKw: load,
    dgKw: dg,
    essKw: ess,
    soc,
    oilPct: get(17),
    pvVoltage: round1(get(9) + (Math.random() - 0.5) * 2),
    essVoltage: get(13),
    dgVoltage: get(11),
    pvEnergyKwh: round1(get(1) + Math.random() * 0.4),
    dgEnergyKwh: round1(get(2) + dg * INTERVAL_HOURS),
    usedEnergyKwh: round1(get(3) + load * INTERVAL_HOURS * 0.01),
    savingUsd: round2(get(4) + Math.random() * 0.15),
  }

  return { updates: snapshotToPointUpdates(snap), snapshot: snap }
}
