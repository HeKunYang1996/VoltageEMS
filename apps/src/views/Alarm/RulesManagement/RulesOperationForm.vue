<template>
  <FormDialog width="13.46rem" ref="dialogRef" :title="dialogTitle">
    <template #dialog-body>
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="0.98rem"
        class="rules-form"
        label-position="right"
        inline
      >
        <el-form-item label="Rule Name:" prop="rule_name">
          <el-input v-model="form.rule_name" placeholder="Enter rule name" />
        </el-form-item>

        <div class="monitor-data-group" ref="monitorDataGroupRef">
          <el-form-item label="Monitor Data:" prop="channel_id" class="rules-form__compact-item">
            <el-select
              v-model="form.channel_id"
              placeholder="Select Instance"
              popper-class="rules-dialog-popper"
              :append-to="monitorDataGroupRef"
              @change="handleInstanceChange"
              :disabled="loadingInstances"
            >
              <el-option
                v-for="item in instanceList"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
          <el-form-item prop="data_type" class="rules-form__compact-item">
            <el-select
              v-model="form.data_type"
              placeholder="Select Point Type"
              popper-class="rules-dialog-popper"
              :append-to="monitorDataGroupRef"
              @change="handlePointTypeChange"
              :disabled="!form.channel_id || loadingPoints"
            >
              <el-option
                v-for="item in data_types"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
          <el-form-item prop="point_id" class="rules-form__compact-item">
            <el-select
              v-model="form.point_id"
              placeholder="Select Point"
              popper-class="rules-dialog-popper"
              :append-to="monitorDataGroupRef"
              :disabled="!form.channel_id || !form.data_type || loadingPoints"
            >
              <el-option
                v-for="item in points"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
        </div>
        <div class="alarm-level-group" ref="alarmLevelGroupRef">
          <el-form-item label="Alarm Level:" prop="warning_level">
            <el-select
              v-model="form.warning_level"
              placeholder="Select level"
              popper-class="rules-dialog-popper"
              :append-to="alarmLevelGroupRef"
            >
              <el-option
                v-for="item in alarmLevelOptions"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
        </div>
        <div class="condition-group" ref="conditionGroupRef">
          <el-form-item prop="operator" label="Condition:" class="rules-form__compact-item">
            <el-select
              v-model="form.operator"
              placeholder="Operator"
              popper-class="rules-dialog-popper"
              :append-to="conditionGroupRef"
            >
              <el-option label=">" value=">" />
              <el-option label=">=" value=">=" />
              <el-option label="<" value="<" />
              <el-option label="<=" value="<=" />
              <el-option label="=" value="=" />
            </el-select>
          </el-form-item>
          <el-form-item prop="value" class="rules-form__compact-item">
            <el-input-number
              v-model="form.value"
              :min="0"
              :max="999999"
              :controls="false"
              placeholder="Value"
              class="rules-form__value-input"
              align="left"
            />
          </el-form-item>
        </div>

        <el-form-item label="Enabled:" prop="enabled" class="rules-form__full-row">
          <el-switch v-model="form.enabled" />
        </el-form-item>

        <el-form-item label="Description:" prop="description" class="rules-form__full-row">
          <el-input
            v-model="form.description"
            type="textarea"
            :rows="3"
            placeholder="Enter description (optional)"
            maxlength="50"
            show-word-limit
          />
        </el-form-item>
      </el-form>
    </template>

    <template #dialog-footer>
      <el-button type="warning" @click="onCancel" style="margin-right: 0.2rem">Cancel</el-button>
      <el-button type="primary" @click="onSubmit">Submit</el-button>
    </template>
  </FormDialog>
</template>

<script setup lang="ts">
import type { FormInstance } from 'element-plus'
import { getRuleDetail, createRule, updateRule } from '@/api/alarm'
import { getInstancePoints } from '@/api/devicesManagement'
import { useDeviceTopologyStore } from '@/stores/deviceTopology'
import type {
  AlarmDataType,
  RuleFormModel,
  DialogExpose,
  Operator,
  UpdateAlarmRulePayload,
} from '@/types/ruleManagement'

const SERVICE_TYPE = 'inst' as const

