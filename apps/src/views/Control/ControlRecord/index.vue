<template>
  <div class="control-history vt-page-shell">
    <LoadingBg :loading="loading">
      <div class="control-history__toolbar vt-toolbar">
        <div class="control-history__toolbar-left vt-toolbar__left">
          <el-form :model="filters" :inline="true" class="control-history__toolbar-form vt-toolbar-form">
            <el-form-item label="Rule Name:">
              <el-input v-model="filters.rule_name" placeholder="Please enter rule name" clearable
                class="control-history__search-input" @keyup.enter="fetchTableData(true)" />
            </el-form-item>
            <el-form-item label="Start Time:">
              <el-date-picker v-model="startTimeDisplay" type="datetime" placeholder="Please select start time"
                format="YYYY-MM-DD HH:mm:ss" :disabled-date="disableStartDate" :disabled-time="disableStartTime"
                :teleported="false" clearable @change="handleStartTimeChange" />
            </el-form-item>
            <el-form-item label="End Time:">
              <el-date-picker v-model="endTimeDisplay" type="datetime" placeholder="Please select end time"
                format="YYYY-MM-DD HH:mm:ss" :disabled-date="disableEndDate" :disabled-time="disableEndTime"
                :teleported="false" clearable @change="handleEndTimeChange" />
            </el-form-item>
          </el-form>
        </div>

        <div class="control-history__toolbar-right vt-toolbar__right">
          <IconButton type="warning" :icon="reloadIcon" text="Reload" custom-class="control-history__btn"
            @click="reloadHistoryFilters" />
          <IconButton type="primary" :icon="searchIcon" text="Search" custom-class="control-history__btn"
            @click="fetchTableData(true)" />
        </div>
      </div>

      <div class="control-history__table vt-table-shell">
        <el-table :data="tableData" class="control-history__table-content vt-table-content" table-layout="fixed"
          align="left">
          <el-table-column prop="rule_name" label="Rule Name" min-width="160" show-overflow-tooltip />

          <el-table-column label="Trigger Reason" min-width="260" show-overflow-tooltip>
            <template #default="{ row }">
              {{ getTriggerReason(row) }}
            </template>
          </el-table-column>

          <el-table-column prop="triggered_at" label="Trigger Time" width="200">
            <template #default="{ row }">
              {{ formatDateTime(row.triggered_at) }}
            </template>
          </el-table-column>

          <el-table-column label="Result" width="100">
            <template #default="{ row }">
              <span
                class="control-history__status"
                :class="isSuccessRow(row) ? 'control-history__status--success' : 'control-history__status--failed'"
              >
                {{ isSuccessRow(row) ? 'Success' : 'Failed' }}
              </span>
            </template>
          </el-table-column>

          <el-table-column label="Summary" min-width="220" show-overflow-tooltip>
            <template #default="{ row }">
              {{ getHistorySummary(row) }}
            </template>
          </el-table-column>
        </el-table>

        <div id="control-history-pagination-anchor" class="control-history__pagination vt-pagination">
          <el-pagination v-model:current-page="pagination.page" v-model:page-size="pagination.pageSize"
            :page-sizes="[10, 20, 50, 100]" :total="pagination.total" layout="total, sizes, prev, pager, next"
            :teleported="false" append-size-to="#control-history-pagination-anchor" @size-change="handlePageSizeChange"
            @current-change="handlePageChange" />
        </div>
      </div>
    </LoadingBg>
  </div>
</template>

<script setup lang="ts">
import LoadingBg from '@/components/common/LoadingBg.vue'
import reloadIcon from '@/assets/icons/table-refresh.svg'
import searchIcon from '@/assets/icons/table-search.svg'
import { useTableData, type TableConfig } from '@/composables/useTableData'
import type { RuleHistoryRecord } from '@/types/controlRule'
import {
  getHistorySummary,
  getTriggerReason,
  isHistorySuccess,
} from '@/utils/ruleHistoryDisplay'
import { formatDateTime } from '@/utils/date'

const tableConfig: TableConfig = {
  listUrl: '/ruleApi/api/rules/history',
  defaultPageSize: 20,
}

