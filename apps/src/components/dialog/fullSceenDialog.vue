<template>
  <el-dialog
    v-model="dialogVisible"
    :title="null"
    :modal="true"
    :close-on-click-modal="false"
    :append-to-body="true"
    :modal-append-to-body="true"
    fullscreen
  >
    <!-- dialog-head插槽，默认显示标题 -->
    <template #header>
      <slot name="dialog-head">
        <div class="dialog-head">
          <img class="dialog-head-icon" src="../../assets/icons/card-icon.svg" />
          <span class="dialog-head-title">{{ props.title }}</span>
        </div>
      </slot>
    </template>

    <!-- dialog-body插槽，默认显示表单内容 -->
    <template #default>
      <slot name="dialog-body"> </slot>
    </template>

    <!-- dialog-footer插槽，默认显示底部按钮 -->
    <template #footer>
      <slot name="dialog-footer"> </slot>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
const props = defineProps<{
  title: string
}>()
const dialogVisible = ref(false)
// 暴露方法和属性
defineExpose({
  dialogVisible,
})
</script>

<style lang="scss" scoped>
.dialog-head {
  display: flex;
  align-items: center;

  .dialog-head-icon {
    width: 0.2rem;
    height: 0.2rem;
    margin-right: 0.03rem;
  }

  .dialog-head-title {
    font-weight: 700;
    font-size: 0.18rem;
    line-height: 0.2rem;
    letter-spacing: 0%;
    color: rgba(245, 247, 255, 1);
  }
}
</style>
