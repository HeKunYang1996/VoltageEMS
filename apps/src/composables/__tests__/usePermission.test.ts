import { describe, expect, it } from 'vitest'
import {
  canAccessConfigTool,
  canManageUsers,
  isValidRole,
  resolvePermissionAllowed,
} from '@/utils/rolePermission'

describe('usePermission', () => {
  it('validates known roles only', () => {
    expect(isValidRole('Admin')).toBe(true)
    expect(isValidRole('Engineer')).toBe(true)
    expect(isValidRole('Viewer')).toBe(true)
    expect(isValidRole('admin')).toBe(false)
    expect(isValidRole('')).toBe(false)
  })

  it('resolves permission levels', () => {
    expect(resolvePermissionAllowed('admin', 'Admin')).toBe(true)
    expect(resolvePermissionAllowed('admin', 'Engineer')).toBe(false)
    expect(resolvePermissionAllowed('engineer', 'Engineer')).toBe(true)
    expect(resolvePermissionAllowed('engineer', 'Viewer')).toBe(false)
    expect(resolvePermissionAllowed('viewer', 'Viewer')).toBe(true)
  })

  it('allows config tool access for admin and engineer only', () => {
    expect(canAccessConfigTool('Admin')).toBe(true)
    expect(canAccessConfigTool('Engineer')).toBe(true)
    expect(canAccessConfigTool('Viewer')).toBe(false)
  })

  it('allows user management for admin only', () => {
    expect(canManageUsers('Admin')).toBe(true)
    expect(canManageUsers('Engineer')).toBe(false)
    expect(canManageUsers('Viewer')).toBe(false)
  })
})