const formRef = ref<FormInstance>()
const dialogRef = ref<DialogExpose>()
const monitorDataGroupRef = ref<HTMLElement>()
const alarmLevelGroupRef = ref<HTMLElement>()
const conditionGroupRef = ref<HTMLElement>()
const topoStore = useDeviceTopologyStore()

const getDefaultForm = (): RuleFormModel => ({
  rule_name: '',
  service_type: SERVICE_TYPE,
  channel_id: undefined,
  point_id: null,
  data_type: null,
  warning_level: null,
  operator: null,
  value: null,
  description: '',
  enabled: true,
})

const form = ref<RuleFormModel>(getDefaultForm())

const instanceList = ref<Array<{ label: string; value: number }>>([])
const loadingInstances = ref(false)

interface InstancePointOption {
  point_id: number
  signal_name: string
  point_type: AlarmDataType
}

const allPointsData = ref<InstancePointOption[]>([])
const points = computed(() => {
  if (!form.value.data_type) return []
  return allPointsData.value
    .filter((point) => point.point_type === form.value.data_type)
    .map((point) => ({
      label: String(point.signal_name || `Point ${point.point_id}`),
      value: Number(point.point_id),
    }))
})
const loadingPoints = ref(false)

const data_types = [
  { label: 'Measurement', value: 'M' },
  { label: 'Action', value: 'A' },
]

const alarmLevelOptions = [
  { label: 'Critical Alarm', value: 1 },
  { label: 'Warning Alarm', value: 2 },
  { label: 'Info Alarm', value: 3 },
]

const rules = {
  rule_name: [{ required: true, message: 'Please input rule name', trigger: 'blur' }],
  channel_id: [{ required: true, message: 'Required', trigger: 'change' }],
  point_id: [{ required: true, message: 'Required', trigger: 'change' }],
  data_type: [{ required: true, message: 'Required', trigger: 'change' }],
  warning_level: [{ required: true, message: 'Please select alarm level', trigger: 'change' }],
  operator: [{ required: true, message: 'Please select operator', trigger: 'change' }],
  value: [{ required: true, message: 'Please input value', trigger: 'blur' }],
}

const mode = ref<'create' | 'edit'>('create')
const dialogTitle = computed(() => (mode.value === 'edit' ? 'Edit Rule' : 'New Rule'))

const loadInstances = async () => {
  try {
    loadingInstances.value = true
    if (!topoStore.loaded) {
      await topoStore.load()
    }

    const options: Array<{ label: string; value: number }> = []
    const seen = new Set<number>()
    for (const binding of topoStore.bindings) {
      for (const inst of binding.instances) {
        const id = Number(inst.instanceId)
        if (!Number.isFinite(id) || id <= 0 || seen.has(id)) continue
        seen.add(id)
        const name = String(inst.instanceName || `Instance ${id}`)
        const product = binding.productName ? ` (${binding.productName})` : ''
        options.push({ label: `${name}${product}`, value: id })
      }
    }
    options.sort((a, b) => a.value - b.value)
    instanceList.value = options
  } catch (error) {
    console.error('Failed to load instances:', error)
    instanceList.value = []
  } finally {
    loadingInstances.value = false
  }
}

const loadAllPoints = async (instanceId: number) => {
  if (!instanceId) {
    allPointsData.value = []
    return
  }
  try {
    loadingPoints.value = true
    const res = await getInstancePoints(instanceId)
    const allPoints: InstancePointOption[] = []

    if (res.success && res.data) {
      const measurements = res.data.measurements ?? {}
      for (const [key, item] of Object.entries(measurements)) {
        const pointId = Number(item.measurement_id ?? item.point_index ?? key)
        if (!Number.isFinite(pointId)) continue
        allPoints.push({
          point_id: pointId,
          signal_name: String(item.name || `Point ${pointId}`),
          point_type: 'M',
        })
      }

      const actions = res.data.actions ?? {}
      for (const [key, item] of Object.entries(actions)) {
        const pointId = Number(item.action_id ?? item.point_index ?? key)
        if (!Number.isFinite(pointId)) continue
        allPoints.push({
          point_id: pointId,
          signal_name: String(item.name || `Point ${pointId}`),
          point_type: 'A',
        })
      }
    }

    allPointsData.value = allPoints
  } catch (error) {
    console.error('Failed to load instance points:', error)
    allPointsData.value = []
  } finally {
    loadingPoints.value = false
  }
}

