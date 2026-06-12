<script setup lang="ts">
/**
 * CollaborationSettings — 设置中的"协作"分区
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 提供：
 *  - 登录/注册/登出（未登录时显示 LoginPanel 缩略版）
 *  - 工作区列表 + 切换 + 创建
 *  - 设备配对（输入配对码）
 *  - 跳转到协作主页
 */
import { ref, onMounted, computed } from 'vue'
import { showToast } from 'vant'
import { useCollaborationStore } from '@/stores/collaboration'
import { workspaceApi } from '@/api'
import type { Workspace as ApiWorkspace } from '@/api'

const store = useCollaborationStore()
const workspaces = ref<ApiWorkspace[]>([])
const loading = ref(false)

const username = ref('')
const password = ref('')
const registerDisplayName = ref('')
const registerEmail = ref('')
const pairCode = ref('')
const deviceName = ref(
  typeof navigator !== 'undefined' ? navigator.userAgent.slice(0, 60) : 'Unknown Device',
)

const activeTab = ref<'login' | 'register' | 'pair'>('login')

// 新建工作区弹窗
const showNewWsDialog = ref(false)
const newWsName = ref('')

async function loadWorkspaces() {
  if (!store.tokens?.access_token) return
  loading.value = true
  try {
    const r = await workspaceApi.list(store.tokens.access_token)
    workspaces.value = r.data
  } catch (e) {
    showToast({ type: 'fail', message: '加载工作区失败' })
  } finally {
    loading.value = false
  }
}

async function onLogin() {
  if (!username.value || !password.value) {
    showToast('请填写用户名和密码')
    return
  }
  try {
    await store.login(username.value, password.value)
    showToast({ type: 'success', message: '登录成功' })
    username.value = ''
    password.value = ''
    await loadWorkspaces()
    // 自动连工作区
    if (workspaces.value.length > 0) {
      await store.switchWorkspace(workspaces.value[0].id)
      await store.connect()
    }
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: err })
  }
}

async function onRegister() {
  if (!username.value || !password.value) {
    showToast('请填写用户名和密码')
    return
  }
  try {
    const r = await store.register({
      username: username.value,
      password: password.value,
      display_name: registerDisplayName.value || undefined,
      email: registerEmail.value || undefined,
    })
    showToast({ type: 'success', message: '注册成功' })
    username.value = ''
    password.value = ''
    registerDisplayName.value = ''
    registerEmail.value = ''
    await loadWorkspaces()
    if (r.workspace_id) {
      await store.connect()
    }
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: err })
  }
}

async function onPair() {
  if (!pairCode.value) {
    showToast('请输入配对码')
    return
  }
  try {
    const r = await store.pairDevice(pairCode.value, deviceName.value)
    if (r.paired) {
      showToast({ type: 'success', message: '配对成功' })
      pairCode.value = ''
    } else {
      showToast('配对码无效或已过期')
    }
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: err })
  }
}

async function onCreateWorkspace() {
  newWsName.value = ''
  showNewWsDialog.value = true
}

function confirmCreateWorkspace() {
  const name = newWsName.value.trim()
  if (!name) return
  showNewWsDialog.value = false
  void _doCreateWorkspace(name)
}

async function _doCreateWorkspace(name: string) {
  try {
    const ws = await store.createWorkspace(name, '')
    showToast({ type: 'success', message: '已创建' })
    await loadWorkspaces()
    await store.switchWorkspace(ws.id)
    await store.connect()
  } catch { /* ignore */ }
}

async function onSwitch(ws: ApiWorkspace) {
  await store.switchWorkspace(ws.id)
  await store.connect()
  showToast({ type: 'success', message: `已切换到 ${ws.name}` })
}

async function onLogout() {
  await store.logout()
  showToast('已登出')
  workspaces.value = []
}

onMounted(() => {
  if (store.isAuthenticated) loadWorkspaces()
})
</script>

