<template>
  <div class="voltage-class forecast-view">
    <!-- 顶部工具栏 -->
    <div class="forecast__toolbar">
      <div class="forecast__toolbar-left">
        <span class="forecast__title">Forecast</span>
        <span class="forecast__subtitle">Next 24 Hours</span>
      </div>
      <div class="forecast__toolbar-right">
        <el-button class="forecast__btn-setting" @click="handleForecastSettings">
          <el-icon class="btn-icon"><Setting /></el-icon>
          Forecast Settings
        </el-button>
        <el-button class="forecast__btn-export" type="primary" @click="handleExport">
          <el-icon class="btn-icon"><Download /></el-icon>
          Export
        </el-button>
      </div>
    </div>

    <!-- 主体内容 -->
    <div class="forecast__body">
      <!-- 左侧主内容区 -->
      <div class="forecast__main">
        <!-- 摘要卡片 -->
        <div class="forecast__summary">
          <div class="section-header">
            <span class="section-title">Forecast Summary</span>
            <span class="section-subtitle">Next 24 Hours</span>
          </div>
          <div class="summary-cards">
            <div class="summary-card">
              <div class="summary-card__icon pv-icon">
                <el-icon><Sunny /></el-icon>
              </div>
              <div class="summary-card__content">
                <div class="summary-card__label">Total PV Generation</div>
                <div class="summary-card__value">
                  18.6<span class="summary-card__unit">MWh</span>
                </div>
                <div class="summary-card__trend up">
                  vs Yesterday <span>▲ 8.3%</span>
                </div>
              </div>
            </div>

            <div class="summary-card">
              <div class="summary-card__icon load-icon">
                <el-icon><TrendCharts /></el-icon>
              </div>
              <div class="summary-card__content">
                <div class="summary-card__label">Total Load</div>
                <div class="summary-card__value">
                  16.2<span class="summary-card__unit">MWh</span>
                </div>
                <div class="summary-card__trend up">
                  vs Yesterday <span>▲ 5.1%</span>
                </div>
              </div>
            </div>

            <div class="summary-card">
              <div class="summary-card__icon self-icon">
                <el-icon><DataAnalysis /></el-icon>
              </div>
              <div class="summary-card__content">
                <div class="summary-card__label">Self-consumption Rate</div>
                <div class="summary-card__value">
                  87<span class="summary-card__unit">%</span>
                </div>
                <div class="summary-card__trend up">
                  vs Yesterday <span>▲ 2.7%</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- PV 功率预测图表 -->
        <div class="forecast__chart-section">
          <div class="section-header">
            <div class="section-title-row">
              <span class="section-title">PV Power Forecast</span>
              <span class="section-subtitle">Next 24 Hours</span>
            </div>
            <div class="chart-legend">
              <span class="legend-item">
                <span class="legend-line solid pv"></span>Actual
              </span>
              <span class="legend-item">
                <span class="legend-line dashed"></span>Forecast
              </span>
              <span class="legend-item">
                <span class="legend-band pv-band"></span>P10-P90 Range
              </span>
            </div>
          </div>
          <div class="forecast__chart-wrap">
            <ForecastLineChart
              :xAxiosOption="chartXOption"
              :yAxiosOption="{ yUnit: 'kW' }"
              :series="pvChartSeries"
              :splitLines="splitLines"
              :gridConfig="{ left: 55, right: 20, top: 30, bottom: 25 }"
              :showAreaStyle="true"
              :showToolbox="false"
              title="PV Power Forecast"
            />
          </div>
        </div>

        <!-- 负荷预测图表 -->
        <div class="forecast__chart-section">
          <div class="section-header">
            <div class="section-title-row">
              <span class="section-title">Load Forecast</span>
              <span class="section-subtitle">Next 24 Hours</span>
            </div>
            <div class="chart-legend">
              <span class="legend-item">
                <span class="legend-line solid load"></span>Actual
              </span>
              <span class="legend-item">
                <span class="legend-line dashed"></span>Forecast
              </span>
              <span class="legend-item">
                <span class="legend-band load-band"></span>P10-P90 Range
              </span>
            </div>
          </div>
          <div class="forecast__chart-wrap">
            <ForecastLineChart
              :xAxiosOption="chartXOption"
              :yAxiosOption="{ yUnit: 'kW' }"
              :series="loadChartSeries"
              :splitLines="splitLines"
              :gridConfig="{ left: 55, right: 20, top: 30, bottom: 25 }"
              :showAreaStyle="true"
              :showToolbox="false"
              title="Load Forecast"
            />
          </div>
        </div>
      </div>

      <!-- 右侧面板 -->
      <div class="forecast__panel">
        <!-- AI 建议 -->
        <div class="panel-card">
          <div class="panel-card__header">
            <div class="panel-card__title">
              <span class="ai-badge">AI</span>
              <span>AI Suggestions</span>
            </div>
            <el-button link class="view-all-btn">View All</el-button>
          </div>
          <div class="panel-card__body">
            <div
              v-for="suggestion in aiSuggestions"
              :key="suggestion.id"
              class="suggestion-item"
              :class="suggestion.type"
            >
              <div class="suggestion-item__icon" :class="suggestion.type">
                <el-icon v-if="suggestion.type === 'warning'"><Lightning /></el-icon>
                <el-icon v-else><Box /></el-icon>
              </div>
              <div class="suggestion-item__content">
                <div class="suggestion-item__title">{{ suggestion.title }}</div>
                <div class="suggestion-item__desc">{{ suggestion.desc }}</div>
                <div class="suggestion-item__impact">Impact: {{ suggestion.impact }}</div>
              </div>
              <el-button
                size="small"
                class="suggestion-item__btn"
                :class="suggestion.type"
              >
                View Details
              </el-button>
            </div>
          </div>
        </div>

        <!-- 推荐操作 -->
        <div class="panel-card">
          <div class="panel-card__header">
            <div class="panel-card__title">
              <span>Recommended Actions</span>
            </div>
            <el-button link class="view-all-btn">View All</el-button>
          </div>
          <div class="panel-card__body">
            <div v-for="action in recommendedActions" :key="action.id" class="action-item">
              <div class="action-item__icon" :class="action.iconType">
                <el-icon v-if="action.iconType === 'battery'"><Box /></el-icon>
                <el-icon v-else><Lightning /></el-icon>
              </div>
              <div class="action-item__content">
                <div class="action-item__title">{{ action.title }}</div>
                <div class="action-item__desc">{{ action.desc }}</div>
                <div class="action-item__meta">{{ action.meta }}</div>
              </div>
              <el-button size="small" class="action-item__btn">
                {{ action.btnText }}
              </el-button>
            </div>
          </div>
        </div>

        <!-- 预测准确率 -->
        <div class="panel-card accuracy-card">
          <div class="panel-card__header">
            <div class="panel-card__title">
              <span>Forecast Accuracy</span>
              <span class="title-sub">(Last 7 Days)</span>
            </div>
          </div>
          <div class="panel-card__body">
            <div class="accuracy-metrics">
              <div class="accuracy-item">
                <div class="accuracy-item__label">PV Forecast</div>
                <div class="accuracy-item__value pv">92.3%</div>
              </div>
              <div class="accuracy-item">
                <div class="accuracy-item__label">Load Forecast</div>
                <div class="accuracy-item__value load">89.7%</div>
              </div>
              <div class="accuracy-item overall">
                <div class="accuracy-item__label">Overall Accuracy</div>
                <div class="accuracy-item__value overall-val">91.0%</div>
              </div>
            </div>
            <el-button link class="view-details-btn">View Details →</el-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  Setting,
  Download,
  Sunny,
  TrendCharts,
  DataAnalysis,
  Lightning,
  Box,
} from '@element-plus/icons-vue'
import ForecastLineChart from '@/components/charts/ForecastLineChart.vue'
import type { ForecastSeriesData } from '@/components/charts/ForecastLineChart.vue'

