<template>
  <div class="card">
    <LoadingBg :loading="props.loading">
      <div class="card__header">
        <div class="card__header-icon"></div>
        <div class="card__header-title">{{ props.title }}</div>
      </div>
      <div class="card__content">
        <slot></slot>
      </div>
      <div class="card__content-footer">
        <slot name="footer"></slot>
      </div>
    </LoadingBg>
  </div>
</template>

<script lang="ts" setup>
const props = defineProps({
  title: {
    type: String,
    required: true,
  },
  loading: {
    type: Boolean,
    default: false,
  },
})
</script>

<style lang="scss" scoped>
.card {
  width: 100%;
  height: 100%;
  min-height: 0;
  background: var(--vt-bg-glass);
  padding: 0.2rem;
  border: 0.01rem solid;
  border-image: var(--vt-border-gradient) 1;
  backdrop-filter: var(--vt-backdrop-blur);
  display: flex;
  flex-direction: column;

  .card__header {
    display: flex;
    align-items: center;

    .card__header-icon {
      width: 0.2rem;
      height: 0.2rem;
      background-image: url('@/assets/images/card-icon.png');
      background-size: cover;
      background-position: center;
      background-repeat: no-repeat;
    }

    .card__header-title {
      margin-left: 0.03rem;
      font-family: var(--vt-font-family-base);
      font-weight: var(--vt-font-weight-bold);
      font-size: var(--vt-font-size-lg);
      line-height: 1em;
      color: var(--vt-text-primary);
    }
  }

  .card__content {
    flex: 1 1 auto;
    min-height: 0;
    width: 100%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;

    // &.no-footer {
    //   height: calc(100% - 0.2rem);
    // }
  }

  .card__content-footer {
    flex: none;
    width: 100%;
    display: flex;
    flex-direction: row-reverse;
    // border-top: 0.01rem solid rgba(255, 255, 255, 0.1);

    .card__content-footer-button {
      margin: 0.3rem 0 0.1rem 0;

      .card__content-footer-button-icon {
        width: 0.16rem;
        height: 0.16rem;
        margin-right: 0.08rem;
        object-position: center;
        object-fit: contain;
      }
    }
  }
}
</style>
