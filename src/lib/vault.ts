// 敏感操作口令守卫：导出 / 备份 / 恢复前要求验证口令锁
import { hasVaultPassword, verifyVaultPassword } from "@/lib/commands";

/**
 * 校验口令后才允许继续敏感操作。
 * - 未开启口令锁：直接放行
 * - 已开启：弹窗要求输入口令，取消或错误时返回 false（调用方应中止操作）
 */
export async function requireVault(action: string): Promise<boolean> {
  const on = await hasVaultPassword();
  if (!on) return true;

  const pwd = window.prompt(`「${action}」属于敏感操作，请输入口令：`);
  if (pwd == null) return false; // 用户取消
  if (!(await verifyVaultPassword(pwd))) {
    window.alert("口令不正确，操作已取消");
    return false;
  }
  return true;
}