const timeLabels = Array.from({ length: 25 }, (_, i) => `${String(i).padStart(2, '0')}:00`)
const chartXOption = { xAxiosData: timeLabels }
const splitLines = [{ index: 14, label: 'Now' }]

// PV 功率模拟数据（以 14:00 为当前时刻）
const pvActualData: (number | null)[] = [
  0, 0, 0, 0, 0, 15, 80, 220, 420, 610, 760, 860, 940, 880,
  null, null, null, null, null, null, null, null, null, null, null,
]
const pvForecastData: (number | null)[] = [
  null, null, null, null, null, null, null, null, null, null, null, null, null, null,
  320, 280, 190, 110, 50, 20, 5, 0, 0, 0, 0,
]

const pvChartSeries: ForecastSeriesData[] = [
  {
    name: 'PV Power',
    segments: [
      { name: 'Actual', data: pvActualData as number[], color: '#69cbff', lineType: 'solid' },
      { name: 'Forecast', data: pvForecastData as number[], color: '#ff6900', lineType: 'dashed' },
    ],
  },
]

// 负荷模拟数据（早晚高峰曲线）
const loadActualData: (number | null)[] = [
  380, 340, 320, 310, 305, 320, 390, 480, 590, 660, 700, 720, 740, 760,
  null, null, null, null, null, null, null, null, null, null, null,
]
const loadForecastData: (number | null)[] = [
  null, null, null, null, null, null, null, null, null, null, null, null, null, null,
  870, 920, 880, 840, 800, 760, 700, 640, 560, 460, 390,
]

