import { computed } from 'vue'
import { useUserStore } from '@/stores/user'
import {
  isValidRole,
  normalizeRoleName,
  canAccessConfigTool,
  canManageUsers,
  type ValidRole,
} from '@/utils/rolePermission'

export {
  VALID_ROLES,
  isValidRole,
  normalizeRoleName,
  canAccessConfigTool,
  canManageUsers,
  resolvePermissionAllowed,
  type ValidRole,
  type PermissionLevel,
} from '@/utils/rolePermission'

export function usePermission() {
  const userStore = useUserStore()

  const roleName = computed(() => normalizeRoleName(userStore.userInfo?.role?.name_en))
  const roleValid = computed(() => isValidRole(roleName.value))

  const isAdmin = computed(() => roleName.value === 'Admin')
  const isEngineer = computed(() => isAdmin.value || roleName.value === 'Engineer')
  const isViewer = computed(() => roleValid.value)
  const canWrite = computed(() => isEngineer.value)
  const canManageUsersRole = computed(() => canManageUsers(roleName.value))
  const canAccessConfigToolRole = computed(() => canAccessConfigTool(roleName.value))

  function hasRouteRole(allowed: ValidRole[] | undefined): boolean {
    if (!allowed?.length) return true
    if (!roleValid.value) return false
    return allowed.includes(roleName.value as ValidRole)
  }

  return {
    roleName,
    roleValid,
    isAdmin,
    isEngineer,
    isViewer,
    canWrite,
    canManageUsersRole,
    canAccessConfigToolRole,
    hasRouteRole,
    isValidRole,
  }
}
