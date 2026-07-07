import type {
  RuleHistoryDisplayAction,
  RuleHistoryDisplayStep,
  RuleHistoryDisplayVariable,
  RuleHistoryItem,
} from '@/types/controlRule'

const formatNumber = (value: number) =>
  Number.isInteger(value) ? String(value) : value.toFixed(2)

export const getHistoryDisplay = (row: RuleHistoryItem) => row.result?.display

export const getHistorySummary = (row: RuleHistoryItem): string => {
  const summary = getHistoryDisplay(row)?.summary
  if (summary) return summary
  if (row.error) return row.error
  return '-'
}

export const getTriggerReason = (row: RuleHistoryItem): string =>
  getHistoryDisplay(row)?.trigger_reason || '-'

export const getDisplayVariables = (row: RuleHistoryItem): RuleHistoryDisplayVariable[] =>
  getHistoryDisplay(row)?.variables || []

export const formatDisplayVariable = (item: RuleHistoryDisplayVariable): string => {
  const unit = item.unit ? ` ${item.unit}` : ''
  return `${item.label} = ${formatNumber(item.value)}${unit}`
}

export const getExecutionSteps = (row: RuleHistoryItem): RuleHistoryDisplayStep[] =>
  getHistoryDisplay(row)?.execution_steps || []

export const formatExecutionStep = (step: RuleHistoryDisplayStep): string => {
  if (step.matched_label) {
    return `${step.label} (${step.matched_label})`
  }
  return step.label
}

export const getDisplayActions = (row: RuleHistoryItem): RuleHistoryDisplayAction[] =>
  getHistoryDisplay(row)?.actions || []
