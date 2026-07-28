<template>
  <div class="alarm-records vt-page-shell">
    <LoadingBg :loading="loading">
      <!-- 琛ㄦ牸宸ュ叿锟?-->
      <div class="alarm-records__toolbar vt-toolbar">
        <div class="alarm-records__toolbar-left vt-toolbar__left" ref="toolbarLeftRef">
          <el-form :model="filters" inline class="alarm-records__toolbar-form vt-toolbar-form">
            <el-form-item label="Alarm Level:">
              <el-select
                v-model="filters.warning_level"
                :append-to="toolbarLeftRef"
                clearable
                placeholder="Please select alarm level"
              >
                <el-option label="Critical Alarm" :value="1" />
                <el-option label="Warning Alarm" :value="2" />
                <el-option label="Info Alarm" :value="3" />
              </el-select>
            </el-form-item>
            <el-form-item label="Start Time:">
              <el-date-picker
                v-model="startTimeDisplay"
                type="datetime"
                placeholder="Please select start time"
                format="YYYY-MM-DD HH:mm:ss"
                :disabled-date="disableStartDate"
                :disabled-time="disableStartTime"
                @change="handleStartTimeChange"
                :teleported="false"
                clearable
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
                @change="handleEndTimeChange"
                :teleported="false"
                clearable
              />
            </el-form-item>
          </el-form>
        </div>

        <div class="alarm-records__toolbar-right vt-toolbar__right">
          <IconButton
            type="warning"
            :icon="reloadIcon"
            text="Reload"
            custom-class="alarm-records__export-btn"
            @click="reloadFilters"
          />
          <IconButton
            type="primary"
            :icon="searchIcon"
            text="Search"
            custom-class="alarm-records__export-btn"
            @click="fetchTableData(true)"
          />
          <IconButton
            type="primary"
            :icon="alarmExportIcon"
            text="Export"
            custom-class="alarm-records__export-btn"
            @click="exportData(`Alarm_History_${Date.now().toString()}.csv`)"
          />
        </div>
      </div>

      <!-- 琛ㄦ牸 -->
      <div class="alarm-records__table vt-table-shell">
        <el-table :data="tableData" class="alarm-records__table-content vt-table-content">
          <el-table-column
            prop="rule_name"
            label="Name"
            min-width="1.2rem"
            class-name="table-ellipsis"
          />
          <el-table-column
            prop="channel_id"
            label="Channel ID"
            min-width="1.2rem"
            class-name="table-ellipsis"
          />
          <el-table-column prop="warning_level" label="Level" min-width="1rem">
            <template #default="scope">
              <span
                class="alarm-records__table-level-text"
                :class="`alarm-level--${scope.row.warning_level}`"
              >
                {{ warningLevelText[scope.row.warning_level as 1 | 2 | 3] || '-' }}
              </span>
            </template>
          </el-table-column>
          <el-table-column
            prop="triggered_at"
            label="Start Time"
            min-width="1.6rem"
            class-name="table-ellipsis"
          >
            <template #default="{ row }">
              <span class="table-ellipsis__text">{{ formatDateTime(row.triggered_at) }}</span>
            </template>
          </el-table-column>
          <el-table-column
            prop="recovered_at"
            label="End Time"
            min-width="1.6rem"
            class-name="table-ellipsis"
          >
            <template #default="{ row }">
              <span class="table-ellipsis__text">{{ formatDateTime(row.recovered_at) }}</span>
            </template>
          </el-table-column>
        </el-table>

        <!-- 鍒嗛〉缁勪欢 -->
        <div id="alarm-history-pagination-anchor" class="alarm-records__pagination vt-pagination">
          <el-pagination
            v-model:current-page="pagination.page"
            v-model:page-size="pagination.pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="pagination.total"
            layout="total, sizes, prev, pager, next"
            :teleported="false"
            append-size-to="#alarm-history-pagination-anchor"
            @size-change="handlePageSizeChange"
            @current-change="handlePageChange"
          />
        </div>
      </div>
    </LoadingBg>
  </div>
</template>

<script setup lang="ts">
import type { HistoryAlarmData } from '@/types/alarm'
import { useTableData } from '@/composables/useTableData'

import alarmExportIcon from '@/assets/icons/alarm-export.svg'
import searchIcon from '@/assets/icons/table-search.svg'
import reloadIcon from '@/assets/icons/table-refresh.svg'
const toolbarLeftRef = ref<HTMLElement | null>(null)
const warningLevelText = {
  1: 'Critical Alarm',
  2: 'Warning Alarm',
  3: 'Info Alarm',
}

// 鏃ユ湡閫夋嫨鍣ㄦ樉绀虹敤鐨?Date 瀵硅薄锛堜笌 filters 涓殑 Unix 鏃堕棿鎴冲垎寮€锛?
const startTimeDisplay = ref<Date | null>(null)
const endTimeDisplay = ref<Date | null>(null)

// 浣跨敤 useTableData composable
const {
  loading,
  tableData,
  pagination,
  handlePageSizeChange,
  handlePageChange,
  fetchTableData,
  filters,
  exportData,
  reloadFilters: _reloadFilters,
} = useTableData<HistoryAlarmData>({
  listUrl: '/alarmApi/alert-events',
  exportUrl: '/alarmApi/alert-events/export',
  enableExport: true,
  defaultPageSize: 20,
})

