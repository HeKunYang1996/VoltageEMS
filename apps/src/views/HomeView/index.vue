<template>
  <div class="home">
    <!-- <EnergyBgCopy></EnergyBgCopy> -->
    <div class="home-left">
      <div class="home-left-top">
        <EnergyCard
          class="home-left-top-item"
          v-for="item in energyDashboardList"
          :key="item.id"
          :title="item.title"
          :icon="item.icon"
          :value="item.value"
          :unit="item.unit"
        />
      </div>
      <div class="home-left-middle">
        <!-- <img :src="tuopuSvg" alt="">
          -->
        <HomeBg :data="tuopuData"></HomeBg>
      </div>
      <div class="home-left-bottom">
        <div class="home-left-LineChart">
          <ModuleCard title="Power Curve">
            <LineChart
              :xAxiosOption="xAxiosOption"
              :yAxiosOption="lineChartYAxiosOption"
              :series="lineChartSeries"
            />
          </ModuleCard>
        </div>
        <div class="home-left-EnergyChart">
          <ModuleCard title="Energy Chart">
            <StackedBarChart
              :xAxiosOption="xAxiosOption"
              :yAxiosOption="yAxiosOption"
              :series="exampleSeries"
            />
          </ModuleCard>
        </div>
      </div>
    </div>
    <div class="home-right">
      <div class="home-station">
        <ModuleCard title="Station infomation">
          <div class="home-stationList">
            <div v-for="item in stationInfoList" :key="item.id" class="home-stationItem">
              <EnergyCard
                :title="item.title"
                :icon="item.icon"
                :value="item.value"
                :unit="item.unit"
              />
            </div>
          </div>
        </ModuleCard>
      </div>
      <div class="home-device">
        <ModuleCard title="Device infomation">
          <!-- <div class="home-deviceValue">
              <div class="home-deviceValue-item" v-for="item in deviceInfoList" :key="item.title">
                <span class="deviceValue-item-title">{{ item.title }}:</span>
                <span class="deviceValue-item-value">{{ item.value }}</span>
                &nbsp;
                <span class="deviceValue-item-unit">{{ item.unit }}</span>
              </div>
            </div> -->
          <div class="home-decice-Carousel">
            <el-carousel
              ref="carouselRef"
              :autoplay="false"
              arrow="never"
              indicator-position="none"
              class="home-device-carousel"
            >
              <el-carousel-item
                v-for="(item, index) in deviceInfoList"
                :key="index"
                class="home-device-carousel__item"
              >
                <div class="home-decice-Carousel-item">
                  <div class="home-deviceValue">
                    <div
                      class="home-deviceValue-item"
                      v-for="dataItem in item.data"
                      :key="dataItem.id"
                    >
                      <span class="deviceValue-item-title">{{ dataItem.title }}:</span>
                      <span class="deviceValue-item-value">{{ dataItem.values }}</span>
                      &nbsp;
                      <span class="deviceValue-item-unit">{{ dataItem.unit }}</span>
                    </div>
                  </div>
                  <img :src="item.icon" />
                  <div class="item-name">{{ item.name }}</div>
                </div>
              </el-carousel-item>
            </el-carousel>

            <!-- 自定义左右切换按钮 -->
            <div class="custom-carousel-controls">
              <div class="custom-arrow custom-arrow-left" @click="handlePrev">
                <img :src="arrowLeftImg" alt="Previous" />
              </div>
              <div class="custom-arrow custom-arrow-right" @click="handleNext">
                <img :src="arrowRightImg" alt="Next" />
              </div>
            </div>
          </div>
        </ModuleCard>
      </div>
      <div class="home-alters">
        <ModuleCard title="Alters infomation">
          <div
            class="home-altersList"
            @touchstart="handleAlarmTouchStart"
            @touchmove="handleAlarmTouchMove"
            @touchend="handleAlarmTouchEnd"
          >
            <div v-if="alarmPullDistance > 0" class="home-altersRefreshHint">
              {{ refreshingAlarms ? 'Refreshing...' : alarmPullDistance >= ALARM_PULL_TRIGGER ? 'Release to refresh' : 'Pull to refresh' }}
            </div>
            <div class="home-altersItem" v-for="item in alterInfoList" :key="item.id">
              <div class="alters__item-name" :title="item.deviceName">{{ item.deviceName }}</div>
              <img
                v-if="item.alterLevel == 'Critical Alarm'"
                :src="alterL1"
                class="alters__item-icon"
              />
              <img
                v-else-if="item.alterLevel == 'Warning Alarm'"
                :src="alterL2"
                class="alters__item-icon"
              />
              <img
                v-else-if="item.alterLevel == 'Info Alarm'"
                :src="alterL3"
                class="alters__item-icon"
              />
              <div class="alters__item-msg" :title="item.alterMsg">{{ item.alterMsg }}</div>
            </div>
            <div v-if="!alterInfoList.length && !refreshingAlarms" class="home-altersEmpty">No current alarms</div>
          </div>
        </ModuleCard>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { EnergyCard } from '@/types/home'
