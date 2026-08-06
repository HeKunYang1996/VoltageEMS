<template>
  <div class="rule-management vt-page-shell" ref="ruleManagementRef">
    <LoadingBg :loading="loading">
      <div class="rule-management__header vt-toolbar">
        <div class="rule-management__toolbar-left vt-toolbar__left" ref="levelSelectRef">
          <el-form :model="filters" :inline="true" class="test-form rule-management__toolbar-form vt-toolbar-form">
            <el-form-item label="Keyword:">
              <el-input v-model="filters.keyword" placeholder="Please enter keyword" />
            </el-form-item>
            <el-form-item label="Alarm Level:">
              <el-select v-model="filters.warning_level" placeholder="Please select level" clearable
                :append-to="levelSelectRef">
                <el-option label="Critical Alarm" :value="1" />
                <el-option label="Warning Alarm" :value="2" />
                <el-option label="Info Alarm" :value="3" />
              </el-select>
            </el-form-item>
            <el-form-item label="Enabled:">
              <el-select v-model="filters.enabled" placeholder="Please select enabled" clearable
                :append-to="levelSelectRef">
                <el-option label="Enabled" :value="true" />
                <el-option label="Disabled" :value="false" />
              </el-select>
            </el-form-item>
          </el-form>
        </div>
        <div class="rule-management__toolbar-right vt-toolbar__right">
          <IconButton type="warning" :icon="tableRefreshIcon" text="Reload" custom-class="rule-management__btn"
            @click="reloadFilters" />
          <IconButton type="primary" :icon="tableSearchIcon" text="Search" custom-class="rule-management__btn"
            @click="fetchTableData(true)" />
          <IconButton v-permission="'engineer'" type="primary" :icon="userAddIcon" text="New rule"
            custom-class="rule-management__btn" @click="handleAddUser" />
        </div>
      </div>
      <div class="rule-management__table vt-table-shell">
        <el-table :data="tableData" class="rule-management__table-content vt-table-content" align="left">
          <!-- <el-table-column prop="id" label="ID" class-name="table-ellipsis" width="80" /> -->
          <el-table-column prop="rule_name" label="Rule Name" class-name="table-ellipsis" min-width="120" />
          <el-table-column prop="warning_level" label="Alarm Level">
            <template #default="{ row }">
              <span class="rule-management__table-level-text" :class="`alarm-level--${row.warning_level}`">
                {{ warningLevelText[row.warning_level as 1 | 2 | 3] || '-' }}
              </span>
            </template>
          </el-table-column>
          <el-table-column prop="monitor_data" label="Monitor Data"  show-overflow-tooltip min-width="100">
            <template #default="{ row }">
              <span class="table-ellipsis__text vt-ellipsis">{{ formatMonitorData(row) }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="condition" label="Condition" show-overflow-tooltip
            min-width="80">
            <template #default="{ row }">
              <span class="table-ellipsis__text vt-ellipsis">{{ formatCondition(row) }}</span>
            </template>
          </el-table-column>
          <!-- <el-table-column prop="notification" label="Notification" show-overflow-tooltip>
          <template #default="{ row }">
            {{ Array.isArray(row.notification) ? row.notification.join(', ') : row.notification }}
          </template>
        </el-table-column> -->
          <el-table-column prop="description" label="Description" show-overflow-tooltip class-name="table-ellipsis"
            min-width="120">
            <template #default="{ row }">
              <span class="table-ellipsis__text vt-ellipsis">{{ row.description || '-' }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="created_at" label="Created At" class-name="table-ellipsis" min-width="120">
            <template #default="{ row }">
              <span class="table-ellipsis__text vt-ellipsis">{{
                formatDateTime(row.created_at)
                }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="enabled" label="Enabled" min-width="80">
            <template #default="{ row }">
              <el-switch
                v-if="canWrite"
                :model-value="row.enabled"
                :loading="switchLoadingId === row.id"
                @change="handleSwitchChange(row)"
              />
              <span v-else :class="row.enabled ? 'status-enabled' : 'status-disabled'">
                {{ row.enabled ? 'Enabled' : 'Disabled' }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="Operation" fixed="right" v-permission="'engineer'" min-width="120">
            <template #default="{ row }">
              <div class="rule-management__operation">
                <div class="rule-management__operation-item" @click="handleEdit(row)">
                  <img :src="tableEditIcon" />
                  <span class="rule-management__operation-text">Edit</span>
                </div>
                <div class="rule-management__operation-item" @click="handleDelete(row)">
                  <img :src="tableDeleteIcon" />
                  <span class="rule-management__operation-text">Delete</span>
                </div>
              </div>
            </template>
          </el-table-column>
        </el-table>

        <div id="alarm-rules-pagination-anchor" class="rule-management__pagination vt-pagination">
          <el-pagination
            v-model:current-page="pagination.page"
            v-model:page-size="pagination.pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="pagination.total"
            layout="total, sizes, prev, pager, next"
            :teleported="false"
            append-size-to="#alarm-rules-pagination-anchor"
            @size-change="handlePageSizeChange"
            @current-change="handlePageChange"
          />
        </div>
      </div>
    </LoadingBg>
    <RulesOperationForm ref="rulesOperationFormRef" @submit="fetchTableData(true)" @cancel="handleRuleCancel" />
  </div>
</template>

<script setup lang="ts">
// 姝ｇ‘寮曞叆SVG鍥炬爣锛岄伩鍏嶉儴缃插悗鍥剧墖鍔犺浇涓嶅嚭锟?
import tableRefreshIcon from '@/assets/icons/table-refresh.svg'
import tableSearchIcon from '@/assets/icons/table-search.svg'
import userAddIcon from '@/assets/icons/user-add.svg'
import tableEditIcon from '@/assets/icons/table-edit.svg'
import tableDeleteIcon from '@/assets/icons/table-delect.svg'
import RulesOperationForm from './RulesOperationForm.vue'
import type { RuleInfo } from '@/types/ruleManagement'

import { useTableData, type TableConfig } from '@/composables/useTableData'
import { usePermission } from '@/composables/usePermission'
import { enableRule, disableRule } from '@/api/alarm'

const { canWrite } = usePermission()

const ruleManagementRef = ref<HTMLElement | null>(null)
const tableConfig: TableConfig = {
  listUrl: '/alarmApi/rules',
  deleteUrl: '/alarmApi/rules/{id}',
  defaultPageSize: 20,
}
const warningLevelText = {
  1: 'Critical Alarm',
  2: 'Warning Alarm',
  3: 'Info Alarm',
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
  deleteRow,
} = useTableData<RuleInfo>(tableConfig)

filters.keyword = ''
filters.warning_level = null
filters.enabled = null

const levelSelectRef = ref<HTMLElement | null>(null)

const rulesOperationFormRef = ref()
const switchLoadingId = ref<string | number | null>(null)

// 鏍煎紡锟?MonitorData
const formatMonitorData = (row: RuleInfo) => {
  if (!row) return '-'
  const dataTypeLabel =
    row.data_type === 'M' ? 'Measurement' : row.data_type === 'A' ? 'Action' : row.data_type
  const device = row.device_name || row.channel_id
  const point = row.point_name || row.point_id
  return [row.service_type || 'inst', device, dataTypeLabel, point, row.unit]
    .filter((v) => v !== null && v !== undefined && v !== '')
    .join(' / ')
}

const formatCondition = (row: RuleInfo) => {
  if (!row || !row.operator || row.value === null || row.value === undefined) return '-'
  return `${row.operator} ${row.value}`
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

// 娣诲姞瑙勫垯
const handleAddUser = () => {
  rulesOperationFormRef.value?.open(undefined, 'create')
}

// 缂栬緫瑙勫垯
const handleEdit = (row: RuleInfo) => {
  rulesOperationFormRef.value?.open(row.id, 'edit')
}

// 鍒犻櫎瑙勫垯
const handleDelete = async (row: RuleInfo) => {
  deleteRow(
    row.id,
    `Are you sure you want to delete rule "${row.rule_name}"?`,
    ruleManagementRef.value,
  )
}
const handleSwitchChange = async (row: RuleInfo) => {
  switchLoadingId.value = row.id
  try {
    if (!row.enabled) {
      const res = await enableRule(row.id)
      if (res.message) {
        row.enabled = true
      }
    } else {
      const res = await disableRule(row.id)
      if (res.message) {
        row.enabled = false
      }
    }
  } finally {
    switchLoadingId.value = null
  }
}

// 澶勭悊瑙勫垯琛ㄥ崟鍙栨秷
const handleRuleCancel = () => {
  console.log('Rule form cancelled')
}
</script>

<style scoped lang="scss">
.rule-management {
  .rule-management__header {
    .rule-management__toolbar-left {
      position: relative;

    }

    .rule-management__toolbar-right {
      display: flex;
      align-items: center;
      gap: 0.1rem;
    }

    .rule-management__btn {
      display: flex;
      align-items: center;
      gap: 0.08rem;

      .rule-management__btn-icon {
        width: 0.14rem;
        height: 0.14rem;
        margin-right: 0.08rem;
      }
    }
  }

  .rule-management__table {
    .rule-management__table-content {
      .rule-management__operation {
        display: flex;
        align-items: center;
        gap: 0.2rem;

        .rule-management__operation-item {
          cursor: pointer;
          display: flex;
          align-items: center;

          img {
            width: 0.14rem;
            height: 0.14rem;
            margin-right: 0.04rem;
            object-fit: contain;
          }
        }
      }

      .rule-management__table-level-text {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .status-enabled {
        color: var(--el-color-success);
      }

      .status-disabled {
        color: var(--el-text-color-secondary);
      }
    }
  }

  :deep(.rule-management__table-content .el-switch) {
    height: 0.22rem;
  }

  :deep(.rule-management__toolbar-form.el-form--inline .el-form-item) {
    margin-bottom: 0;
  }
}
</style>