// 閲嶇疆鏃跺悓姝ユ竻绌烘棩鏈熼€夋嫨鍣ㄦ樉绀哄€?
const reloadFilters = () => {
  startTimeDisplay.value = null
  endTimeDisplay.value = null
  _reloadFilters()
}

// 鍒濆鍖杅ilters
filters.warning_level = null
filters.start_time = null
filters.end_time = null

// 澶勭悊寮€濮嬫椂闂村彉鍖?
const handleStartTimeChange = (value: Date | null) => {
  startTimeDisplay.value = value
  // 璁板綍鍘熷 Date 浠ヤ究绂佺敤瑙勫垯璁＄畻
  filters.startTime = value || null
  // 濡傛灉寮€濮嬫椂闂存櫄浜庢垨绛変簬缁撴潫鏃堕棿锛屾竻绌虹粨鏉熸椂闂?
  if (value && filters.endTime && value.getTime() >= new Date(filters.endTime).getTime()) {
    filters.endTime = null
    filters.end_time = null
    endTimeDisplay.value = null
  }
  // 杞彉涓哄悗绔渶瑕佺殑 Unix 绉掓椂闂存埑
  filters.start_time = value ? Math.floor(value.getTime() / 1000) : null
}

// 澶勭悊缁撴潫鏃堕棿鍙樺寲
const handleEndTimeChange = (value: Date | null) => {
  // 璁板綍鍘熷 Date 浠ヤ究绂佺敤瑙勫垯璁＄畻
  const adjusted: Date | null = value ? new Date(value) : null
  // 鑻ユ椂闂存湭鎸囧畾锛?0:00:00锛夛紝榛樿璁剧疆鍒板綋澶?23:59:59
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
  // 濡傛灉缁撴潫鏃堕棿鏃╀簬鎴栫瓑浜庡紑濮嬫椂闂达紝娓呯┖寮€濮嬫椂闂?
  if (
    adjusted &&
    filters.startTime &&
    adjusted.getTime() <= new Date(filters.startTime).getTime()
  ) {
    filters.startTime = null
    filters.start_time = null
    startTimeDisplay.value = null
  }
  // 杞彉涓哄悗绔渶瑕佺殑 Unix 绉掓椂闂存埑
  filters.end_time = adjusted ? Math.floor(adjusted.getTime() / 1000) : null
}

// 绂佺敤寮€濮嬫椂闂寸殑鏃ユ湡閫夋嫨
const disableStartDate = (time: Date) => {
  if (!filters.endTime) return false
  // 寮€濮嬫棩鏈熶笉寰楁櫄浜庣粨鏉熸棩鏈燂紙鍚屾棩鍏佽锛屽叿浣撴椂闂寸敱 disableStartTime 鎺у埗锛?
  return time.getTime() > new Date(filters.endTime).getTime()
}

// 绂佺敤寮€濮嬫椂闂寸殑鏃堕棿閫夋嫨
const disableStartTime = (date: Date, type: string) => {
  if (!filters.endTime || type !== 'minute') return {}
  const endTime = new Date(filters.endTime)
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

// 绂佺敤缁撴潫鏃堕棿鐨勬棩鏈熼€夋嫨
const disableEndDate = (time: Date) => {
  if (!filters.startTime) return false
  // 缁撴潫鏃ユ湡涓嶅緱鏃╀簬寮€濮嬫棩鏈燂紙鍚屾棩鍏佽锛屽叿浣撴椂闂寸敱 disableEndTime 鎺у埗锛?
  return time.getTime() < new Date(filters.startTime).getTime()
}

// 绂佺敤缁撴潫鏃堕棿鐨勬椂闂撮€夋嫨
const disableEndTime = (date: Date, type: string) => {
  if (!filters.startTime || type !== 'minute') return {}
  const startTime = new Date(filters.startTime)
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

// 鏍煎紡鍖栨椂闂达紙鏀寔 Unix 绉掓椂闂存埑鍜屾棩鏈熷瓧绗︿覆锛?
const formatDateTime = (dateTime: number | string | null | undefined): string => {
  if (dateTime === null || dateTime === undefined || dateTime === '') return '-'
  try {
    // Unix 鏃堕棿鎴充负绉掞紝闇€杞崲涓烘绉?
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

// 澶勭悊瀵煎嚭
</script>

<style scoped lang="scss">
.alarm-records {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;

  .alarm-records__toolbar {

    .alarm-records__toolbar-left {
      position: relative;
      display: flex;
      align-items: center;
      gap: 0.16rem;
    }

    .alarm-records__toolbar-right {
      display: flex;
      align-items: center;
      gap: 0.1rem;

      .alarm-records__export-btn {
        display: flex;
        align-items: center;
        gap: 0.1rem;

        .alarm-records__export-icon {
          width: 0.16rem;
          height: 0.16rem;
          margin-right: 0.08rem;
        }
      }
    }
  }

  .alarm-records__table {
    height: calc(100% - 0.52rem);
    width: 100%;
    display: flex;
    flex-direction: column;

    .alarm-records__table-content {
      width: 100%;
      height: calc(100% - 0.92rem);
      overflow-y: auto;

      .alarm-records__table-level-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }
    }

  }

  :deep(.alarm-records__toolbar-form.el-form--inline .el-form-item) {
    margin-bottom: 0;
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
