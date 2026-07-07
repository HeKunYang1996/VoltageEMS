<template>
  <FormDialog width="13.46rem" ref="dialogRef" :title="dialogTitle">
    <template #dialog-body>
      <div class="rule-history-dialog">
        <div class="rule-history-dialog__toolbar">
          <el-form :inline="true" class="rule-history-dialog__toolbar-form">
            <el-form-item label="Start Time:">
              <el-date-picker
                v-model="startTimeDisplay"
                type="datetime"
                placeholder="Please select start time"
                format="YYYY-MM-DD HH:mm:ss"
                :disabled-date="disableStartDate"
                :disabled-time="disableStartTime"
                :teleported="false"
                clearable
                @change="handleStartTimeChange"
              />
            </el-form-item>
            <el-form-item label="End Time:">
              <el-date-picker
                v-model="endTimeDisplay"
                type="datetime"
                placeholder="Please select end time"
                format="YYYY-MM-DD HH:mm:ss"
                :disabled-date="disableEndDate"
                :disabled-time="disableEndTime"
                :teleported="false"
                clearable
                @change="handleEndTimeChange"
              />
            </el-form-item>
          </el-form>
          <div class="rule-history-dialog__toolbar-actions">
            <IconButton
              type="warning"
              :icon="reloadIcon"
              text="Reload"
              custom-class="rule-history-dialog__btn"
              @click="reloadHistoryFilters"
            />
            <IconButton
              type="primary"
              :icon="searchIcon"
              text="Search"
              custom-class="rule-history-dialog__btn"
              @click="searchHistory"
            />
          </div>
        </div>
        <div class="rule-history-dialog__body">
        <LoadingBg :loading="loading" class="rule-history-dialog__loading">
          <div class="rule-history-dialog__table-wrap">
            <el-table
              :data="historyList"
              class="rule-history-dialog__table"
              height="5rem"
              table-layout="fixed"
              align="left"
            >
              <el-table-column type="expand" width="48">
                <template #default="{ row }">
                  <div class="rule-history-dialog__detail">
                    <div v-if="row.error" class="rule-history-dialog__error-banner">
                      {{ row.error }}
                    </div>

                    <div class="rule-history-dialog__detail-grid">
                      <div class="rule-history-dialog__detail-item rule-history-dialog__detail-item--full">
                        <span class="rule-history-dialog__detail-label">Trigger Reason:</span>
                        <span class="rule-history-dialog__detail-value">
                          {{ getTriggerReason(row) }}
                        </span>
                      </div>

                      <div class="rule-history-dialog__detail-item rule-history-dialog__detail-item--full">
                        <span class="rule-history-dialog__detail-label">Variables:</span>
                        <div class="rule-history-dialog__tag-list">
                          <template v-if="getDisplayVariables(row).length">
                            <span
                              v-for="item in getDisplayVariables(row)"
                              :key="item.key"
                              class="rule-history-dialog__tag"
                            >
                              {{ formatDisplayVariable(item) }}
                            </span>
                          </template>
                          <span v-else class="rule-history-dialog__detail-value">-</span>
                        </div>
                      </div>

                      <div class="rule-history-dialog__detail-item rule-history-dialog__detail-item--full">
                        <span class="rule-history-dialog__detail-label">Execution Steps:</span>
                        <div class="rule-history-dialog__path">
                          <template v-if="getExecutionSteps(row).length">
                            <template
                              v-for="(step, index) in getExecutionSteps(row)"
                              :key="`${step.node_id}-${index}`"
                            >
                              <span class="rule-history-dialog__path-node">
                                {{ formatExecutionStep(step) }}
                              </span>
                              <span
                                v-if="index < getExecutionSteps(row).length - 1"
                                class="rule-history-dialog__path-arrow"
                              >
                                →
                              </span>
                            </template>
                          </template>
                          <span v-else class="rule-history-dialog__detail-value">-</span>
                        </div>
                      </div>
                    </div>

                    <div
                      v-if="getDisplayActions(row).length"
                      class="rule-history-dialog__actions"
                    >
                      <div class="rule-history-dialog__detail-label">Actions Executed:</div>
                      <div class="rule-history-dialog__actions-list">
                        <div
                          v-for="(action, index) in getDisplayActions(row)"
                          :key="`action-${index}`"
                          class="rule-history-dialog__action-card"
                        >
                          <span class="rule-history-dialog__action-description">
                            {{ action.description }}
                          </span>
                          <span
                            class="rule-history-dialog__action-status"
                            :class="
                              action.success
                                ? 'rule-history-dialog__action-status--success'
                                : 'rule-history-dialog__action-status--failed'
                            "
                          >
                            {{ action.success === false ? 'Failed' : action.success ? 'Success' : '-' }}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                </template>
              </el-table-column>

              <el-table-column prop="triggered_at" label="Trigger Time" width="200">
                <template #default="{ row }">
                  <span class="rule-history-dialog__cell-text">{{ formatDateTime(row.triggered_at) }}</span>
                </template>
              </el-table-column>

              <el-table-column label="Result" width="100">
                <template #default="{ row }">
                  <span
                    class="rule-history-dialog__status"
                    :class="isSuccessRow(row) ? 'rule-history-dialog__status--success' : 'rule-history-dialog__status--failed'"
                  >
                    {{ isSuccessRow(row) ? 'Success' : 'Failed' }}
                  </span>
                </template>
              </el-table-column>

              <el-table-column label="Summary" min-width="200">
                <template #default="{ row }">
                  <span class="rule-history-dialog__cell-text">{{ getHistorySummary(row) }}</span>
                </template>
              </el-table-column>
            </el-table>
          </div>
        </LoadingBg>

        <div id="rule-history-pagination-anchor" class="rule-history-dialog__pagination vt-pagination">
          <el-pagination
            v-model:current-page="pagination.page"
            v-model:page-size="pagination.pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="pagination.total"
            layout="total, sizes, prev, pager, next"
            :teleported="false"
            append-size-to="#rule-history-pagination-anchor"
            @size-change="handlePageSizeChange"
            @current-change="handlePageChange"
          />
        </div>
        </div>
      </div>
    </template>

    <template #dialog-footer>
      <el-button type="warning" @click="close">Close</el-button>
    </template>
  </FormDialog>