const {
  loading,
  tableData,
  pagination,
  handlePageSizeChange,
  fetchTableData,
  filters,
  handlePageChange,
  reloadFilters,
} = useTableData<RuleHistoryRecord>(tableConfig)

filters.rule_name = ''
filters.start_time = null
filters.end_time = null
filters.startTime = null
filters.endTime = null

const startTimeDisplay = ref<Date | null>(null)
const endTimeDisplay = ref<Date | null>(null)

const reloadHistoryFilters = () => {
  startTimeDisplay.value = null
  endTimeDisplay.value = null
  filters.start_time = null
  filters.end_time = null
  filters.startTime = null
  filters.endTime = null
  reloadFilters()
}

const handleStartTimeChange = (value: Date | null) => {
  startTimeDisplay.value = value
  filters.startTime = value || null

  if (value && endTimeDisplay.value && value.getTime() >= endTimeDisplay.value.getTime()) {
    endTimeDisplay.value = null
    filters.endTime = null
    filters.end_time = null
  }

  filters.start_time = value ? value.getTime() : null
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
  filters.endTime = adjusted || null

  if (adjusted && startTimeDisplay.value && adjusted.getTime() <= startTimeDisplay.value.getTime()) {
    startTimeDisplay.value = null
    filters.startTime = null
    filters.start_time = null
  }

  filters.end_time = adjusted ? adjusted.getTime() : null
}

const disableStartDate = (time: Date) => {
  if (!filters.endTime) return false
  return time.getTime() > filters.endTime.getTime()
}

const disableStartTime = (date: Date, type: string) => {
  if (!filters.endTime || type !== 'minute') return {}
  const endTime = filters.endTime
  if (date.getDate() === endTime.getDate()) {
    return {
      disabledHours: () => Array.from({ length: 24 }, (_, i) => i).filter((h) => h > endTime.getHours()),
      disabledMinutes: () =>
        Array.from({ length: 60 }, (_, i) => i).filter((m) => m > endTime.getMinutes()),
    }
  }
  return {}
}

const disableEndDate = (time: Date) => {
  if (!filters.startTime) return false
  return time.getTime() < filters.startTime.getTime()
}

const disableEndTime = (date: Date, type: string) => {
  if (!filters.startTime || type !== 'minute') return {}
  const startTime = filters.startTime
  if (date.getDate() === startTime.getDate()) {
    return {
      disabledHours: () => Array.from({ length: 24 }, (_, i) => i).filter((h) => h < startTime.getHours()),
      disabledMinutes: () =>
        Array.from({ length: 60 }, (_, i) => i).filter((m) => m < startTime.getMinutes()),
    }
  }
  return {}
}

const isSuccessRow = (row: RuleHistoryRecord) => isHistorySuccess(row)

</script>

<style scoped lang="scss">
.control-history {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;

  .control-history__toolbar {
    .control-history__toolbar-left {
      position: relative;
      display: flex;
      align-items: center;
      gap: 0.16rem;
    }

    .control-history__toolbar-right {
      display: flex;
      align-items: center;
      gap: 0.1rem;
    }
  }

  :deep(.control-history__toolbar-form.el-form--inline .el-form-item) {
    margin-bottom: 0;
  }

  .control-history__table {
    height: calc(100% - 0.52rem);
    width: 100%;
    display: flex;
    flex-direction: column;

    .control-history__table-content {
      width: 100%;
      height: calc(100% - 0.92rem);
      overflow-y: auto;
    }

    .control-history__pagination {
      position: relative;
      padding: 0.2rem 0;
      display: flex;
      justify-content: flex-end;
    }
  }

  .control-history__status {
    display: inline-flex;
    align-items: center;
    padding: 0 0.08rem;
    border-radius: 0.04rem;
    font-size: 0.12rem;
    line-height: 0.22rem;
  }

  .control-history__status--success {
    color: #67c23a;
    background: rgba(103, 194, 58, 0.12);
  }

  .control-history__status--failed {
    color: #f56c6c;
    background: rgba(245, 108, 108, 0.12);
  }
}
</style>