import { HOMEPAGE_POINT_IDS } from '@/types/home'
import useWebSocket from '@/composables/useWebSocket'
import { formatNumber } from '@/utils/common'

import ModuleCard from '@/components/card/ModuleCard.vue'
import StackedBarChart from '@/components/charts/StackedBarChart.vue'
import LineChart from '@/components/charts/lineChart.vue'
import { batchQueryHistory } from '@/api/Statistic/overview'
import dayjs from 'dayjs'
import { getRecentHoursRange } from '@/utils/date'
import type { BatchQueryResponse } from '@/types/Statistics/OverView'
import { getCurrentAlarms } from '@/api/alarm'
import type { CurrentAlarmData } from '@/types/alarm'
import type { AlarmMessage } from '@/types/websocket'

import alterL1 from '@/assets/icons/home-alter-L1.svg'
import alterL2 from '@/assets/icons/home-alter-L2.svg'
import alterL3 from '@/assets/icons/home-alter-L3.svg'

import arrowLeftImg from '@/assets/icons/arrow-left.svg'
import arrowRightImg from '@/assets/icons/arrow-right.svg'

import devicePV from '@/assets/icons/device-pv.svg'
import deviceDiesel from '@/assets/icons/device-diesel.svg'
import deviceBattery from '@/assets/images/device-battery.png'

import PVEnergy from '@/assets/icons/icon-pv-energy.svg'
import DieselEnergy from '@/assets/icons/icon-diesel-energy.svg'
import EnergyUsed from '@/assets/icons/icon-energy-used.svg'
import SavingBilling from '@/assets/icons/icon-saving-billing.svg'
import ESSEnergyIcon from '@/assets/icons/icon-ess-energy.svg'

import HomeBg from './HomeBg.vue'

/** imgurl 到图标路径（Energy Card / Station 使用，拓扑图与 Device 无视 imgurl） */
const ICON_BY_IMGURL: Record<string, string> = {
  'icon-pv-energy': PVEnergy,
  'icon-diesel-energy': DieselEnergy,
  'icon-energy-used': EnergyUsed,
  'icon-saving-billing': SavingBilling,
  'icon-ess-energy': ESSEnergyIcon,
}
const iconGlob = import.meta.glob<{ default: string }>('@/assets/icons/*.svg', { eager: true })
for (const [path, mod] of Object.entries(iconGlob)) {
  const m = path.match(/([^/\\]+)\.svg$/)
  const url = (mod?.default ?? (mod as any)) as string
  if (m && typeof url === 'string') ICON_BY_IMGURL[m[1]] = url
}
function getIconByImgurl(imgurl?: string): string {
  if (!imgurl) return PVEnergy
  return ICON_BY_IMGURL[imgurl] ?? PVEnergy
}

/** 点位存储：id -> { id, name, value, unit, imgurl }，由 homepage_batch 推送更新 */
interface PointInfo {
  id: number
  name: string
  values: string | number
  unit: string
  imgurl?: string
}
const pointStore = reactive<Record<number, PointInfo>>({})

