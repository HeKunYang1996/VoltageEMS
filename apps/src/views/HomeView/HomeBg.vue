<template>
  <div class="energy-bg">
    <svg
      class="svg"
      width="1310"
      height="569"
      viewBox="0 0 1310 569"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <!-- Figma cut 4045:1733: keep the complete topology inside its 1310×569 frame. -->
      <g :clip-path="`url(#${flowId}-frame)`">
        <g class="topology-devices">
          <image :href="devicePv" data-device="pv" x="197" y="-2" width="534" height="233" />
          <image :href="deviceLoad" data-device="load" x="318" y="266" width="359" height="306" />
          <image
            :href="deviceBess"
            data-device="bess"
            x="657"
            y="172"
            width="301"
            height="216"
            preserveAspectRatio="xMidYMid slice"
          />
          <image
            :href="deviceDiesel"
            data-device="diesel"
            x="956"
            y="366"
            width="155"
            height="140"
          />
        </g>
        <g
          transform="translate(555 181.4519) rotate(-2.32068)"
          class="flow-track"
          :class="{ 'flow-track--inactive': !isActive(flows[0].direction) }"
        >
          <image :href="pvTrack" x="-36" y="-28" width="206.643" height="166.444" />
        </g>
        <image
          :href="loadTrack"
          x="628"
          y="316.3542"
          width="158.3"
          height="135"
          class="flow-track"
          :class="{ 'flow-track--inactive': !isActive(flows[1].direction) }"
        />
        <image
          :href="dieselTrack"
          x="823"
          y="335.3542"
          width="171.5"
          height="139.5"
          class="flow-track"
        />
        <g
          v-for="flow in flows"
          :key="flow.id"
          :transform="flow.transform"
          :data-flow="flow.id"
          :data-direction="flow.direction"
        >
          <g v-if="isActive(flow.direction)">
            <g
              :transform="
                flow.direction === 'reverse' ? 'translate(58.4476 43.6887) rotate(180)' : undefined
              "
            >
              <!-- Both ends follow the V profile, so the repeated artwork flows through matching cuts. -->
              <g :clip-path="`url(#${flowId}-${flow.id})`">
                <g
                  class="flow-arrows"
                  :style="{
                    '--flow-repeat-x': `${-FLOW_REPEAT_X}px`,
                    '--flow-repeat-y': `${-FLOW_REPEAT_Y}px`,
                  }"
                >
                  <image
                    v-for="copy in [-1, 0, 1, 2]"
                    :key="copy"
                    :href="flow.asset"
                    :x="copy * FLOW_REPEAT_X"
                    :y="copy * FLOW_REPEAT_Y"
                    width="58.4476"
                    height="43.6887"
                  />
                </g>
              </g>
            </g>
          </g>
        </g>
        <g v-for="card in cards" :key="card.id" :data-card="card.id">
          <g :transform="`translate(${card.x} ${card.y})`">
            <path
              :d="cardPath"
              fill="var(--vt-bg-topology-card)"
              :stroke="`url(#${flowId}-card-border)`"
              stroke-width="2"
            />
            <text :x="card.textX" y="37.5" class="svg-card-title">{{ card.title }}</text>
            <text
              v-for="(point, index) in card.rows"
              :key="index"
              :x="card.textX"
              :y="card.rowY + index * 21"
              class="svg-card-value"
            >
              <tspan>{{ point?.name }}:</tspan>
              <tspan dx="8" class="svg-card-number">{{ point?.value }}</tspan>
              <tspan dx="5" class="svg-card-unit">{{ point?.unit }}</tspan>
            </text>
          </g>
          <circle
            :cx="card.anchor[0]"
            :cy="card.anchor[1]"
            r="6"
            fill="white"
            fill-opacity="0.8"
            stroke="white"
            stroke-opacity="0.2"
            stroke-width="20"
          />
          <path :d="card.connector" stroke="white" stroke-opacity="0.3" />
        </g>
      </g>
      <defs>
        <clipPath :id="`${flowId}-frame`" clipPathUnits="userSpaceOnUse">
          <rect width="1310" height="569" />
        </clipPath>
        <clipPath
          v-for="flow in flows"
          :id="`${flowId}-${flow.id}`"
          :key="flow.id"
          clipPathUnits="userSpaceOnUse"
        >
          <!-- Follow the first V's inner edge and the last V's middle, as in the marked reference. -->
          <path
            d="M9.81689 0L54.31689 24L55.5397 36.4682L44.5 42.1056L0 18.1056L11.0397 12.4682Z"
          />
        </clipPath>
        <linearGradient :id="`${flowId}-card-border`" x1="0" y1="0" x2="1" y2="1">
          <stop stop-color="#C7C7C7" />
          <stop offset="0.30251" stop-color="#0035A2" stop-opacity="0" />
          <stop offset="0.718599" stop-color="#0035A2" stop-opacity="0.344221" />
          <stop offset="0.990385" stop-color="#C7C7C7" />
        </linearGradient>
      </defs>
    </svg>
  </div>