const handleInstanceChange = async () => {
  form.value.data_type = null
  form.value.point_id = null
  allPointsData.value = []
  if (form.value.channel_id) {
    await loadAllPoints(Number(form.value.channel_id))
  }
}

const handlePointTypeChange = () => {
  form.value.point_id = null
}

const rules_id = ref<string>()
async function open(rulesId?: string, openMode: 'create' | 'edit' = 'create') {
  try {
    mode.value = openMode
    form.value = getDefaultForm()
    rules_id.value = rulesId || ''

    await loadInstances()

    if (rulesId) {
      const res = await getRuleDetail(rules_id.value)
      if (res.success && res.data.list?.length) {
        const rule = res.data.list[0]
        form.value.rule_name = rule.rule_name
        form.value.service_type = SERVICE_TYPE
        form.value.channel_id = rule.channel_id
        form.value.point_id = rule.point_id
        form.value.data_type = (rule.data_type as AlarmDataType) || null
        form.value.warning_level = rule.warning_level
        form.value.operator = rule.operator as Operator
        form.value.value = rule.value
        form.value.description = rule.description || ''
        form.value.enabled = rule.enabled

        if (form.value.channel_id) {
          await loadAllPoints(Number(form.value.channel_id))
        }
      }
    }
    nextTick(() => {
      setTimeout(() => {
        formRef.value?.clearValidate()
      }, 100)
    })
    dialogRef.value && (dialogRef.value.dialogVisible = true)
  } catch (error) {
    console.error(error)
  }
}

function close() {
  dialogRef.value && (dialogRef.value.dialogVisible = false)
}

const emit = defineEmits<{
  (e: 'submit', value: RuleFormModel): void
  (e: 'cancel'): void
}>()

function onCancel() {
  close()
  emit('cancel')
}

function buildCreatePayload() {
  return {
    rule_name: form.value.rule_name,
    service_type: SERVICE_TYPE,
    channel_id: Number(form.value.channel_id),
    data_type: String(form.value.data_type),
    point_id: Number(form.value.point_id),
    warning_level: Number(form.value.warning_level),
    operator: String(form.value.operator),
    value: Number(form.value.value),
    enabled: form.value.enabled,
    description: form.value.description || undefined,
  }
}

function buildUpdatePayload(): UpdateAlarmRulePayload {
  return {
    service_type: SERVICE_TYPE,
    channel_id: form.value.channel_id != null ? Number(form.value.channel_id) : undefined,
    data_type: form.value.data_type ?? undefined,
    point_id: form.value.point_id != null ? Number(form.value.point_id) : undefined,
    rule_name: form.value.rule_name,
    warning_level: form.value.warning_level != null ? Number(form.value.warning_level) : undefined,
    operator: form.value.operator ?? undefined,
    value: form.value.value != null ? Number(form.value.value) : undefined,
    enabled: form.value.enabled,
    description: form.value.description,
  }
}

async function onSubmit() {
  formRef.value?.validate(async (valid) => {
    if (!valid) return
    if (mode.value === 'create') {
      const res = await createRule(buildCreatePayload() as RuleFormModel)
      if (res.success) {
        emit('submit', form.value)
        close()
      } else {
        throw new Error(res.message)
      }
    } else {
      if (!rules_id.value) {
        throw new Error('rules_id is required')
      }
      const res = await updateRule(rules_id.value, buildUpdatePayload())
      if (res.success) {
        emit('submit', form.value)
        close()
      }
    }
  })
}

defineExpose({ open, close })
</script>

<style scoped lang="scss">
.rules-form {
  display: flex;
  flex-wrap: wrap;

  .monitor-data-group,
  .alarm-level-group,
  .condition-group {
    position: relative;
    display: flex;
    gap: 0.16rem;
  }

  .rules-form__compact-item {
    margin-right: 0 !important;
  }

  .rules-form__full-row {
    width: 100%;
    margin-right: 0 !important;
  }

  :deep(.rules-form__value-input) {
    width: 4.96rem;
  }

  :deep(.el-switch) {
    height: 0.32rem;
  }
}
</style>