/** 无数据时展示 '-'，有数据时限制最多三位小数 */
function displayValue(v: string | number | null | undefined): string {
  if (v === null || v === undefined) return '-'
  return formatNumber(v)
}
/** Energy Card / Station 点位默认 name/unit（无 WebSocket 数据时的占位） */
const HOMEPAGE_POINT_DEFAULTS: Record<number, { name: string; unit: string; imgurl?: string }> = {
  1: { name: 'PV Energy', unit: 'kWh', imgurl: 'icon-pv-energy' },
  2: { name: 'Diesel Energy', unit: 'kWh', imgurl: 'icon-diesel-energy' },
  3: { name: 'Energy Used', unit: 'kWh', imgurl: 'icon-energy-used' },
  4: { name: 'Saving Billing', unit: '$', imgurl: 'icon-saving-billing' },
  5: { name: 'PV Power', unit: 'kW', imgurl: 'icon-pv-energy' },
  6: { name: 'Diesel Power', unit: 'kW', imgurl: 'icon-diesel-energy' },
  7: { name: 'ESS Power', unit: 'kW', imgurl: 'icon-ess-energy' },
  8: { name: 'P', unit: 'kW' },
  9: { name: 'U', unit: 'V' },
  10: { name: 'P', unit: 'kW' },
  11: { name: 'U', unit: 'V' },
  12: { name: 'P', unit: 'kW' },
  13: { name: 'U', unit: 'V' },
  14: { name: 'P', unit: 'kW' },
  15: { name: 'P', unit: 'kW' },
  16: { name: 'P', unit: 'kW' },
  17: { name: 'Oil', unit: '%' },
  18: { name: 'P', unit: 'kW' },
  19: { name: 'SOC', unit: '%' },
}

/** Energy Card：id 1-4 */
const energyDashboardList = computed<EnergyCard[]>(() =>
  HOMEPAGE_POINT_IDS.energyCard.map((id) => {
    const p = pointStore[id]
    const def = HOMEPAGE_POINT_DEFAULTS[id]
    return {
      id,
      title: p?.name ?? def?.name,
      icon: getIconByImgurl(p?.imgurl ?? def?.imgurl),
      value: displayValue(p?.values),
      unit: p?.unit ?? def?.unit,
    }
  }),
)

/** Station information：id 5-7 */
const stationInfoList = computed<EnergyCard[]>(() =>
  HOMEPAGE_POINT_IDS.stationInfo.map((id) => {
    const p = pointStore[id]
    const def = HOMEPAGE_POINT_DEFAULTS[id]
    return {
      id,
      title: p?.name ?? def?.name,
      icon: getIconByImgurl(p?.imgurl ?? def?.imgurl),
      value: displayValue(p?.values),
      unit: p?.unit ?? def?.unit,
    }
  }),
)

// 拓朴图数据
const tuopuData = computed(() => {
  const { topology } = HOMEPAGE_POINT_IDS
  const fmt = (p: PointInfo | undefined, defaultName: string, defaultUnit: string) =>
    p
      ? {
          ...p,
          name: p.name ?? defaultName,
          value: displayValue(p.values),
          unit: p.unit ?? defaultUnit,
        }
      : { id: undefined, name: defaultName, value: '-' as const, unit: defaultUnit }
  return {
    pv: {
      P: fmt(
        pointStore[topology.pv.P],
        HOMEPAGE_POINT_DEFAULTS[14].name ?? '',
        HOMEPAGE_POINT_DEFAULTS[14].unit ?? '',
      ),
    },
    load: {
      P: fmt(
        pointStore[topology.load.P],
        HOMEPAGE_POINT_DEFAULTS[15].name ?? '',
        HOMEPAGE_POINT_DEFAULTS[15].unit ?? 'kW',
      ),
    },
    diesel: {
      p: fmt(
        pointStore[topology.diesel.p],
        HOMEPAGE_POINT_DEFAULTS[16].name ?? '',
        HOMEPAGE_POINT_DEFAULTS[16].unit ?? '',
      ),
      oil: fmt(
        pointStore[topology.diesel.oil],
        HOMEPAGE_POINT_DEFAULTS[17].name ?? '',
        HOMEPAGE_POINT_DEFAULTS[17].unit ?? '%',
      ),
    },
    ess: {
      p: fmt(
        pointStore[topology.ess.p],
        HOMEPAGE_POINT_DEFAULTS[18].name ?? '',
        HOMEPAGE_POINT_DEFAULTS[18].unit ?? 'kW',
      ),
      soc: fmt(
        pointStore[topology.ess.soc],
        HOMEPAGE_POINT_DEFAULTS[19].name ?? '',
        HOMEPAGE_POINT_DEFAULTS[19].unit ?? '%',
      ),
    },
  }
})