const loadChartSeries: ForecastSeriesData[] = [
  {
    name: 'Load',
    segments: [
      { name: 'Actual', data: loadActualData as number[], color: '#a78bfa', lineType: 'solid' },
      { name: 'Forecast', data: loadForecastData as number[], color: '#ff6900', lineType: 'dashed' },
    ],
  },
]

const aiSuggestions = [
  {
    id: 1,
    type: 'warning',
    title: 'PV power drop predicted in 1h 45m',
    desc: 'Prepare ESS discharge or connect backup source.',
    impact: '-1.2 MWh',
  },
  {
    id: 2,
    type: 'info',
    title: 'Load peak expected today 14:00 - 16:00',
    desc: 'Recommend charging ESS before 12:00.',
    impact: 'Reduce peak by 15%',
  },
]

const recommendedActions = [
  {
    id: 1,
    iconType: 'battery',
    title: 'Charge ESS before load peak',
    desc: 'Recommended: Before 12:00',
    meta: 'Target SoC: 90%',
    btnText: 'Schedule',
  },
  {
    id: 2,
    iconType: 'lightning',
    title: 'Prepare backup for PV drop',
    desc: 'Recommended: Before 14:00',
    meta: 'Duration: ~3h',
    btnText: 'Prepare',
  },
]

function handleForecastSettings() {}
function handleExport() {}
</script>

