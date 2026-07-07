import { ElMessage } from 'element-plus'
import {
  canAccessConfigTool,
  isValidRole,
  normalizeRoleName,
  type ValidRole,
} from '@/utils/rolePermission'
import type { UserInfo } from '@/types/user'

export interface RoleGuardOptions {
  configTool?: boolean
}

export function assertValidUserRole(
  user: UserInfo | null | undefined,
  options: RoleGuardOptions = {},
): user is UserInfo & { role: { name_en: ValidRole } } {
  const name = normalizeRoleName(user?.role?.name_en)

  if (!isValidRole(name)) {
    ElMessage.error('Invalid account role. Please contact your administrator.')
    return false
  }

  if (options.configTool && !canAccessConfigTool(name)) {
    ElMessage.error('The configuration tool is only available to administrators and engineers.')
    return false
  }

  return true
}