</template>
<script setup lang="ts">
import { computed, useId } from 'vue'
import { flowDirection, type FlowDirection } from '@/utils/homeEnergyFlow'
import devicePv from '@/assets/images/home-topology/device-pv.png'
import deviceLoad from '@/assets/images/home-topology/device-load.png'
import deviceBess from '@/assets/images/home-topology/device-bess-skid.png'
import deviceDiesel from '@/assets/images/home-topology/device-diesel.png'
import pvTrack from '@/assets/images/home-topology/pv-track.svg'
import loadTrack from '@/assets/images/home-topology/load-track.svg'
import dieselTrack from '@/assets/images/home-topology/diesel-track.svg'
import pvArrows from '@/assets/images/home-topology/pv-arrows.svg'
import loadArrows from '@/assets/images/home-topology/load-arrows.svg'
interface TopologyPoint {
  id?: number
  name?: string
  value?: unknown
  rawValue?: unknown
  unit?: string
}
const props = defineProps<{
  data: {
    pv: { P?: TopologyPoint }
    load: { P?: TopologyPoint }
    diesel: { p?: TopologyPoint; oil?: TopologyPoint }
    ess: { p?: TopologyPoint; soc?: TopologyPoint }
  }
}>()
// Four V-shaped chevrons per Figma image. Repeat by four measured tip intervals,
// rather than the axis-aligned image width, to preserve the isometric spacing.
const FLOW_REPEAT_X = 53.6488
const FLOW_REPEAT_Y = 29.1544
const flowId = `home-energy-${useId()}`
const isActive = (direction: FlowDirection) => direction === 'forward' || direction === 'reverse'
// 139×99 card, radius 20 on the top-left and bottom-right corners.
const cardPath = 'M20 0H139V79A20 20 0 0 1 119 99H0V20A20 20 0 0 1 20 0Z'

// Use the standalone cut (4045:1733), rather than coordinates from the composed homepage.
const lanes = [
  { id: 'pv', asset: pvArrows, transform: 'translate(595 198.3542)' },
  {
    id: 'load',
    asset: loadArrows,
    transform: 'matrix(-0.9965968 0.0824303 0.0824303 0.9965968 732.2487 346.3542)',
  },
]

const flows = computed(() => {
  const directions = [flowDirection(props.data.pv.P), flowDirection(props.data.load.P)]
  return lanes.map((lane, index) => ({ ...lane, direction: directions[index]! }))
})

// Card dimensions and anchors are separate from the live point values.
const cards = computed(() => [
  {
    id: 'pv',
    title: 'PV',
    x: 63.5,
    y: 38.5,
    width: 139,
    height: 99,
    textX: 19.5,
    rowY: 67.5,
    rows: [props.data.pv.P],
    anchor: [306, 121],
    connector: 'M203 95H285L305.5 119',
  },
  {
    id: 'load',
    title: 'Load',
    x: 101.5,
    y: 294.5,
    width: 139,
    height: 99,
    textX: 19.5,
    rowY: 67.5,
    rows: [props.data.load.P],
    anchor: [392, 378],
    connector: 'M241 352H371L391.5 376',
  },
  {
    id: 'ess',
    title: 'BESS',
    x: 1016.5,
    y: 180.5,
    width: 139,
    height: 99,
    textX: 29.5,
    rowY: 64.5,
    rows: [props.data.ess.p, props.data.ess.soc],
    anchor: [820, 232],
    connector: 'M1016 206H842L821.5 230',
  },
  {
    id: 'diesel',
    title: 'Diesel',
    x: 1113.5,
    y: 396.5,
    width: 139,
    height: 99,
    textX: 29.5,
    rowY: 64.5,
    rows: [props.data.diesel.p, { ...props.data.diesel.oil, name: 'Fuel' }],
    anchor: [994, 428],
    connector: 'M1113 454H1015L994.5 430',
  },
])
</script>

<style lang="scss" scoped>
.energy-bg {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;

  .svg {
    width: 100%;
    height: 100%;
    display: block;
    overflow: hidden;
  }
}
.topology-devices {
  filter: drop-shadow(0 8px 12px rgba(0, 0, 0, 0.25));
}
.svg-card-title,
.svg-card-value,
.svg-card-number,
.svg-card-unit {
  font-family: var(--vt-font-family-base), Arial, sans-serif;
  fill: var(--vt-text-primary);
}
.flow-track {
  transition: opacity 0.2s ease;
}
.flow-track--inactive {
  opacity: 0.25;
  filter: grayscale(1);
}
.flow-arrows {
  animation: energy-flow 2.4s linear infinite;
}
@keyframes energy-flow {
  from {
    transform: translate(var(--flow-repeat-x), var(--flow-repeat-y));
  }
  to {
    transform: translate(0, 0);
  }
}
.svg-card-title {
  font-size: 16px;
  font-weight: 700;
}
.svg-card-value {
  font-size: 14px;
  font-weight: 400;
}
.svg-card-number {
  font-size: 20px;
  font-weight: 700;
}
.svg-card-unit {
  font-size: 12px;
  fill: var(--vt-text-unit);
}
</style>