/** Device information：PV/Diesel/ESS 轮播（拓扑图与 Device 无视 imgurl，使用固定图标） */
interface DevicePointItem {
  id: number
  title: string
  values: string | number
  unit: string
}
interface DeviceSlide {
  data: DevicePointItem[]
  icon: string
  name: string
}
const deviceIcons = [devicePV, deviceDiesel, deviceBattery]
const deviceNames = ['PV', 'Diesel Generator', 'ESS']
const deviceInfoList = computed<DeviceSlide[]>(() =>
  HOMEPAGE_POINT_IDS.deviceInfo.map((ids, idx) => ({
    data: ids.map((pointId) => {
      const p = pointStore[pointId]
      const def = HOMEPAGE_POINT_DEFAULTS[pointId]
      return {
        id: pointId,
        title: p?.name ?? def?.name,
        values: displayValue(p?.values),
        unit: p?.unit ?? def?.unit,
      }
    }),
    icon: deviceIcons[idx],
    name: deviceNames[idx],
  })),
)

/** 应用 homepage_batch 推送数据*/
function applyHomepageBatch(data: {
  updates?: Array<{ id: number; name: string; values?: number; unit: string; imgurl?: string }>
}) {
  const updates = data?.updates ?? []

  for (const u of updates) {
    pointStore[u.id] = {
      id: u.id,
      name: u.name,
      values: u.values !== null && u.values !== undefined ? formatNumber(u.values) : '-',
      unit: u.unit,
      imgurl: u.imgurl,
    }
  }
}

/** 首页 WebSocket 订阅：进入订阅、离开取消 */
useWebSocket(
  {
    source: 'homepage',
    interval: 1000,
  },
  {
    onBatchDataUpdate: (data: any) => {
      if (data?.updates?.length) {
        applyHomepageBatch(data)
      }
    },
    onAlarm: handleHomeAlarm,
  },
)

interface HomeAlarmItem {
  id: number | string
  deviceName: string
  alterLevel: string
  alterMsg: string
}

type HomeAlarmSource = CurrentAlarmData | AlarmMessage['data']

const alterInfoList = ref<HomeAlarmItem[]>([])
const refreshingAlarms = ref(false)
const alarmPullDistance = ref(0)
const alarmTouchStartY = ref<number | null>(null)
const ALARM_PULL_TRIGGER = 48
let alarmListVersion = 0
const alarmLevelText: Record<number, string> = {
  1: 'Critical Alarm',
  2: 'Warning Alarm',
  3: 'Info Alarm',
}

const toHomeAlarm = (alarm: HomeAlarmSource): HomeAlarmItem => {
  if ('warning_level' in alarm) {
    const pointName = alarm.point_name || `Point ${alarm.point_id}`
    const unit = alarm.unit || ''
    const value = alarm.current_value ?? '-'
    const threshold = alarm.threshold_value ?? '-'
    const fallbackMessage = `${pointName}: ${value}${unit} ${alarm.operator || ''} ${threshold}${unit} (Rule: ${alarm.rule_name || 'Alarm'})`

    return {
      id: alarm.id,
      deviceName: alarm.device_name || alarm.channel_id.toString() || '-',
      alterLevel: alarmLevelText[alarm.warning_level] || 'Alarm',
      alterMsg: fallbackMessage,
    }
  }

  return {
    id: alarm.alarm_id,
    deviceName: alarm.device_name || alarm.device || alarm.channel_id?.toString() || '-',
    alterLevel: alarmLevelText[alarm.level] || 'Alarm',
    alterMsg: alarm.message.trim() || 'Alarm',
  }
}

