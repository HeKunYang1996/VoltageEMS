/**
 * 拓扑感知的 WebSocket 订阅 composable。
 *
 * 规则：
 *   - onMounted：若拓扑已解析（getChannels 返回非空数组）则立即订阅；否则静默等待。
 *   - watch(channels)：拓扑加载完成或重新加载后 channels 变化时，自动取消旧订阅并重新订阅。
 *   - onUnmounted：自动清理，无内存泄漏。
 *
 * 设计原则：
 *   不使用 { immediate: true }，避免在组件挂载前（setup 阶段）发起订阅；
 *   通过 onMounted 作为"已挂载且拓扑已就绪"时的首次订阅触发点。
 *
 * 使用示例：
 *   useTopologySubscribe(
 *     () => topoStore.getInstanceIds('Battery'),
 *     { source: 'inst', dataTypes: ['M'], interval: 1000 },
 *     { onBatchDataUpdate: handler },
 *   )
 */

import { computed, onMounted, onUnmounted, watch } from 'vue'
import wsManager from '@/utils/websocket'
import type { ListenerConfig, SubscriptionConfig } from '@/types/websocket'

export default function useTopologySubscribe(
  getChannels: () => number[],
  config: Omit<SubscriptionConfig, 'channels'>,
  listeners: Partial<ListenerConfig>,
) {
  const channels = computed(getChannels)
  let subId = ''

  function resub() {
    if (subId) { wsManager.unsubscribe(subId); subId = '' }
    const ch = channels.value
    if (!ch.length) return           // 拓扑未加载或该产品不存在时静默跳过
    subId = wsManager.subscribe({ ...config, channels: ch }, listeners)
  }

  // 组件挂载后：若拓扑已解析则立即订阅（拓扑先于组件加载完成的场景）
  onMounted(resub)

  // 拓扑加载完成或重新加载时：channels 变化后重新订阅（拓扑后于组件加载的场景）
  // 不使用 immediate，避免与 onMounted 重复订阅
  watch(channels, resub)

  onUnmounted(() => {
    if (subId) { wsManager.unsubscribe(subId); subId = '' }
  })
}