</template>

<script setup lang="ts">
import FormDialog from '@/components/dialog/FormDialog.vue'
import LoadingBg from '@/components/common/LoadingBg.vue'
import reloadIcon from '@/assets/icons/table-refresh.svg'
import searchIcon from '@/assets/icons/table-search.svg'
import { getModRuleHistory } from '@/api/rulesManagement'
import type { ModRuleSummary, RuleHistoryItem } from '@/types/controlRule'
import {
  formatDisplayVariable,
  formatExecutionStep,
  getDisplayActions,
  getDisplayVariables,
  getExecutionSteps,
  getHistorySummary,
  getTriggerReason,
} from '@/utils/ruleHistoryDisplay'

const dialogRef = ref<InstanceType<typeof FormDialog> | null>(null)
const loading = ref(false)
const currentRule = ref<ModRuleSummary | null>(null)
const historyList = ref<RuleHistoryItem[]>([])

const pagination = reactive({
  page: 1,
  pageSize: 20,
  total: 0,
})

const historyFilters = reactive({
  start_time: null as number | null,
  end_time: null as number | null,
  startTime: null as Date | null,
  endTime: null as Date | null,
})

const startTimeDisplay = ref<Date | null>(null)
const endTimeDisplay = ref<Date | null>(null)

const resetHistoryFilters = () => {
  historyFilters.start_time = null
  historyFilters.end_time = null
  historyFilters.startTime = null
  historyFilters.endTime = null
  startTimeDisplay.value = null
  endTimeDisplay.value = null
}

const handleStartTimeChange = (value: Date | null) => {
  startTimeDisplay.value = value
  historyFilters.startTime = value || null
  if (value && historyFilters.endTime && value.getTime() >= historyFilters.endTime.getTime()) {
    historyFilters.endTime = null
    historyFilters.end_time = null
    endTimeDisplay.value = null
  }
  historyFilters.start_time = value ? value.getTime() : null
}

