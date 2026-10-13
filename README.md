# 报销台账

一款完全本地运行的桌面记账工具，专注「报销」这件事：不只是记流水，还能追踪每一笔钱**报没报、到账没到账**。

数据只存本机，不接入任何服务器，不含遥测与广告。

> 技术栈：Tauri 2（Rust）+ Vue 3 + TypeScript + Tailwind CSS + SQLite

---

## 功能

| 模块 | 说明 |
|---|---|
| 记账 | 支出 / 收入；支出 15 类、收入 7 类常用分类可点选，也支持自定义；金额、往来方、支付方式、备注 |
| **报销状态** | 每笔支出可标记「待报销 / 已提交 / 已回款」，看板实时统计在途金额与笔数 |
| 凭证 | 附加发票、收据图片，本地保存，随记录一起管理 |
| 预算 | 账套总预算 + **分类级预算**（差旅、招待…分别设额度），≥85% 琥珀预警、超支标红 |
| 导入导出 | Excel 模板导入（容错日期格式 / 千分位 / 中文支付方式）、Excel 与 CSV 导出、打印或另存 PDF |
| 统计看板 | 支出 / 收入 / 待报销 / 净额 / 笔数五张 KPI，分类占比、月度趋势、支付方式构成 |
| 账套 | 多账套分开记（工作 / 个人），各自独立预算与统计 |
| 安全 | 口令锁（Argon2id）保护导出与备份恢复；本地数据库备份与恢复 |

## 下载使用

到 Releases 下载 `报销台账_x.x.x_x64-setup.exe` 安装即可，无需管理员权限。

免安装的便携版也在 Releases 中（解压后需与 `WebView2Loader.dll` 放在同一目录）。

## 从源码构建

前置：Node 22+、Rust（Windows 建议 `stable-x86_64-pc-windows-gnu` + MinGW-w64）。

```bash
npm install
npm run tauri dev      # 开发模式
npm run tauri build    # 构建 + 打包
```

> Windows 上若项目路径含中文，MinGW 的 `windres` 会失败。仓库内 `scripts/build-installer.sh`
> 已把源码镜像到 ASCII 路径后再构建，直接运行即可产出安装包：
> ```bash
> bash scripts/build-installer.sh
> ```

## 数据存放位置

| 系统 | 路径 |
|---|---|
| Windows | `%APPDATA%\com.expenseledger.app\ledger.db`，凭证在同级 `receipts\` |

卸载不会自动删除这些数据；需要彻底清理请手动删除上述目录，或先用「设置 → 备份数据库」留一份。

## 目录结构

```
src/                 Vue 3 前端（页面、组件、Pinia store、Tauri 命令封装）
src-tauri/           Rust 后端（SQLite 读写、导入导出、口令哈希、备份恢复）
  src/commands.rs    全部 Tauri 命令
  src/db.rs          建表与老库平滑迁移（ALTER TABLE 补列）
  src/crypto.rs      Argon2id + AES-256-GCM（敏感字段加密预留）
brand/               Logo 与图标源文件（SVG + 生成脚本）
scripts/             打包脚本
```

## 开源协议

[MIT](./LICENSE)
