<script setup lang="ts">
/**
 * LoginPanel — 登录/注册/配对 三合一入口
 *
 * 2026-06-09 TASK-4.4 / D5 引入
 *
 * 当用户未登录时（store.isAuthenticated = false）显示。
 * 登录/注册成功后会自动连工作区。
 */
import { ref } from 'vue'
import { showToast } from 'vant'
import { useCollaborationStore } from '@/stores/collaboration'

const store = useCollaborationStore()

type Mode = 'login' | 'register' | 'pair'
const mode = ref<Mode>('login')

const username = ref('')
const password = ref('')
const displayName = ref('')
const email = ref('')
const pairCode = ref('')
const deviceName = ref(
  typeof navigator !== 'undefined' ? navigator.userAgent.slice(0, 60) : 'Unknown Device',
)
const loading = ref(false)

async function onSubmit() {
  if (loading.value) return
  loading.value = true
  try {
    if (mode.value === 'login') {
      if (!username.value || !password.value) {
        showToast('请填写用户名和密码')
        return
      }
      await store.login(username.value, password.value)
      showToast({ type: 'success', message: '登录成功' })
    } else if (mode.value === 'register') {
      if (!username.value || !password.value) {
        showToast('请填写用户名和密码')
        return
      }
      await store.register({
        username: username.value,
        password: password.value,
        display_name: displayName.value || undefined,
        email: email.value || undefined,
      })
      showToast({ type: 'success', message: '注册成功' })
    } else if (mode.value === 'pair') {
      if (!pairCode.value) {
        showToast('请输入配对码')
        return
      }
      const r = await store.pairDevice(pairCode.value, deviceName.value)
      if (r.paired) {
        showToast({ type: 'success', message: '配对成功' })
      } else {
        showToast('配对码无效或已过期')
      }
    }
    // 登录/注册成功后自动连工作区
    await store.connect()
  } catch (e) {
    const err = e instanceof Error ? e.message : String(e)
    showToast({ type: 'fail', message: err })
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="login-panel">
    <div class="title">加入协作工作区</div>
    <van-tabs v-model:active="mode" sticky>
      <van-tab title="登录" name="login" />
      <van-tab title="注册" name="register" />
      <van-tab title="设备配对" name="pair" />
    </van-tabs>

    <div class="form-body">
      <template v-if="mode === 'login' || mode === 'register'">
        <van-field
          v-model="username"
          label="用户名"
          placeholder="3-32 位"
          autocomplete="username"
          required
        />
        <van-field
          v-if="mode === 'register'"
          v-model="displayName"
          label="昵称"
          placeholder="显示在协作中的名字"
        />
        <van-field
          v-if="mode === 'register'"
          v-model="email"
          label="邮箱"
          placeholder="可选"
          type="email"
        />
        <van-field
          v-model="password"
          label="密码"
          type="password"
          placeholder="至少 6 位"
          required
        />
      </template>

      <template v-else>
        <van-field
          v-model="pairCode"
          label="配对码"
          placeholder="6 位字符"
          maxlength="6"
        />
        <van-field
          v-model="deviceName"
          label="设备名"
          placeholder="如 iPhone 15 / MacBook"
        />
      </template>

      <div class="submit-area">
        <van-button
          type="primary"
          block
          :loading="loading"
          @click="onSubmit"
        >
          {{
            mode === 'login' ? '登录' :
            mode === 'register' ? '注册并创建工作区' :
            '配对'
          }}
        </van-button>
      </div>

      <div v-if="store.error" class="error-msg">{{ store.error }}</div>
    </div>
  </div>
</template>

<style scoped>
.login-panel {
  max-width: 480px;
  margin: 40px auto;
  padding: 24px;
  border-radius: 12px;
  background: var(--yj-card-bg, #1a1a1a);
  border: 1px solid var(--yj-border, #2a2a2a);
}
.title {
  font-size: 18px;
  font-weight: 600;
  text-align: center;
  margin-bottom: 16px;
  color: var(--yj-text, #eee);
}
.form-body { padding: 12px 0; }
.submit-area { margin-top: 24px; }
.error-msg {
  margin-top: 12px;
  padding: 8px 12px;
  background: rgba(239, 68, 68, 0.1);
  border-left: 3px solid #ef4444;
  color: #fca5a5;
  font-size: 13px;
  border-radius: 4px;
}
</style>
