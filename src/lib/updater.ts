// 应用内自动更新：检测 → 下载 → 安装（Windows 下安装后会自动重启应用）
// 更新源为 tauri.conf.json 里配置的 plugins.updater.endpoints
import { ref } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "uptodate"
  | "available"
  | "downloading"
  | "installing"
  | "error";

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  notes?: string;
  date?: string;
}

/** 开发模式不做检测：devUrl 下没有签名产物，检测只会报无意义错误 */
const DEV = import.meta.env.DEV;

export const phase = ref<UpdatePhase>("idle");
export const info = ref<UpdateInfo | null>(null);
export const errorText = ref("");
export const percent = ref<number | null>(null);
export const checkedAt = ref("");

/** 检测结果句柄：下载/安装需要复用它，不能只留版本号 */
let pending: Update | null = null;

async function releasePending() {
  if (pending) {
    try {
      await pending.close();
    } catch {
      /* 关闭失败无副作用，忽略 */
    }
    pending = null;
  }
}

/**
 * 检查更新。
 * @param silent true 时只写状态，不把 "已是最新" 之外的结果弹成错误提示
 */
export async function runCheck(): Promise<UpdateInfo | null> {
  if (DEV) {
    errorText.value = "开发模式不支持检测更新";
    phase.value = "error";
    return null;
  }
  phase.value = "checking";
  errorText.value = "";
  await releasePending();
  try {
    const u = await check();
    checkedAt.value = new Date().toLocaleString("zh-CN", { hour12: false });
    if (!u) {
      info.value = null;
      phase.value = "uptodate";
      return null;
    }
    pending = u;
    info.value = {
      version: u.version,
      currentVersion: u.currentVersion,
      notes: u.body,
      date: u.date,
    };
    phase.value = "available";
    return info.value;
  } catch (e) {
    errorText.value = String(e);
    phase.value = "error";
    return null;
  }
}

/** 下载并安装当前检测到的更新。Windows 安装完成后应用会退出并自动重启。 */
export async function runInstall(): Promise<void> {
  if (!pending) {
    errorText.value = "没有待安装的更新，请先检测";
    phase.value = "error";
    return;
  }
  percent.value = 0;
  let total = 0;
  let done = 0;
  try {
    await pending.downloadAndInstall((ev) => {
      if (ev.event === "Started") {
        total = ev.data.contentLength ?? 0;
        phase.value = "downloading";
        percent.value = 0;
      } else if (ev.event === "Progress") {
        done += ev.data.chunkLength;
        percent.value = total
          ? Math.min(99, Math.round((done / total) * 100))
          : null;
      } else {
        percent.value = 100;
        phase.value = "installing";
      }
    });
    // Windows 上走到这里说明安装器已启动，应用随即退出
  } catch (e) {
    errorText.value = String(e);
    phase.value = "error";
  }
}

/** 忽略本次更新提示 */
export async function dismissUpdate(): Promise<void> {
  await releasePending();
  info.value = null;
  phase.value = "idle";
}
