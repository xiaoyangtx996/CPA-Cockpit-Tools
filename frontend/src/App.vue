<template>
  <div class="app">
    <!-- 顶部标题栏 -->
    <header class="header">
      <div class="logo">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <path d="M12 2L2 7L12 12L22 7L12 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M2 17L12 22L22 17" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M2 12L12 17L22 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span>CPA-Cockpit Tools</span>
      </div>
      <div class="version">v1.0.0</div>
    </header>

    <!-- 功能选择卡片 -->
    <div class="cards">
      <div 
        class="card" 
        :class="{ active: mode === 'test' }"
        @click="mode = 'test'"
      >
        <div class="card-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none">
            <circle cx="12" cy="12" r="10" stroke="currentColor" stroke-width="2"/>
            <path d="M12 6V12L16 14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="card-title">测试账号</div>
        <div class="card-desc">批量测试Token有效性</div>
      </div>

      <div 
        class="card" 
        :class="{ active: mode === 'merge' }"
        @click="mode = 'merge'"
      >
        <div class="card-icon">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none">
            <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M14 2V8H20" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M12 18V12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <path d="M9 15L12 12L15 15" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </div>
        <div class="card-title">CPA转Cockpit</div>
        <div class="card-desc">转换并导出配置文件</div>
      </div>
    </div>

    <!-- 输入选择 -->
    <div class="input-section">
      <div class="section-title">输入目录</div>
      <div class="path-input">
        <input 
          type="text" 
          v-model="inputPath" 
          placeholder="选择包含Token文件的目录..."
          readonly
        />
        <button class="btn-browse" @click="selectInput">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
            <path d="M3 7V17C3 18.1046 3.89543 19 5 19H19C20.1046 19 21 18.1046 21 17V9C21 7.89543 20.1046 7 19 7H13L11 5H5C3.89543 5 3 5.89543 3 7Z" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          浏览
        </button>
      </div>
    </div>

    <!-- 操作按钮 -->
    <div class="actions">
      <button 
        class="btn-primary" 
        :disabled="running || !inputPath"
        @click="runScript"
      >
        <svg v-if="!running" width="20" height="20" viewBox="0 0 24 24" fill="none">
          <polygon points="5 3 19 12 5 21 5 3" fill="currentColor"/>
        </svg>
        <div v-else class="spinner"></div>
        {{ running ? '执行中...' : '开始执行' }}
      </button>
      <button 
        v-if="running"
        class="btn-danger"
        @click="stopScript"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
          <rect x="6" y="6" width="12" height="12" fill="currentColor"/>
        </svg>
        停止
      </button>
    </div>

    <!-- 日志展示 -->
    <div class="log-section">
      <div class="section-header">
        <div class="section-title">执行日志</div>
        <button class="btn-clear" @click="logs = []">清空</button>
      </div>
      <div class="log-container" ref="logContainer">
        <div 
          v-for="(log, index) in logs" 
          :key="index"
          class="log-line"
          :class="log.type"
        >
          <span class="log-time">{{ log.time }}</span>
          <span class="log-text">{{ log.text }}</span>
        </div>
        <div v-if="logs.length === 0" class="log-empty">
          暂无日志
        </div>
      </div>
    </div>

    <!-- 状态栏 -->
    <div class="status-bar">
      <div class="status-item">
        <span class="status-dot" :class="running ? 'running' : 'idle'"></span>
        {{ running ? '执行中' : '就绪' }}
      </div>
      <div class="status-item" v-if="inputPath">
        输入: {{ inputPath.split(/[\\/]/).pop() }}
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, nextTick, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

const mode = ref('test')
const inputPath = ref('')
const running = ref(false)
const logs = ref([])
const logContainer = ref(null)

// 添加日志
function addLog(text, type = 'info') {
  const now = new Date()
  const time = now.toLocaleTimeString('zh-CN', { hour12: false })
  logs.value.push({ time, text, type })
  
  // 滚动到底部
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight
    }
  })
}

// 选择输入目录
async function selectInput() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择输入目录'
    })
    if (selected) {
      inputPath.value = selected
      addLog(`已选择目录: ${selected}`)
    }
  } catch (e) {
    addLog(`选择目录失败: ${e}`, 'error')
  }
}