<template>
  <div class="collab-settings">
    <!-- 已登录状态 -->
    <template v-if="store.isAuthenticated">
      <div class="user-card">
        <div class="avatar">{{ store.currentUser?.display_name?.slice(0, 1) || store.currentUser?.username?.slice(0, 1) || '?' }}</div>
        <div class="user-info">
          <div class="name">{{ store.currentUser?.display_name || store.currentUser?.username }}</div>
          <div class="email">{{ store.currentUser?.email || '@' + store.currentUser?.username }}</div>
        </div>
        <van-button size="small" plain @click="onLogout">登出</van-button>
      </div>

      <div class="connection-status">
        <span class="dot" :class="store.connectionStatus"></span>
        {{
          store.connectionStatus === 'connected' ? '已连接到工作区' :
          store.connectionStatus === 'connecting' ? '连接中…' :
          store.connectionStatus === 'offline' ? '离线（本地可用）' :
          '未连接'
        }}
        <van-button
          v-if="store.connectionStatus === 'disconnected' || store.connectionStatus === 'offline'"
          size="mini"
          type="primary"
          @click="store.connect()"
          style="margin-left: 12px;"
        >
          重连
        </van-button>
      </div>

      <h3 class="section-title">我的工作区</h3>
      <van-loading v-if="loading" size="16" />
      <van-cell-group v-else>
        <van-cell
          v-for="ws in workspaces"
          :key="ws.id"
          :title="ws.name"
          :label="`${ws.role} · ${new Date(ws.created_at).toLocaleDateString()}`"
          is-link
          @click="onSwitch(ws)"
        >
          <template #icon>
            <van-icon
              :name="ws.id === store.activeWorkspaceId ? 'success' : 'cluster-o'"
              :style="{ color: ws.id === store.activeWorkspaceId ? '#22c55e' : '#7c3aed', marginRight: '8px' }"
            />
          </template>
        </van-cell>
      </van-cell-group>
      <van-button block plain icon="plus" @click="onCreateWorkspace">新建工作区</van-button>

      <div class="actions-row">
        <van-button type="primary" block @click="$router.push('/collaboration')">
          打开协作主页
        </van-button>
      </div>
    </template>

    <!-- 未登录状态 -->
    <template v-else>
      <p class="hint">登录或注册账号，即可与他人实时协作。Y.js 支持断网编辑，恢复网络后自动合并。</p>
      <van-tabs v-model:active="activeTab">
        <van-tab title="登录" name="login">
          <div class="form">
            <van-field v-model="username" label="用户名" required />
            <van-field v-model="password" type="password" label="密码" required />
            <van-button type="primary" block @click="onLogin">登录</van-button>
          </div>
        </van-tab>
        <van-tab title="注册" name="register">
          <div class="form">
            <van-field v-model="username" label="用户名" required />
            <van-field v-model="registerDisplayName" label="昵称" />
            <van-field v-model="registerEmail" label="邮箱" type="email" />
            <van-field v-model="password" type="password" label="密码" required />
            <van-button type="primary" block @click="onRegister">注册</van-button>
          </div>
        </van-tab>
        <van-tab title="设备配对" name="pair">
          <div class="form">
            <van-field v-model="pairCode" label="配对码" placeholder="6 位字符" maxlength="6" />
            <van-field v-model="deviceName" label="设备名" />
            <van-button type="primary" block @click="onPair">配对</van-button>
          </div>
        </van-tab>
      </van-tabs>
    </template>

    <div v-if="store.error" class="error-msg">{{ store.error }}</div>

    <!-- 新建工作区弹窗 -->
    <van-dialog
      v-model:show="showNewWsDialog"
      title="新建工作区"
      show-cancel-button
      @confirm="confirmCreateWorkspace"
    >
      <div style="padding: 12px;">
        <van-field
          v-model="newWsName"
          label="工作区名"
          placeholder="如：我的开源项目"
          required
          clearable
          @keydown.enter="confirmCreateWorkspace"
        />
      </div>
    </van-dialog>
  </div>
</template>

<style scoped>
.collab-settings {
  padding: 0;
}
.user-card {
  display: flex; align-items: center; gap: 12px;
  padding: 12px;
  background: var(--yj-card-bg, #1a1a1a);
  border-radius: 8px;
  margin-bottom: 16px;
}
.avatar {
  width: 40px; height: 40px; border-radius: 50%;
  background: #7c3aed; color: white;
  display: flex; align-items: center; justify-content: center;
  font-weight: 600; font-size: 16px;
}
.user-info { flex: 1; }
.user-info .name { font-size: 14px; font-weight: 500; }
.user-info .email { font-size: 12px; color: var(--yj-text-secondary, #888); }
.connection-status {
  display: flex; align-items: center; gap: 8px;
  padding: 8px 12px; font-size: 13px;
  background: rgba(255,255,255,0.04);
  border-radius: 6px; margin-bottom: 16px;
}
.dot {
  width: 8px; height: 8px; border-radius: 50%;
  background: #555;
}
.dot.connected { background: #22c55e; }
.dot.connecting { background: #eab308; }
.dot.offline { background: #6b7280; }
.dot.disconnected { background: #ef4444; }
.section-title {
  font-size: 13px; font-weight: 600;
  margin: 16px 0 8px;
  color: var(--yj-text-secondary, #aaa);
}
.actions-row { margin-top: 24px; }
.form { padding: 12px 0; }
.form .van-button { margin-top: 16px; }
.hint {
  font-size: 13px; color: var(--yj-text-secondary, #aaa);
  padding: 8px 0 16px; line-height: 1.6;
}
.error-msg {
  margin-top: 12px; padding: 8px 12px;
  background: rgba(239, 68, 68, 0.1);
  border-left: 3px solid #ef4444;
  color: #fca5a5; font-size: 13px;
  border-radius: 4px;
}
</style>
