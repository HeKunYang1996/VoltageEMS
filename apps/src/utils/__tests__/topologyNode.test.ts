import { describe, expect, it } from 'vitest'
import { collectFlowNodeInstanceIds, getFlowNodeInstanceId } from '../topologyNode'
import type { TopologyNodeData } from '@/types/stationTopology'

describe('utils/topologyNode.ts', () => {
  describe('getFlowNodeInstanceId', () => {
    it('returns positive instanceId from node data', () => {
      const data: TopologyNodeData = { label: 'PCS', instanceId: 42 }
      expect(getFlowNodeInstanceId(data)).toBe(42)
    })

    it('returns null for missing or invalid instanceId', () => {
      expect(getFlowNodeInstanceId(undefined)).toBeNull()
      expect(getFlowNodeInstanceId({ label: 'PCS', instanceId: null })).toBeNull()
      expect(getFlowNodeInstanceId({ label: 'PCS', instanceId: 0 })).toBeNull()
    })
  })

  describe('collectFlowNodeInstanceIds', () => {
    it('collects instanceIds only for selected nodes', () => {
      const nodes = [
        { id: 'n1', data: { label: 'PV', instanceId: 1 } },
        { id: 'n2', data: { label: 'DG', instanceId: 2 } },
        { id: 'n3', data: { label: 'ESS', instanceId: null } },
      ]
      expect(collectFlowNodeInstanceIds(nodes, new Set(['n1', 'n3']))).toEqual([1])
    })
  })
})