const fetchHomeAlarms = async () => {
  const requestVersion = alarmListVersion
  refreshingAlarms.value = true
  try {
    const response = await getCurrentAlarms({ page: 1, page_size: 8 })
    if (response.success && requestVersion === alarmListVersion) {
      alterInfoList.value = (response.data?.list ?? []).map(toHomeAlarm)
    }
  } catch (error) {
    console.error('Failed to fetch current alarms:', error)
  } finally {
    refreshingAlarms.value = false
    alarmPullDistance.value = 0
  }
}

function handleHomeAlarm(alarm: AlarmMessage['data']) {
  alarmListVersion += 1
  const id = alarm.alarm_id
  if (alarm.status === 0) {
    alterInfoList.value = alterInfoList.value.filter((item) => String(item.id) !== String(id))
    return
  }
  alterInfoList.value = [
    toHomeAlarm(alarm),
    ...alterInfoList.value.filter((item) => String(item.id) !== String(id)),
  ].slice(0, 8)
}

const handleAlarmTouchStart = (event: TouchEvent) => {
  if (refreshingAlarms.value) return
  alarmTouchStartY.value = event.touches[0]?.clientY ?? null
}

const handleAlarmTouchMove = (event: TouchEvent) => {
  if (alarmTouchStartY.value === null || refreshingAlarms.value) return
  const distance = (event.touches[0]?.clientY ?? 0) - alarmTouchStartY.value
  if (distance > 0) alarmPullDistance.value = Math.min(distance * 0.5, 80)
}

const handleAlarmTouchEnd = () => {
  if (alarmPullDistance.value >= ALARM_PULL_TRIGGER) fetchHomeAlarms()
  else alarmPullDistance.value = 0
  alarmTouchStartY.value = null
}

fetchHomeAlarms()

// ─── 图表数据（Power Curve + Energy Chart）────────────────────
const chartXAxisData = ref<string[]>([])

const lineChartSeries = ref([
  { name: 'PV', data: [] as number[], color: 'rgba(105, 203, 255, 1)' },
  { name: 'DG', data: [] as number[], color: 'rgba(246, 200, 95, 1)' },
  { name: 'ESS', data: [] as number[], color: 'rgba(29, 134, 255, 1)' },
])

const exampleSeries = ref([
  { name: 'PV', data: [] as number[], color: 'rgba(105, 203, 255, 1)' },
  { name: 'DG', data: [] as number[], color: 'rgba(246, 200, 95, 1)' },
  { name: 'ESS', data: [] as number[], color: 'rgba(29, 134, 255, 1)' },
])

const xAxiosOption = computed(() => ({ xAxiosData: chartXAxisData.value }))
const yAxiosOption = { yUnit: 'kWh' }
const lineChartYAxiosOption = { yUnit: 'kW' }

const fmtLabel = (ts: string) => dayjs(ts).format('HH:mm')
const fmtVal = (v: number | null | undefined) => Number(Number(v ?? 0).toFixed(3))

