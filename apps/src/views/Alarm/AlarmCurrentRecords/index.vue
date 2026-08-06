<template>
  <div class="alarm-records vt-page-shell">
    <LoadingBg :loading="loading">
      <!-- toolbar -->
      <div class="alarm-records__toolbar vt-toolbar">
        <div class="alarm-records__toolbar-left vt-toolbar__left" ref="toolbarLeftRef">
          <el-form :model="filters" :inline="true" class="test-form alarm-records__toolbar-form vt-toolbar-form">
            <el-form-item label="Alarm Level:">
              <el-select v-model="filters.warning_level" clearable placeholder="Please select level"
                :append-to="toolbarLeftRef" style="width: 2.4rem">
                <el-option label="Critical Alarm" :value="1" />
                <el-option label="Warning Alarm" :value="2" />
                <el-option label="Info Alarm" :value="3" />
              </el-select>
            </el-form-item>
          </el-form>
        </div>

        <div class="alarm-records__toolbar-right vt-toolbar__right">
          <IconButton type="warning" :icon="reloadIcon" text="Reload" custom-class="alarm-records__btn"
            @click="reloadFilters" />
          <IconButton type="primary" :icon="searchIcon" text="Search" custom-class="alarm-records__btn"
            @click="fetchTableData(true)" />
        </div>
      </div>

      <!-- table -->
      <div class="alarm-records__table vt-table-shell">
        <el-table :data="tableData" class="alarm-records__table-content vt-table-content">
          <el-table-column prop="rule_name" label="Rule Name" :min-width="160" show-overflow-tooltip />
          <el-table-column prop="warning_level" label="Alarm Level" :width="160">
            <template #default="{ row }">
              <span class="alarm-records__table-level-text" :class="`alarm-level--${row.warning_level}`">
                {{ levelTextList[row.warning_level as 1 | 2 | 3] || '-' }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="Device Name" :min-width="160" show-overflow-tooltip>
            <template #default="{ row }">{{ row.device_name || '-' }}</template>
          </el-table-column>
          <el-table-column label="Point Name" :min-width="140" show-overflow-tooltip>
            <template #default="{ row }">{{ row.point_name || '-' }}</template>
          </el-table-column>
          <el-table-column label="Trigger Value" :min-width="140">
            <template #default="{ row }">{{ formatValue(row.current_value, row.unit) }}</template>
          </el-table-column>
          <el-table-column label="Condition" :min-width="140">
            <template #default="{ row }">{{ formatCondition(row) }}</template>
          </el-table-column>
          <el-table-column label="Triggered Time" :min-width="180">
            <template #default="{ row }">
              <span>{{ formatDateTime(row.triggered_at) }}</span>
            </template>
          </el-table-column>
        </el-table>

        <!-- pagination -->
        <div id="alarm-current-pagination-anchor" class="alarm-records__pagination vt-pagination">
          <el-pagination v-model:current-page="pagination.page" v-model:page-size="pagination.pageSize"
            :page-sizes="[10, 20, 50, 100]" :total="pagination.total" layout="total, sizes, prev, pager, next"
            :teleported="false" append-size-to="#alarm-current-pagination-anchor" @size-change="handlePageSizeChange"
            @current-change="handlePageChange" />
        </div>
      </div>
    </LoadingBg>
  </div>
</template>

<script setup lang="ts">
import type { CurrentAlarmData } from '@/types/alarm'
import { useTableData, type TableConfig } from '@/composables/useTableData'

import reloadIcon from '@/assets/icons/table-refresh.svg'
import searchIcon from '@/assets/icons/table-search.svg'
const levelTextList = {
  1: 'Critical Alarm',
  2: 'Warning Alarm',
  3: 'Info Alarm',
}
const toolbarLeftRef = ref<HTMLElement | null>(null)
// table config
const tableConfig: TableConfig = {
  listUrl: '/alarmApi/alerts',
  defaultPageSize: 20,
}

// format trigger value with unit
const formatValue = (value: number | string | null | undefined, unit?: string | null): string => {
  if (value === null || value === undefined || value === '') return '-'
  const v = typeof value === 'number' ? value : Number(value)
  const num = Number.isFinite(v) ? v : value
  return unit ? `${num} ${unit}` : String(num)
}

// format condition with unit
const formatCondition = (row: CurrentAlarmData): string => {
  const op = row.operator
  const threshold = row.threshold_value
  if (!op || threshold === null || threshold === undefined) return '-'
  const t = typeof threshold === 'number' ? threshold : Number(threshold)
  const val = Number.isFinite(t) ? t : threshold
  return row.unit ? `${op} ${val} ${row.unit}` : `${op} ${val}`
}

// use useTableData composable
const {
  loading,
  tableData,
  pagination,
  handlePageSizeChange,
  fetchTableData,
  filters,
  reloadFilters,
  handlePageChange,
} = useTableData<CurrentAlarmData>(tableConfig)

filters.warning_level = null

// format date time
const formatDateTime = (dateTime: number | string | null | undefined): string => {
  if (dateTime === null || dateTime === undefined || dateTime === '') return '-'
  try {
    // Unix timestamp to date time
    const date = typeof dateTime === 'number' ? new Date(dateTime * 1000) : new Date(dateTime)
    if (isNaN(date.getTime())) return String(dateTime)
    const year = date.getFullYear()
    const month = String(date.getMonth() + 1).padStart(2, '0')
    const day = String(date.getDate()).padStart(2, '0')
    const hours = String(date.getHours()).padStart(2, '0')
    const minutes = String(date.getMinutes()).padStart(2, '0')
    const seconds = String(date.getSeconds()).padStart(2, '0')
    return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`
  } catch {
    return String(dateTime)
  }
}
</script>

<style scoped lang="scss">
.alarm-records {
  .alarm-records__toolbar {
    .alarm-records__toolbar-left {
      position: relative;
    }

    .alarm-records__toolbar-right {
      display: flex;
      align-items: center;
      gap: 0.16rem;

      .alarm-records__btn {
        display: flex;
        align-items: center;
        gap: 0.1rem;
      }
    }
  }

  .alarm-records__table {
    .alarm-records__table-content {
      .alarm-records__table-level-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
    }
  }

  :deep(.alarm-records__toolbar-form.el-form--inline .el-form-item) {
    margin-bottom: 0;
    margin-right: 0.2rem;
  }

  .alarm-level--1 {
    color: var(--vt-color-level-critical);
  }

  .alarm-level--2 {
    color: var(--vt-color-level-warning);
  }

  .alarm-level--3 {
    color: var(--vt-color-level-info);
  }
}
</style>