const handleEndTimeChange = (value: Date | null) => {
  const adjusted: Date | null = value ? new Date(value) : null
  if (
    adjusted &&
    adjusted.getHours() === 0 &&
    adjusted.getMinutes() === 0 &&
    adjusted.getSeconds() === 0
  ) {
    adjusted.setHours(23, 59, 59, 999)
  }
  endTimeDisplay.value = adjusted
  historyFilters.endTime = adjusted || null
  if (
    adjusted &&
    historyFilters.startTime &&
    adjusted.getTime() <= historyFilters.startTime.getTime()
  ) {
    historyFilters.startTime = null
    historyFilters.start_time = null
    startTimeDisplay.value = null
  }
  historyFilters.end_time = adjusted ? adjusted.getTime() : null
}

const disableStartDate = (time: Date) => {
  if (!historyFilters.endTime) return false
  return time.getTime() > historyFilters.endTime.getTime()
}

const disableStartTime = (date: Date, type: string) => {
  if (!historyFilters.endTime || type !== 'minute') return {}
  const endTime = historyFilters.endTime
  if (date.getDate() === endTime.getDate()) {
    return {
      disabledHours: () =>
        Array.from({ length: 24 }, (_, i) => i).filter((h) => h > endTime.getHours()),
      disabledMinutes: () =>
        Array.from({ length: 60 }, (_, i) => i).filter((m) => m > endTime.getMinutes()),
    }
  }
  return {}
}

const disableEndDate = (time: Date) => {
  if (!historyFilters.startTime) return false
  return time.getTime() < historyFilters.startTime.getTime()
}

const disableEndTime = (date: Date, type: string) => {
  if (!historyFilters.startTime || type !== 'minute') return {}
  const startTime = historyFilters.startTime
  if (date.getDate() === startTime.getDate()) {
    return {
      disabledHours: () =>
        Array.from({ length: 24 }, (_, i) => i).filter((h) => h < startTime.getHours()),
      disabledMinutes: () =>
        Array.from({ length: 60 }, (_, i) => i).filter((m) => m < startTime.getMinutes()),
    }
  }
  return {}
}

const dialogTitle = computed(() => {
  const name = currentRule.value?.name || ''
  return name ? `Trigger History: ${name}` : 'Trigger History'
})

const formatDateTime = (dateTime: string | number | null | undefined): string => {
  if (dateTime === null || dateTime === undefined || dateTime === '') return '-'
  try {
    const date = typeof dateTime === 'number' ? new Date(dateTime * 1000) : new Date(dateTime)
    if (isNaN(date.getTime())) return String(dateTime)
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
  } catch {
    return String(dateTime)
  }
}

const isSuccessRow = (row: RuleHistoryItem) => {
  if (row.error) return false
  return row.result?.success === true
}

const fetchHistory = async () => {
  if (!currentRule.value) return
  loading.value = true
  try {
    const res = await getModRuleHistory(currentRule.value.id, {
      page: pagination.page,
      page_size: pagination.pageSize,
      ...(historyFilters.start_time != null ? { start_time: historyFilters.start_time } : {}),
      ...(historyFilters.end_time != null ? { end_time: historyFilters.end_time } : {}),
    })
    if (res.success && res.data) {
      historyList.value = res.data.list || []
      pagination.total = res.data.total || 0
    } else {
      historyList.value = []
      pagination.total = 0
    }
  } catch {
    historyList.value = []
    pagination.total = 0
  } finally {
    loading.value = false
  }
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.pageSize = pageSize
  pagination.page = 1
  void fetchHistory()
}

const handlePageChange = (page: number) => {
  pagination.page = page
  void fetchHistory()
}

const searchHistory = () => {
  pagination.page = 1
  void fetchHistory()
}

const reloadHistoryFilters = () => {
  resetHistoryFilters()
  pagination.page = 1
  void fetchHistory()
}

const open = async (rule: ModRuleSummary) => {
  currentRule.value = rule
  pagination.page = 1
  pagination.pageSize = 20
  pagination.total = 0
  historyList.value = []
  resetHistoryFilters()
  dialogRef.value!.dialogVisible = true
  await fetchHistory()
}

const close = () => {
  if (dialogRef.value) {
    dialogRef.value.dialogVisible = false
  }
}

defineExpose({ open, close })
</script>

