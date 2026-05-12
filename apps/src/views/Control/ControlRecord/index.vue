<template>
  <div class="voltage-class control-records">
    <!-- 表格工具栏 -->
    <div class="control-records__toolbar">
      <div class="control-records__toolbar-left">
        <el-form :inline="true" class="control-records__toolbar-form">
          <el-form-item label="Name:">
            <el-input
              v-model="searchName"
              placeholder="Please enter name"
              clearable
              class="control-records__search-input"
            />
          </el-form-item>
        </el-form>
      </div>

      <div class="control-records__toolbar-right">
        <IconButton
          type="warning"
          :icon="reloadIcon"
          text="Reload"
          custom-class="control-records__btn"
          @click="handleReload"
        />
        <IconButton
          type="primary"
          :icon="searchIcon"
          text="Search"
          custom-class="control-records__btn"
          @click="handleSearch"
        />
      </div>
    </div>

    <!-- 表格 -->
    <div class="control-records__table">
      <el-table :data="pagedData" class="control-records__table-content">
        <el-table-column
          prop="rule_name"
          label="Name"
          min-width="2rem"
          class-name="table-ellipsis"
        />
        <el-table-column prop="triggered_at" label="Trigger Time" min-width="1.4rem" class-name="table-ellipsis">
          <template #default="{ row }">
            <span class="table-ellipsis__text">{{ formatDateTime(row.triggered_at) }}</span>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <div class="control-records__pagination">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="filteredData.length"
          layout="total, sizes, prev, pager, next"
          @size-change="currentPage = 1"
          @current-change="(v: number) => (currentPage = v)"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import reloadIcon from '@/assets/icons/table-refresh.svg'
import searchIcon from '@/assets/icons/table-search.svg'

// ─── 假数据（暂无记录）─────────────────────────────────────────
interface ControlRecord {
  id: number
  rule_name: string
  triggered_at: number
}

const mockData: ControlRecord[] = []

// ─── 搜索 & 过滤 ───────────────────────────────────────────────
const searchName = ref('')
const activeSearch = ref('')
const currentPage = ref(1)
const pageSize = ref(20)

const handleSearch = () => {
  activeSearch.value = searchName.value
  currentPage.value = 1
}

const handleReload = () => {
  searchName.value = ''
  activeSearch.value = ''
  currentPage.value = 1
}

const filteredData = computed(() => {
  const kw = activeSearch.value.trim().toLowerCase()
  if (!kw) return mockData
  return mockData.filter((r) => r.rule_name.toLowerCase().includes(kw))
})

const pagedData = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredData.value.slice(start, start + pageSize.value)
})

// ─── 时间格式化 ────────────────────────────────────────────────
const formatDateTime = (ts: number | string | null | undefined): string => {
  if (ts === null || ts === undefined || ts === '') return '-'
  try {
    const date = typeof ts === 'number' ? new Date(ts * 1000) : new Date(ts)
    if (isNaN(date.getTime())) return String(ts)
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
  } catch {
    return String(ts)
  }
}
</script>

<style scoped lang="scss">
.voltage-class.control-records {
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
      padding: 0.2rem 0;
      display: flex;
      justify-content: flex-end;
    }
  }

  :deep(.control-records__table-content .table-ellipsis .cell) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .table-ellipsis__text {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }


}
</style>
