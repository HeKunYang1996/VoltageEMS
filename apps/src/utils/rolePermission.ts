export const VALID_ROLES = ['Admin', 'Engineer', 'Viewer'] as const
export type ValidRole = (typeof VALID_ROLES)[number]
export type PermissionLevel = 'admin' | 'engineer' | 'viewer'

export function normalizeRoleName(name: unknown): string {
  return String(name ?? '').trim()
}

export function isValidRole(name: unknown): name is ValidRole {
  return (VALID_ROLES as readonly string[]).includes(normalizeRoleName(name))
}

export function canAccessConfigTool(roleName: unknown): boolean {
  const r = normalizeRoleName(roleName)
  return r === 'Admin' || r === 'Engineer'
}

/** 用户管理（增删改用户、分配角色）仅管理员可用 */
export function canManageUsers(roleName: unknown): boolean {
  return normalizeRoleName(roleName) === 'Admin'
}

export function resolvePermissionAllowed(
  value: PermissionLevel | ValidRole[] | undefined,
  roleName: string,
): boolean {
  if (!value) return true

  if (typeof value === 'string') {
    const level = value.toLowerCase() as PermissionLevel
    switch (level) {
      case 'admin':
        return roleName === 'Admin'
      case 'engineer':
        return roleName === 'Admin' || roleName === 'Engineer'
      case 'viewer':
        return isValidRole(roleName)
      default:
        return roleName === value
    }
  }

  if (Array.isArray(value)) {
    return isValidRole(roleName) && value.includes(roleName as ValidRole)
  }

  return false
}
