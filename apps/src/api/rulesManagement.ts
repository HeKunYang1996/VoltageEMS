import Request from '@/utils/request'
import type { Rule, CreateRulePayload, UpdateRulePayload } from '@/types/ruleConfiguration'
import type { RuleChainPayload } from '@/types/ruleConfiguration'
import type { ModRuleSummary, PaginatedList, RuleHistoryItem } from '@/types/controlRule'

export interface ModRuleListQuery {
  page?: number
  page_size?: number
  /** Fuzzy name filter (case-insensitive) */
  name?: string
}

export interface ModRuleHistoryQuery {
  page?: number
  page_size?: number
  /** Start time filter: Unix timestamp in ms (inclusive) */
  start_time?: number
  /** End time filter: Unix timestamp in ms (inclusive) */
  end_time?: number
}

export const listModRules = async (params: ModRuleListQuery = {}) => {
  return await Request.get<PaginatedList<ModRuleSummary>>('/ruleApi/api/rules', params)
}

export const getModRuleHistory = async (
  id: string | number,
  params: ModRuleHistoryQuery = {},
) => {
  return await Request.get<PaginatedList<RuleHistoryItem>>(
    `/ruleApi/api/rules/${id}/history`,
    params,
  )
}

export const listRules = async (params: ModRuleListQuery = {}) => {
  return await Request.get<{ list: Rule[] }>('/ruleApi/api/rules', params)
}

export const getRuleDetail = async (id: string) => {
  return await Request.get<RuleChainPayload>(`/ruleApi/api/rules/${id}`)
}

export const createRule = async (payload: CreateRulePayload) => {
  return await Request.post<Rule>('/ruleApi/api/rules', payload)
}

export const updateRule = async (
  payload: RuleChainPayload | { name: string; description: string; id: string },
) => {
  return await Request.put<Rule>(`/ruleApi/api/rules/${payload.id}`, payload)
}

export const deleteRule = async (id: string) => {
  return await Request.delete(`/ruleApi/api/rules/${id}`)
}

export const enableRule = async (id: string) => {
  return await Request.post(`/ruleApi/api/rules/${id}/enable`)
}

export const disableRule = async (id: string) => {
  return await Request.post(`/ruleApi/api/rules/${id}/disable`)
}

export const submitRuleChain = async (payload: RuleChainPayload) => {
  return await Request.post('/ruleApi/api/rules', payload)
}
