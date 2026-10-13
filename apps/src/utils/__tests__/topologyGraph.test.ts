import { describe, expect, it } from 'vitest'
import {
  isTopologyLoadProduct,
  resolveMeterLoadNodeIds,
  TOPOLOGY_LOAD_PRODUCTS,
} from '../topologyGraph'

describe('utils/topologyGraph.ts', () => {
  describe('isTopologyLoadProduct', () => {
    it('recognizes load products from voltage-product-lib', () => {
      for (const name of TOPOLOGY_LOAD_PRODUCTS) {
        expect(isTopologyLoadProduct(name)).toBe(true)
      }
      expect(isTopologyLoadProduct('Meter')).toBe(false)
      expect(isTopologyLoadProduct('Distribution_Board')).toBe(false)
    })
  })

  describe('resolveMeterLoadNodeIds', () => {
    const nodes = [
      {
        id: 'load-1',
        data: { label: 'Load', productName: 'Single_Phase_Load' },
      },
      {
        id: 'meter-load',
        data: { label: 'Load Meter', productName: 'Meter', instanceId: 201 },
      },
      {
        id: 'meter-pv',
        data: { label: 'PV Meter', productName: 'Meter', instanceId: 202 },
      },
      {
        id: 'inverter-1',
        data: { label: 'Hybrid', productName: 'Hybrid_Inverter', instanceId: 101 },
      },
    ]

    it('returns Meter nodes adjacent to a Load product via edges', () => {
      const ids = resolveMeterLoadNodeIds(nodes, [
        { source: 'load-1', target: 'meter-load' },
        { source: 'inverter-1', target: 'meter-pv' },
      ])
      expect(ids).toEqual(['meter-load'])
    })

    it('treats the graph as undirected', () => {
      const ids = resolveMeterLoadNodeIds(nodes, [{ source: 'meter-load', target: 'load-1' }])
      expect(ids).toEqual(['meter-load'])
    })

    it('returns empty when Meter is not connected to any Load product', () => {
      expect(resolveMeterLoadNodeIds(nodes, [{ source: 'inverter-1', target: 'meter-pv' }])).toEqual(
        [],
      )
      expect(resolveMeterLoadNodeIds(nodes, [])).toEqual([])
    })

    it('ignores parentNode and Distribution_Board semantics', () => {
      const legacyNodes = [
        {
          id: 'meter-legacy',
          data: { label: 'Meter', productName: 'Meter', instanceId: 301 },
          parentNode: 'board-1',
        },
        {
          id: 'board-1',
          data: { label: 'Board', productName: 'Distribution_Board' },
        },
      ]
      expect(resolveMeterLoadNodeIds(legacyNodes, [])).toEqual([])
    })
  })
})