<style lang="scss" scoped>
.voltage-class.forecast-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  //padding: 0.2rem 0.24rem;
  gap: 0.16rem;
  overflow: hidden;

  // ── 顶部工具栏 ──
  .forecast__toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;

    .forecast__toolbar-left {
      display: flex;
      align-items: baseline;
      gap: 0.1rem;

      .forecast__title {
        font-size: var(--vt-font-size-xl);
        font-weight: var(--vt-font-weight-bold);
        color: var(--vt-text-primary);
        font-family: var(--vt-font-family-heading);
      }

      .forecast__subtitle {
        font-size: var(--vt-font-size-sm);
        color: var(--vt-text-secondary);
      }
    }

    .forecast__toolbar-right {
      display: flex;
      align-items: center;
      gap: 0.1rem;

      .forecast__btn-setting {
        background: var(--vt-bg-glass);
        border: 1px solid var(--vt-border-color);
        color: var(--vt-text-primary);
        font-size: var(--vt-font-size-sm);
        height: 0.32rem;

        &:hover {
          background: var(--vt-bg-glass-strong);
          border-color: var(--vt-border-color-strong);
        }

        .btn-icon {
          margin-right: 0.05rem;
        }
      }

      .forecast__btn-export {
        background: var(--vt-color-primary);
        border-color: var(--vt-color-primary);
        font-size: var(--vt-font-size-sm);
        height: 0.32rem;

        &:hover {
          background: var(--vt-color-primary-hover);
        }

        .btn-icon {
          margin-right: 0.05rem;
        }
      }
    }
  }

  // ── 主体 ──
  .forecast__body {
    display: flex;
    gap: 0.16rem;
    flex: 1;
    min-height: 0;
    overflow: hidden;

    // 左侧主内容（可滚动）
    .forecast__main {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 0.14rem;
      min-width: 0;
      overflow-y: auto;

      &::-webkit-scrollbar {
        width: 0.04rem;
      }
      &::-webkit-scrollbar-thumb {
        background: var(--vt-scrollbar-thumb);
        border-radius: 0.04rem;
      }
    }

    // 右侧固定面板
    .forecast__panel {
      width: 3.2rem;
      flex-shrink: 0;
      display: flex;
      flex-direction: column;
      gap: 0.12rem;
      overflow-y: auto;

      &::-webkit-scrollbar {
        width: 0.04rem;
      }
      &::-webkit-scrollbar-thumb {
        background: var(--vt-scrollbar-thumb);
        border-radius: 0.04rem;
      }
    }
  }

  // ── 摘要卡片区 ──
  .forecast__summary {
    background: var(--vt-bg-glass);
    border: 1px solid var(--vt-border-color);
    border-radius: var(--vt-radius-md);
    padding: 0.14rem 0.16rem;
    flex-shrink: 0;

    .section-header {
      display: flex;
      align-items: center;
      margin-bottom: 0.1rem;
      gap: 0.08rem;
    }

    .summary-cards {
      display: flex;
      gap: 0.12rem;

      .summary-card {
        flex: 1;
        display: flex;
        align-items: center;
        gap: 0.12rem;
        background: var(--vt-bg-glass-strong);
        border: 1px solid var(--vt-border-color-soft);
        border-radius: var(--vt-radius-md);
        padding: 0.12rem 0.14rem;

        .summary-card__icon {
          width: 0.4rem;
          height: 0.4rem;
          border-radius: 50%;
          display: flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;
          font-size: 0.2rem;

          &.pv-icon {
            background: rgba(255, 165, 0, 0.15);
            color: #ffa500;
          }
          &.load-icon {
            background: rgba(105, 203, 255, 0.15);
            color: #69cbff;
          }
          &.self-icon {
            background: rgba(82, 196, 26, 0.15);
            color: #52c41a;
          }
        }

        .summary-card__content {
          display: flex;
          flex-direction: column;
          gap: 0.03rem;

          .summary-card__label {
            font-size: var(--vt-font-size-xs);
            color: var(--vt-text-secondary);
          }

          .summary-card__value {
            font-size: var(--vt-font-size-xl);
            font-weight: var(--vt-font-weight-bold);
            color: var(--vt-text-primary);
            line-height: 1.2;

            .summary-card__unit {
              font-size: var(--vt-font-size-xs);
              color: var(--vt-text-secondary);
              margin-left: 0.04rem;
              font-weight: var(--vt-font-weight-normal);
            }
          }

          .summary-card__trend {
            font-size: var(--vt-font-size-xs);
            color: var(--vt-text-secondary);

            span {
              font-weight: var(--vt-font-weight-semibold);
            }

            &.up span {
              color: var(--vt-color-success);
            }
            &.down span {
              color: var(--vt-color-danger);
            }
          }
        }
      }
    }
  }

  // ── 图表区域（PV + Load 共用） ──
  .forecast__chart-section {
    background: var(--vt-bg-glass);
    border: 1px solid var(--vt-border-color);
    border-radius: var(--vt-radius-md);
    padding: 0.14rem 0.16rem;
    display: flex;
    flex-direction: column;

    .section-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.08rem;

      .section-title-row {
        display: flex;
        align-items: baseline;
        gap: 0.08rem;
      }

      .section-title {
        font-size: var(--vt-font-size-md);
        font-weight: var(--vt-font-weight-semibold);
        color: var(--vt-text-primary);
      }

      .section-subtitle {
        font-size: var(--vt-font-size-xs);
        color: var(--vt-text-secondary);
      }

      .chart-legend {
        display: flex;
        align-items: center;
        gap: 0.14rem;

        .legend-item {
          display: flex;
          align-items: center;
          gap: 0.06rem;
          font-size: var(--vt-font-size-xs);
          color: rgba(255, 255, 255, 0.6);

          .legend-line {
            display: inline-block;
            width: 0.22rem;
            height: 0.02rem;

            &.solid.pv {
              background: #69cbff;
            }
            &.solid.load {
              background: #a78bfa;
            }
            &.dashed {
              background: none;
              border-top: 0.02rem dashed #ff6900;
            }
          }

          .legend-band {
            display: inline-block;
            width: 0.18rem;
            height: 0.09rem;
            border-radius: 0.02rem;

            &.pv-band {
              background: rgba(105, 203, 255, 0.2);
              border: 1px dashed rgba(105, 203, 255, 0.4);
            }
            &.load-band {
              background: rgba(167, 139, 250, 0.2);
              border: 1px dashed rgba(167, 139, 250, 0.4);
            }
          }
        }
      }
    }

    .forecast__chart-wrap {
      height: 2.4rem;
      position: relative;
    }
  }

  // ── 右侧面板卡片 ──
  .panel-card {
    background: var(--vt-bg-glass);
    border: 1px solid var(--vt-border-color);
    border-radius: var(--vt-radius-md);
    padding: 0.12rem 0.14rem;

    .panel-card__header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.1rem;

      .panel-card__title {
        display: flex;
        align-items: center;
        gap: 0.06rem;
        font-size: var(--vt-font-size-sm);
        font-weight: var(--vt-font-weight-semibold);
        color: var(--vt-text-primary);

        .ai-badge {
          background: var(--vt-color-primary);
          color: #fff;
          font-size: 0.1rem;
          font-weight: var(--vt-font-weight-bold);
          padding: 0.01rem 0.05rem;
          border-radius: 0.03rem;
          line-height: 1.4;
        }

        .title-sub {
          font-size: var(--vt-font-size-xs);
          color: var(--vt-text-secondary);
          font-weight: var(--vt-font-weight-normal);
        }
      }

      .view-all-btn {
        font-size: var(--vt-font-size-xs);
        color: var(--vt-color-primary);
        padding: 0;
      }
    }

    .panel-card__body {
      display: flex;
      flex-direction: column;
      gap: 0.08rem;
    }
  }

  // ── AI 建议条目 ──
  .suggestion-item {
    display: flex;
    align-items: flex-start;
    gap: 0.08rem;
    padding: 0.1rem;
    border-radius: var(--vt-radius-sm);
    background: var(--vt-bg-glass-strong);
    border-left: 0.03rem solid transparent;

    &.warning {
      border-left-color: var(--vt-color-level-warning);
    }
    &.info {
      border-left-color: var(--vt-color-warning);
    }

    .suggestion-item__icon {
      width: 0.28rem;
      height: 0.28rem;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 0.14rem;
      flex-shrink: 0;
      margin-top: 0.01rem;

      &.warning {
        background: rgba(255, 110, 8, 0.15);
        color: var(--vt-color-level-warning);
      }
      &.info {
        background: rgba(250, 173, 20, 0.15);
        color: var(--vt-color-warning);
      }
    }

    .suggestion-item__content {
      flex: 1;
      min-width: 0;

      .suggestion-item__title {
        font-size: var(--vt-font-size-xs);
        font-weight: var(--vt-font-weight-semibold);
        color: var(--vt-text-primary);
        line-height: 1.4;
        margin-bottom: 0.03rem;
      }

      .suggestion-item__desc {
        font-size: 0.11rem;
        color: var(--vt-text-secondary);
        line-height: 1.4;
      }

      .suggestion-item__impact {
        font-size: 0.11rem;
        color: var(--vt-text-secondary);
        margin-top: 0.03rem;
      }
    }

    .suggestion-item__btn {
      flex-shrink: 0;
      font-size: 0.11rem;
      height: 0.26rem;
      padding: 0 0.08rem;
      color: #fff;

      &.warning {
        background: var(--vt-color-primary);
        border-color: var(--vt-color-primary);
      }
      &.info {
        background: var(--vt-color-warning);
        border-color: var(--vt-color-warning);
      }
    }
  }

  // ── 推荐操作条目 ──
  .action-item {
    display: flex;
    align-items: center;
    gap: 0.08rem;
    padding: 0.1rem;
    border-radius: var(--vt-radius-sm);
    background: var(--vt-bg-glass-strong);
    border: 1px solid var(--vt-border-color-soft);

    .action-item__icon {
      width: 0.32rem;
      height: 0.32rem;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
      font-size: 0.15rem;

      &.battery {
        background: rgba(105, 203, 255, 0.15);
        color: #69cbff;
      }
      &.lightning {
        background: rgba(255, 105, 0, 0.15);
        color: var(--vt-color-primary);
      }
    }

    .action-item__content {
      flex: 1;
      min-width: 0;

      .action-item__title {
        font-size: var(--vt-font-size-xs);
        font-weight: var(--vt-font-weight-semibold);
        color: var(--vt-text-primary);
      }

      .action-item__desc,
      .action-item__meta {
        font-size: 0.11rem;
        color: var(--vt-text-secondary);
        line-height: 1.4;
      }
    }

    .action-item__btn {
      flex-shrink: 0;
      font-size: 0.11rem;
      height: 0.26rem;
      padding: 0 0.1rem;
      background: var(--vt-bg-glass-heavy);
      border: 1px solid var(--vt-border-color);
      color: var(--vt-text-primary);

      &:hover {
        border-color: var(--vt-color-primary);
        color: var(--vt-color-primary);
      }
    }
  }

  // ── 预测准确率 ──
  .accuracy-card {
    .accuracy-metrics {
      display: flex;
      flex-direction: column;
      gap: 0.08rem;
      margin-bottom: 0.1rem;

      .accuracy-item {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.08rem 0.1rem;
        border-radius: var(--vt-radius-sm);
        background: var(--vt-bg-glass-strong);

        &.overall {
          background: rgba(82, 196, 26, 0.08);
          border: 1px solid rgba(82, 196, 26, 0.2);
        }

        .accuracy-item__label {
          font-size: var(--vt-font-size-xs);
          color: var(--vt-text-secondary);
        }

        .accuracy-item__value {
          font-size: var(--vt-font-size-md);
          font-weight: var(--vt-font-weight-bold);

          &.pv {
            color: #ffa500;
          }
          &.load {
            color: #a78bfa;
          }
          &.overall-val {
            color: var(--vt-color-success);
            font-size: var(--vt-font-size-lg);
          }
        }
      }
    }

    .view-details-btn {
      font-size: var(--vt-font-size-xs);
      color: var(--vt-color-primary);
      padding: 0;
    }
  }

  // 通用 section header 变量
  .section-title {
    font-size: var(--vt-font-size-md);
    font-weight: var(--vt-font-weight-semibold);
    color: var(--vt-text-primary);
  }

  .section-subtitle {
    font-size: var(--vt-font-size-xs);
    color: var(--vt-text-secondary);
  }
}
</style>