// 执行脚本
async function runScript() {
  if (!inputPath.value) {
    addLog('请先选择输入目录', 'error')
    return
  }

  running.value = true
  logs.value = []
  addLog(`开始执行${mode.value === 'test' ? '测试' : '转换'}...`)

  try {
    if (mode.value === 'test') {
      // 测试模式
      const resultDir = await invoke('test_tokens', {
        inputDir: inputPath.value,
        concurrency: 50
      })
      addLog(`结果保存在: ${resultDir}`, 'success')
    } else {
      // 转换模式 - 先输出到临时目录，再让用户选择保存目录
      const tempOutput = await invoke('merge_tokens', {
        inputDir: inputPath.value,
        outputDir: inputPath.value + '_cpa_to_cockpit'
      })
      addLog(`转换完成(临时输出): ${tempOutput}`, 'success')

      const saveDir = await open({
        directory: true,
        multiple: false,
        title: '选择保存目录'
      })

      if (saveDir) {
        const srcFile = tempOutput + '\\cockpit_accounts.json'
        const savedPath = await invoke('copy_file_to_dir', {
          srcFile,
          dstDir: saveDir,
          dstFilename: 'cockpit_accounts.json'
        })
        addLog(`已保存到: ${savedPath}`, 'success')
      } else {
        addLog('未选择保存目录，结果保留在临时输出目录', 'warn')
      }
    }
  } catch (e) {
    addLog(`执行失败: ${e}`, 'error')
  } finally {
    running.value = false
  }
}

// 打开结果目录
async function openResult(path) {
  try {
    await invoke('open_folder', { path })
  } catch (e) {
    addLog(`打开目录失败: ${e}`, 'error')
  }
}

// 停止脚本
async function stopScript() {
  try {
    await invoke('stop_script')
    addLog('已停止执行', 'warn')
  } catch (e) {
    addLog(`停止失败: ${e}`, 'error')
  }
  running.value = false
}

// 监听后端日志事件
import { listen } from '@tauri-apps/api/event'
listen('script-log', (event) => {
  addLog(event.payload.text, event.payload.type || 'info')
})
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

:root {
  --primary: #6366f1;
  --primary-hover: #4f46e5;
  --success: #10b981;
  --warning: #f59e0b;
  --danger: #ef4444;
  --bg: #0f172a;
  --bg-card: #1e293b;
  --bg-input: #334155;
  --text: #f1f5f9;
  --text-muted: #94a3b8;
  --border: #334155;
  --radius: 12px;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: var(--bg);
  color: var(--text);
  overflow: hidden;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  padding: 16px;
  gap: 16px;
}

/* Header */
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border);
}

.logo {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 18px;
  font-weight: 600;
  color: var(--primary);
}

.version {
  font-size: 12px;
  color: var(--text-muted);
}

/* Cards */
.cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.card {
  background: var(--bg-card);
  border: 2px solid var(--border);
  border-radius: var(--radius);
  padding: 20px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.card:hover {
  border-color: var(--primary);
  transform: translateY(-2px);
}

.card.active {
  border-color: var(--primary);
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.1), transparent);
}

.card-icon {
  color: var(--primary);
  margin-bottom: 12px;
}

.card-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 4px;
}

.card-desc {
  font-size: 13px;
  color: var(--text-muted);
}

/* Input Section */
.input-section {
  background: var(--bg-card);
  border-radius: var(--radius);
  padding: 16px;
}

.section-title {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.path-input {
  display: flex;
  gap: 10px;
}

.path-input input {
  flex: 1;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
  color: var(--text);
  font-size: 14px;
}

.path-input input:focus {
  outline: none;
  border-color: var(--primary);
}

.btn-browse {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 16px;
  color: var(--text);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-browse:hover {
  background: var(--primary);
  border-color: var(--primary);
}

/* Actions */
.actions {
  display: flex;
  gap: 10px;
}

.btn-primary {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: var(--primary);
  border: none;
  border-radius: var(--radius);
  padding: 14px;
  color: white;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

.btn-primary:hover:not(:disabled) {
  background: var(--primary-hover);
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-danger {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: var(--danger);
  border: none;
  border-radius: var(--radius);
  padding: 14px 24px;
  color: white;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
}

/* Log Section */
.log-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border-radius: var(--radius);
  overflow: hidden;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.btn-clear {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 10px;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
}

.log-container {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 13px;
  line-height: 1.6;
}

.log-line {
  display: flex;
  gap: 10px;
}

.log-time {
  color: var(--text-muted);
  flex-shrink: 0;
}

.log-text {
  word-break: break-all;
}

.log-line.success .log-text { color: var(--success); }
.log-line.error .log-text { color: var(--danger); }
.log-line.warn .log-text { color: var(--warning); }

.log-empty {
  color: var(--text-muted);
  text-align: center;
  padding: 40px;
}

/* Status Bar */
.status-bar {
  display: flex;
  gap: 20px;
  padding: 10px 0;
  border-top: 1px solid var(--border);
  font-size: 12px;
  color: var(--text-muted);
}

.status-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
}

.status-dot.running {
  background: var(--success);
  animation: pulse 1s infinite;
}

.status-dot.idle {
  background: var(--text-muted);
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

/* Spinner */
.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid rgba(255,255,255,0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Scrollbar */
::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 3px;
}
</style>
