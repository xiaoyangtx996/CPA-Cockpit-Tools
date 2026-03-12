# CPA-Cockpit Tools

本项目是一个本地桌面工具（Tauri + Vue），用于**CPA 侧账号/Token 的批量测试**与**配置/账号数据导出**，方便把 CPA 侧沉淀的数据整理后，交给 **cockpit-tools** 进一步管理与切换。

相关项目：

- CLIProxyAPI（CPA）：https://github.com/router-for-me/CLIProxyAPI
- cockpit-tools：https://github.com/jlcodes99/cockpit-tools

## 为什么需要它

市面上 CPA（CLIProxyAPI）相关的 Codex 账号/Token 数据量很大，但它们通常以“散落的 JSON 文件”形式存在：

- 不方便快速判断哪些账号可用、哪些无额度、哪些已失效。
- 不方便导出成 cockpit-tools 更易消费/管理的汇总配置。

这个工具的目标是：**把“批量测试 + 汇总导出”做成一个离线一键化流程**。

## 功能

### 1) 测试账号（批量检测 Token 有效性）

- 扫描你选择的目录下的 Token JSON 文件
- 并发调用接口检测有效性/额度状态
- 按结果分类归档

输出：

- `<输入目录>_result/`
- 按分类目录归档（例如：成功、无额度、401失效、403禁止、其他错误）
- 生成 `test_results.json` 汇总明细

### 2) CPA 转 Cockpit（导出配置/汇总文件）

- 递归扫描输入目录下的 JSON
- 把识别到的 token 信息汇总成 cockpit-tools 可消费的统一 JSON 文件

输出：

- 临时输出目录：`<输入目录>_cpa_to_cockpit/`
- 导出文件：`cockpit_accounts.json`
- 可选择把 `cockpit_accounts.json` 复制保存到你指定的目录

## 使用方法

1. 启动软件
2. 选择“输入目录”
3. 选择功能：
   - 测试账号：点击“测试账号”卡片
   - CPA转Cockpit：点击“CPA转Cockpit”卡片
4. 点击“开始执行”
5. 执行完成后，根据日志提示找到输出目录/导出文件

## 注意事项

- 本工具为本地离线处理工具；测试功能会发起网络请求，请确保网络环境可访问目标接口。
- 账号/Token 属于敏感信息：
  - 建议只在本机使用
  - 不要把导出结果上传到不可信位置

## 构建与打包

- 安装依赖：`npm install`
- 前端构建：`npm run build`
- Tauri 打包：`npm run tauri build`

打包产物目录通常在：

- `src-tauri/target/release/bundle/`
