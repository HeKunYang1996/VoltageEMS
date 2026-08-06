import { describe, expect, it } from 'vitest'
import {
  buildExecutionTree,
  formatAssignmentDetail,
  formatCalculationDetail,
  formatConditionDetail,
  formatDisplayVariable,
  formatPeriodDeltaDetail,
  getExecutedNodeIdSet,
  getHistoryErrors,
  getTriggerReason,
  isHistorySuccess,
} from '../ruleHistoryDisplay'
import type { RuleHistoryDisplayStep, RuleHistoryItem } from '@/types/controlRule'

describe('ruleHistoryDisplay', () => {
  it('returns trigger_reason text as-is', () => {
    const row = {
      result: {
        display: {
          trigger_reason: 'Condition matched — SOC Branch: BMS-1·SOC (78 %)>=70',
        },
      },
    } as RuleHistoryItem

    expect(getTriggerReason(row)).toBe(
      'Condition matched — SOC Branch: BMS-1·SOC (78 %)>=70',
    )
  })

  it('treats non-empty error as failed even when success is true', () => {
    const row = {
      error: '2 of 3 action(s) failed to write',
      result: { success: true },
    } as RuleHistoryItem

    expect(isHistorySuccess(row)).toBe(false)
  })

  it('treats unmatched switch branch as success when error is null', () => {
    const row = {
      error: null,
      result: { success: true },
    } as RuleHistoryItem

    expect(isHistorySuccess(row)).toBe(true)
  })

  it('splits engine errors joined by semicolon', () => {
    const row = {
      error: 'write failed on A; timeout on B',
    } as RuleHistoryItem

    expect(getHistoryErrors(row)).toEqual(['write failed on A', 'timeout on B'])
  })

  it('appends formula_resolved for combined variables', () => {
    expect(
      formatDisplayVariable({
        key: 'X1',
        label: 'X1',
        value: 10,
        formula_resolved: 'A + B',
      }),
    ).toContain('⟵ A + B')
  })

  it('builds a multi-branch execution tree from fan-out steps', () => {
    const steps: RuleHistoryDisplayStep[] = [
      { node_id: 'start', label: 'Start', node_kind: 'start' },
      {
        node_id: 'sw1',
        label: 'Switch',
        node_kind: 'switch',
        conditions: [{ port: 'out1', expression: 'x>0', result: true }],
      },
      {
        node_id: 'a1',
        label: 'Action A',
        node_kind: 'change',
        terminal: true,
        terminal_reason: 'this branch stops here',
      },
      {
        node_id: 'a2',
        label: 'Action B',
        node_kind: 'change',
        terminal: true,
        terminal_reason: 'this branch stops here',
      },
    ]

    const tree = buildExecutionTree(steps)
    expect(tree).toHaveLength(1)
    expect(tree[0].step.node_id).toBe('start')
    expect(tree[0].children).toHaveLength(1)
    expect(tree[0].children[0].step.node_id).toBe('sw1')
    expect(tree[0].children[0].children).toHaveLength(2)
    expect(tree[0].children[0].children.map((c) => c.step.node_id)).toEqual(['a1', 'a2'])
  })

  it('keeps deep switch branches and mid-node fan-out as siblings', () => {
    const deepSwitch: RuleHistoryDisplayStep[] = [
      { node_id: 'start', label: 'Start', node_kind: 'start' },
      { node_id: 'sw1', label: 'Switch', node_kind: 'switch' },
      { node_id: 'mid', label: 'Mid', node_kind: 'change' },
      {
        node_id: 'leaf1',
        label: 'Leaf1',
        node_kind: 'change',
        terminal: true,
        terminal_reason: 'done',
      },
      {
        node_id: 'leaf2',
        label: 'Leaf2',
        node_kind: 'change',
        terminal: true,
        terminal_reason: 'done',
      },
    ]
    const deepTree = buildExecutionTree(deepSwitch)
    expect(deepTree[0].children[0].children.map((c) => c.step.node_id)).toEqual([
      'mid',
      'leaf2',
    ])
    expect(deepTree[0].children[0].children[0].children[0].step.node_id).toBe('leaf1')

    const midFanOut: RuleHistoryDisplayStep[] = [
      { node_id: 'a', label: 'A', node_kind: 'change' },
      { node_id: 'b', label: 'B', node_kind: 'change', terminal: true },
      { node_id: 'c', label: 'C', node_kind: 'change', terminal: true },
    ]
    const midTree = buildExecutionTree(midFanOut)
    expect(midTree).toHaveLength(1)
    expect(midTree[0].children.map((c) => c.step.node_id)).toEqual(['b', 'c'])
  })

  it('collects multi-path node ids as a set', () => {
    const row = {
      result: {
        execution_path: ['start', 'sw1', 'a1', 'a2'],
        display: {
          execution_steps: [
            { node_id: 'start', label: 'Start' },
            { node_id: 'sw1', label: 'Switch' },
            { node_id: 'a1', label: 'A' },
            { node_id: 'a2', label: 'B' },
          ],
        },
      },
    } as RuleHistoryItem

    const ids = getExecutedNodeIdSet(row)
    expect(ids.has('a1')).toBe(true)
    expect(ids.has('a2')).toBe(true)
  })

  it('formats structured step details', () => {
    expect(
      formatConditionDetail({
        port: 'out001',
        expression: 'X1>=70',
        expression_resolved: 'BMS-1·SOC (78 %)>=70',
        result: true,
      }),
    ).toContain('BMS-1·SOC (78 %)>=70')

    expect(
      formatAssignmentDetail({
        target: { instance_name: 'BMS-1', point_name: 'Adjustment', unit: '', value: 1 },
        written_value: 1,
        value_source: 'literal',
        raw_value: 1,
        success: false,
      }),
    ).toContain('(failed)')

    expect(
      formatCalculationDetail({
        output_variable: 'Y1',
        formula_resolved: 'a + b',
        result: 3,
        success: true,
      }),
    ).toContain('Y1 = a + b → 3')

    expect(
      formatPeriodDeltaDetail({
        node_id: 'n1',
        label: 'Period',
        node_kind: 'periodDelta',
        period: 'daily',
        input: { instance_name: 'Meter', point_name: 'Energy', value: 120.5, unit: 'kWh' },
        output: { instance_name: 'Meter', point_name: 'DailyDelta', unit: 'kWh' },
        delta: 12.5,
        success: true,
      }),
    ).toContain('Δ=12.50')
  })
})