const fetchHomeChartData = async () => {
  const range = getRecentHoursRange(6)
  try {
    const res = await batchQueryHistory({
      start_time: range.start!,
      end_time: range.end!,
      limit_per_series: 500,
      series: [
        { redis_key: 'inst:4:M', point_id: '7' },  // Power PV
        { redis_key: 'inst:2:M', point_id: '1' },  // Power DG
        { redis_key: 'inst:1:M', point_id: '5' },  // Power ESS
        { redis_key: 'inst:4:M', point_id: '15' }, // Energy PV
        { redis_key: 'inst:2:M', point_id: '2' },  // Energy DG
        { redis_key: 'inst:1:M', point_id: '9' },  // Energy ESS
      ],
    })

    const responses: BatchQueryResponse[] = res.data?.series ?? []
    const find = (rk: string, pid: string) =>
      responses.find((r) => r.redis_key === rk && r.point_id === pid)

    const allTs = new Set<string>()
    responses.forEach((r) => (r.data ?? []).forEach((p) => allTs.add(p.timestamp)))
    const sorted = [...allTs].sort((a, b) => new Date(a).getTime() - new Date(b).getTime())
    chartXAxisData.value = sorted.map(fmtLabel)

    const makeVals = (s: BatchQueryResponse | undefined) => {
      if (!s) return sorted.map(() => 0)
      const map = new Map((s.data ?? []).map((p) => [p.timestamp, p.value]))
      return sorted.map((ts) => fmtVal(map.get(ts)))
    }

    lineChartSeries.value = [
      { name: 'PV', data: makeVals(find('inst:4:M', '7')), color: 'rgba(105, 203, 255, 1)' },
      { name: 'DG', data: makeVals(find('inst:2:M', '1')), color: 'rgba(246, 200, 95, 1)' },
      { name: 'ESS', data: makeVals(find('inst:1:M', '5')), color: 'rgba(29, 134, 255, 1)' },
    ]

    exampleSeries.value = [
      { name: 'PV', data: makeVals(find('inst:4:M', '15')), color: 'rgba(105, 203, 255, 1)' },
      { name: 'DG', data: makeVals(find('inst:2:M', '2')), color: 'rgba(246, 200, 95, 1)' },
      { name: 'ESS', data: makeVals(find('inst:1:M', '9')), color: 'rgba(29, 134, 255, 1)' },
    ]
  } catch (error) {
    console.error('Failed to fetch home chart data:', error)
  }
}

onMounted(() => {
  fetchHomeChartData()
})

// Carousel引用
const carouselRef = ref()

// 切换到上一张
const handlePrev = () => {
  carouselRef.value?.prev()
}

// 切换到下一张
const handleNext = () => {
  console.log('next')

  carouselRef.value?.next()
}
</script>

