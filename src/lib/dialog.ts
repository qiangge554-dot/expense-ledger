// 文件对话框封装（基于 @tauri-apps/plugin-dialog）
import { open, save } from "@tauri-apps/plugin-dialog";

const IMG_FILTER = [
  { name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif"] },
];

/** 选择一张或多张图片，返回绝对路径数组（取消则返回空数组） */
export async function pickImages(multiple = true): Promise<string[]> {
  const res = await open({ multiple, filters: IMG_FILTER });
  if (!res) return [];
  return Array.isArray(res) ? res : [res];
}

/** 选择任意文件（用于恢复备份），返回路径或 null */
export async function pickAnyFile(): Promise<string | null> {
  const res = await open({
    filters: [{ name: "数据库备份", extensions: ["db", "sqlite", "bak"] }],
  });
  return typeof res === "string" ? res : null;
}

/** 另存为对话框，返回目标路径或 null */
export async function saveAs(ext: string, defaultName: string): Promise<string | null> {
  const res = await save({
    defaultPath: defaultName,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });
  return typeof res === "string" ? res : null;
}

/** 选择一个 xlsx 导入文件，返回路径或 null */
export async function pickExcelFile(): Promise<string | null> {
  const res = await open({
    filters: [{ name: "Excel 工作簿", extensions: ["xlsx"] }],
  });
  return typeof res === "string" ? res : null;
}