<style scoped lang="scss">
.rule-history-dialog {
  .rule-history-dialog__toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.16rem;
    margin-bottom: 0.16rem;
  }

  .rule-history-dialog__toolbar-form {
    flex: 1;
    min-width: 0;
  }

  :deep(.rule-history-dialog__toolbar-form.el-form--inline .el-form-item) {
    margin-bottom: 0;
  }

  .rule-history-dialog__toolbar-actions {
    display: flex;
    align-items: center;
    gap: 0.1rem;
    flex-shrink: 0;
  }

  .rule-history-dialog__body {
    display: flex;
    flex-direction: column;
  }

  .rule-history-dialog__table {
    width: 100%;
  }

  .rule-history-dialog__status {
    display: inline-flex;
    align-items: center;
    padding: 0 0.08rem;
    border-radius: 0.04rem;
    font-size: 0.12rem;
    line-height: 0.22rem;
  }

  .rule-history-dialog__status--success {
    color: #67c23a;
    background: rgba(103, 194, 58, 0.12);
  }

  .rule-history-dialog__status--failed {
    color: #f56c6c;
    background: rgba(245, 108, 108, 0.12);
  }

  .rule-history-dialog__pagination {
    position: relative;
    padding-top: 0.16rem;
  }

  .rule-history-dialog__detail {
    padding: 0.12rem 0.16rem 0.16rem 0.48rem;
    overflow-x: auto;
  }

  .rule-history-dialog__error-banner {
    margin-bottom: 0.12rem;
    padding: 0.08rem 0.12rem;
    border-radius: 0.04rem;
    color: #ffb4b4;
    background: rgba(245, 108, 108, 0.12);
    border: 0.01rem solid rgba(245, 108, 108, 0.24);
    line-height: 1.5;
    word-break: break-word;
  }

  .rule-history-dialog__detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.12rem 0.24rem;
  }

  .rule-history-dialog__detail-item {
    display: flex;
    flex-direction: column;
    gap: 0.06rem;
    min-width: 0;
  }

  .rule-history-dialog__detail-item--full {
    grid-column: 1 / -1;
  }

  .rule-history-dialog__detail-label {
    color: rgba(255, 255, 255, 0.55);
    font-size: 0.12rem;
    line-height: 1.4;
  }

  .rule-history-dialog__detail-value {
    color: #fff;
    font-size: 0.14rem;
    line-height: 1.5;
    word-break: break-word;
  }

  .rule-history-dialog__tag-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.08rem;
  }

  .rule-history-dialog__tag {
    display: inline-flex;
    align-items: center;
    padding: 0.02rem 0.08rem;
    border-radius: 0.04rem;
    background: rgba(84, 98, 140, 0.35);
    color: #fff;
    font-size: 0.12rem;
    line-height: 0.2rem;
  }

  .rule-history-dialog__path {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.06rem;
  }

  .rule-history-dialog__path-node {
    display: inline-flex;
    align-items: center;
    padding: 0.02rem 0.08rem;
    border-radius: 0.04rem;
    background: rgba(255, 105, 0, 0.14);
    color: #fff;
    font-size: 0.12rem;
    line-height: 0.2rem;
  }

  .rule-history-dialog__path-arrow {
    color: rgba(255, 255, 255, 0.45);
    font-size: 0.12rem;
  }

  .rule-history-dialog__actions {
    margin-top: 0.12rem;
  }

  .rule-history-dialog__actions-list {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    margin-top: 0.08rem;
  }

  .rule-history-dialog__action-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.12rem;
    padding: 0.08rem 0.12rem;
    border-radius: 0.04rem;
    background: rgba(84, 98, 140, 0.2);
    color: #fff;
    font-size: 0.13rem;
    line-height: 1.4;
    min-width: max-content;
  }

  .rule-history-dialog__action-description {
    flex: 1;
    min-width: 0;
    word-break: break-word;
  }

  .rule-history-dialog__action-status {
    font-size: 0.12rem;
    white-space: nowrap;
  }

  .rule-history-dialog__action-status--success {
    color: #67c23a;
  }

  .rule-history-dialog__action-status--failed {
    color: #f56c6c;
  }

  :deep(.rule-history-dialog__table) {
    .el-table__expanded-cell {
      background: rgba(15, 23, 42, 0.35);
    }
  }
}
</style>
