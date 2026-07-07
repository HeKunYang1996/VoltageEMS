<template>
  <div class="voltage-class control-records vt-page-shell">
    <LoadingBg :loading="loading">
      <div class="control-records__toolbar vt-toolbar">
        <div class="control-records__toolbar-left vt-toolbar__left">
          <el-form :inline="true" class="control-records__toolbar-form vt-toolbar-form">
            <el-form-item label="Name:">
              <el-input
                v-model="filters.name"
                placeholder="Please enter name"
                clearable
                class="control-records__search-input"
                @keyup.enter="fetchTableData(true)"
              />
            </el-form-item>
          </el-form>
        </div>

        <div class="control-records__toolbar-right vt-toolbar__right">
          <IconButton
            type="warning"
            :icon="reloadIcon"
            text="Reload"
            custom-class="control-records__btn"
            @click="reloadFilters"
          />
          <IconButton
            type="primary"
            :icon="searchIcon"
            text="Search"
            custom-class="control-records__btn"
            @click="fetchTableData(true)"
          />
        </div>
      </div>

      <div class="control-records__table vt-table-shell">
        <el-table :data="tableData" class="control-records__table-content vt-table-content" align="left">
          <el-table-column prop="name" label="Name" min-width="160" class-name="table-ellipsis" />
          <el-table-column prop="description" label="Description" min-width="180" show-overflow-tooltip />
          <el-table-column prop="enabled" label="Enabled" min-width="100">
            <template #default="{ row }">
              <span :class="row.enabled ? 'control-records__enabled' : 'control-records__disabled'">
                {{ row.enabled ? 'Enabled' : 'Disabled' }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="Operation" fixed="right" min-width="160" class-name="leave-alone">
            <template #default="{ row }">
              <div class="control-records__operation">
                <div class="control-records__operation-item" @click="openHistory(row)">
                  <img :src="tableDetailIcon" alt="" />
                  <span class="control-records__operation-text">Trigger History</span>
                </div>
              </div>
            </template>
          </el-table-column>
        </el-table>

        <div id="control-records-pagination-anchor" class="control-records__pagination vt-pagination">
          <el-pagination
            v-model:current-page="pagination.page"
            v-model:page-size="pagination.pageSize"
            :page-sizes="[10, 20, 50, 100]"
            :total="pagination.total"
            layout="total, sizes, prev, pager, next"
            :teleported="false"
            append-size-to="#control-records-pagination-anchor"
            @size-change="handlePageSizeChange"
            @current-change="handlePageChange"
          />
        </div>
      </div>
    </LoadingBg>

    <RuleHistoryDialog ref="historyDialogRef" />
  </div>
</template>

<script setup lang="ts">
import reloadIcon from '@/assets/icons/table-refresh.svg'
import searchIcon from '@/assets/icons/table-search.svg'
import tableDetailIcon from '@/assets/icons/button-detail.svg'
import LoadingBg from '@/components/common/LoadingBg.vue'
import RuleHistoryDialog from './RuleHistoryDialog.vue'
import { useTableData, type TableConfig } from '@/composables/useTableData'
import type { ModRuleSummary } from '@/types/controlRule'

const tableConfig: TableConfig = {
  listUrl: '/ruleApi/api/rules',
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
} = useTableData<ModRuleSummary>(tableConfig)

filters.name = ''

const historyDialogRef = ref<InstanceType<typeof RuleHistoryDialog> | null>(null)

const openHistory = (row: ModRuleSummary) => {
  historyDialogRef.value?.open(row)
}

onMounted(() => {
  void fetchTableData(true)
})
</script>

<style scoped lang="scss">
.voltage-class.control-records {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;

  .control-records__toolbar {
    padding-bottom: 0.2rem;
    display: flex;
    align-items: center;
    justify-content: space-between;

    .control-records__toolbar-left {
      position: relative;
      display: flex;
      align-items: center;
      gap: 0.16rem;
    }

    .control-records__toolbar-right {
      display: flex;
      align-items: center;
      gap: 0.1rem;
    }
  }

  :deep(.control-records__toolbar-form.el-form--inline .el-form-item) {
    margin-bottom: 0;
  }

  .control-records__table {
    height: calc(100% - 0.52rem);
    width: 100%;
    display: flex;
    flex-direction: column;

    .control-records__table-content {
      width: 100%;
      height: calc(100% - 0.92rem);
      overflow-y: auto;
    }

    .control-records__pagination {
      position: relative;
      padding: 0.2rem 0;
      display: flex;
      justify-content: flex-end;
    }
  }

  .control-records__enabled {
    color: var(--el-color-success);
  }

  .control-records__disabled {
    color: var(--el-text-color-secondary);
  }

  .control-records__operation {
    display: flex;
    align-items: center;
    gap: 0.2rem;

    .control-records__operation-item {
      cursor: pointer;
      display: flex;
      align-items: center;
    }
  }
}
</style>