<style scoped lang="scss">
.home {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: space-between;
  z-index: 2;

  &::before {
    content: '';
    position: absolute;
    top: -0.2rem;
    left: -0.2rem;
    width: calc(100% + 0.4rem);
    height: calc(100% + 0.4rem);
    background: url('@/assets/images/home-bg.png') no-repeat center center;
    background-size: 100% 100%;
    z-index: 1;
  }

  .home-left {
    position: relative;
    z-index: 2;
    width: calc(100% - 3.9rem);
    height: 100%;
    margin-right: 0.2rem;
    display: flex;
    flex-direction: column;
    justify-content: space-between;

    .home-left-top {
      width: 100%;
      height: 0.8rem;
      padding-top: 0.1rem;
      display: flex;
      justify-content: space-between;
      z-index: 1;

      .home-left-top-item {
        height: 0.7rem;
      }
    }

    .home-left-middle {
      width: 100%;
      height: calc(69% - 1.2rem);
      flex: 1;
      // background-image: url('@/assets/images/tuopu.png');
      // background-size: 100% 100%;
      // background-repeat: no-repeat;
      // background-position: center;

      img {
        width: 100%;
        height: 100%;
        object-fit: contain;
      }
    }

    .home-left-bottom {
      width: 100%;
      height: 30.89%;
      display: flex;
      justify-content: space-between;

      .home-left-EnergyChart {
        width: calc((100% - 0.2rem) / 2);
        height: 100%;
      }

      .home-left-LineChart {
        width: calc((100% - 0.2rem) / 2);
        height: 100%;
      }
    }
  }

  .home-right {
    position: relative;
    z-index: 2;

    width: 3.7rem;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 0.2rem;

    .home-station {
      width: 100%;
      height: 36.75%;

      .home-stationList {
        height: 100%;
        padding-top: 0.2rem;

        .home-stationItem {
          height: 33.33%;
          padding-top: 0.12rem;
          padding-bottom: 0.13rem;
          border-bottom: 0.01rem dashed var(--vt-border-color-dashed);

          &:last-child {
            border-bottom: none;
            // padding-bottom: 0;
            margin-bottom: 0;
          }
        }
      }
    }

    .home-device {
      width: 100%;
      height: 28.27%;

      .home-decice-Carousel {
        height: 100%;
        width: 100%;

        .home-decice-Carousel-item {
          height: 100%;
          width: 100%;
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;

          .home-deviceValue {
            width: 100%;
            padding: 0.15rem 0;
            margin-bottom: 0.2rem;
            border-bottom: 0.01rem dashed var(--vt-border-color-dashed);
            display: flex;
            justify-content: space-between;

            .home-deviceValue-item {
              width: 50%;
              display: flex;
              align-items: flex-end;
              justify-content: center;
              font-size: 0.16rem;
              font-weight: 400;
              color: rgba(255, 255, 255, 0.6);
              height: 0.16rem;

              .deviceValue-item-title {
                font-size: 0.16rem;
                font-weight: 600;
                margin-right: 0.09rem;
              }

              .deviceValue-item-value {
                font-size: 0.22rem;
                font-weight: 700;
                color: var(--vt-text-primary);
                line-height: 0.26rem;
              }

              .deviceValue-item-unit {
                font-size: 0.14rem;
                font-weight: 400;
              }
            }
          }

          img {
            width: 1.2rem;
            height: 0.73rem;
            object-fit: contain;
            margin-bottom: 0.05rem;
          }

          .item-name {
            font-size: 0.18rem;
            font-weight: 500;
            line-height: 100%;
            letter-spacing: 0%;
          }
        }
      }
    }

    .home-alters {
      height: 30.89%;
      width: 100%;

      .home-altersList {
        height: 100%;
        overflow-y: scroll;
        touch-action: pan-y;
        // 默认隐藏滚动条
        scrollbar-width: none;
        /* Firefox */
        -ms-overflow-style: none;
        /* IE and Edge */

        // Webkit浏览器隐藏滚动条
        &::-webkit-scrollbar {
          width: 0;
          height: 0;
        }

        .home-altersRefreshHint,
        .home-altersEmpty {
          padding: 0.12rem 0;
          color: rgba(255, 255, 255, 0.55);
          font-size: 0.13rem;
          text-align: center;
        }

        // 鼠标悬停时显示滚动条
        &:hover {
          scrollbar-width: auto;
          /* Firefox */
          -ms-overflow-style: auto;
          /* IE and Edge */

          &::-webkit-scrollbar {
            width: 0.04rem;
            height: 0.04rem;
          }

          &::-webkit-scrollbar-thumb {
            border-radius: 0.02rem;
          }
        }

        .home-altersItem {
          min-height: 0.6rem;
          border-bottom: 0.01rem solid rgba(255, 255, 255, 0.2);
          display: grid;
          grid-template-columns: 0.4rem 0.46rem minmax(0, 1fr);
          column-gap: 0.1rem;
          align-items: center;

          .alters__item-name {
            font-size: 0.16rem;
            font-weight: 700;
            line-height: 0.16rem;
            overflow: hidden;
            word-break: break-word;
            display: -webkit-box;
            -webkit-box-orient: vertical;
            -webkit-line-clamp: 2;
          }

          .alters__item-icon {
            width: 0.46rem;
            height: 0.2rem;
            object-fit: contain;
          }

          .alters__item-msg {
            min-width: 0;
            font-size: 0.14rem;
            line-height: 0.16rem;
            font-weight: 400;
            overflow: hidden;
            word-break: break-word;
            display: -webkit-box;
            -webkit-box-orient: vertical;
            -webkit-line-clamp: 2;
          }


        }
      }
    }
  }
}

:deep(.home-device-carousel.el-carousel),
:deep(.home-device-carousel .el-carousel__container),
:deep(.home-device-carousel__item.el-carousel__item) {
  height: 100%;
}

:deep(.home-device-carousel__item.el-carousel__item) {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

// 自定义carousel控制按钮样式
.custom-carousel-controls {
  width: 100%;
  height: 0.32rem;
  position: absolute;
  top: 50%;
  left: 0;
  right: 0;
  transform: translateY(-50%);
  z-index: 999;
}

.custom-arrow {
  position: absolute;
  width: 0.32rem;
  height: 0.32rem;
  cursor: pointer;

  img {
    width: 0.32rem;
    height: 0.32rem;
    object-fit: contain;
  }

  // &:hover {
  //   background-color: rgba(84, 98, 140, 1);
  // }
}

.custom-arrow-left {
  left: 0.1rem;
}

.custom-arrow-right {
  right: 0.1rem;
}
</style>
