<script setup lang="ts">
/**
 * DictationButton.vue - 语音听写按钮
 *
 * TASK-2.5 (2026-06-10) 语音听写（Whisper）
 *
 * 功能:
 *   1. Hold-to-talk：按住开始录音，松开转写
 *   2. 实时波形（Web Audio API + AnalyserNode）
 *   3. MediaRecorder 录音（webm/opus 优先）
 *   4. 转写结果通过 v-model:text 暴露给父组件
 *   5. 超时自动停止（默认 60s）
 *   6. 错误提示（无 mic 权限 / API 错误）
 *
 * 父组件用法:
 *   <DictationButton v-model:text="inputText" />
 */
import { ref, onUnmounted, computed } from 'vue'
import { showToast } from 'vant'

interface Props {
  /** 父组件控制的文本（v-model:text） */
  text: string
  /** 转写时是否追加到末尾（true）还是替换（false） */
  append?: boolean
  /** 最大录音时长（ms），默认 60s */
  maxDurationMs?: number
  /** Whisper API 配置 */
  apiBase?: string
  apiKey?: string
  language?: string
  /** 自定义 fetch 实现（用于测试） */
  customFetch?: (audio: Blob, mimeType: string) => Promise<{ text: string }>
}

const props = withDefaults(defineProps<Props>(), {
  append: true,
  maxDurationMs: 60_000,
  apiBase: '',
  apiKey: '',
  language: 'auto',
  customFetch: undefined,
})

const emit = defineEmits<{
  (e: 'update:text', val: string): void
  (e: 'transcribe-start'): void
  (e: 'transcribe-end', text: string): void
  (e: 'error', msg: string): void
}>()

const isRecording = ref(false)
const isTranscribing = ref(false)
const audioLevels = ref<number[]>([])
const recordingDurationMs = ref(0)
const lastError = ref('')

let mediaRecorder: MediaRecorder | null = null
let audioContext: AudioContext | null = null
let analyser: AnalyserNode | null = null
let stream: MediaStream | null = null
let audioChunks: Blob[] = []
let startTime = 0
let levelTimer: number | null = null
let durationTimer: number | null = null
let timeoutTimer: number | null = null
let cancelled = false

const durationText = computed(() => {
  const s = Math.floor(recordingDurationMs.value / 1000)
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
})

const recording = ref(false)

function clearTimers() {
  if (levelTimer) {
    window.clearInterval(levelTimer)
    levelTimer = null
  }
  if (durationTimer) {
    window.clearInterval(durationTimer)
    durationTimer = null
  }
  if (timeoutTimer) {
    window.clearTimeout(timeoutTimer)
    timeoutTimer = null
  }
}

function releaseStream() {
  if (stream) {
    stream.getTracks().forEach((t) => t.stop())
    stream = null
  }
  if (audioContext && audioContext.state !== 'closed') {
    audioContext.close().catch(() => {})
  }
  audioContext = null
  analyser = null
}

async function startRecording() {
  if (isRecording.value || isTranscribing.value) return
  lastError.value = ''
  cancelled = false
  audioLevels.value = []
  recordingDurationMs.value = 0

  try {
    stream = await navigator.mediaDevices.getUserMedia({ audio: true })
  } catch (e: any) {
    const msg = e?.name === 'NotAllowedError' ? '麦克风权限被拒绝' : `无法访问麦克风: ${e?.message || e}`
    lastError.value = msg
    emit('error', msg)
    showToast({ type: 'fail', message: msg })
    return
  }

  // AudioContext 用于实时波形
  try {
    audioContext = new (window.AudioContext || (window as any).webkitAudioContext)()
    const source = audioContext.createMediaStreamSource(stream)
    analyser = audioContext.createAnalyser()
    analyser.fftSize = 64
    source.connect(analyser)
  } catch {
    // AudioContext 失败不影响录音
  }

  // 选择最佳 MIME
  const mimeCandidates = [
    'audio/webm;codecs=opus',
    'audio/webm',
    'audio/ogg;codecs=opus',
    'audio/mp4',
  ]
  let mime = ''
  for (const m of mimeCandidates) {
    if (typeof MediaRecorder !== 'undefined' && MediaRecorder.isTypeSupported?.(m)) {
      mime = m
      break
    }
  }

  try {
    mediaRecorder = mime ? new MediaRecorder(stream, { mimeType: mime }) : new MediaRecorder(stream)
  } catch (e: any) {
    lastError.value = `MediaRecorder 初始化失败: ${e?.message || e}`
    emit('error', lastError.value)
    showToast({ type: 'fail', message: lastError.value })
    releaseStream()
    return
  }

  audioChunks = []
  mediaRecorder.ondataavailable = (e) => {
    if (e.data && e.data.size > 0) audioChunks.push(e.data)
  }
  mediaRecorder.onstop = handleRecordingStop

  startTime = Date.now()
  isRecording.value = true
  recording.value = true
  mediaRecorder.start(100)

  // 实时音量
  if (analyser) {
    const buf = new Uint8Array(analyser.frequencyBinCount)
    levelTimer = window.setInterval(() => {
      if (!analyser) return
      analyser.getByteFrequencyData(buf)
      let sum = 0
      for (let i = 0; i < buf.length; i++) sum += buf[i]
      const avg = sum / buf.length / 255
      audioLevels.value = [...audioLevels.value.slice(-19), avg]
    }, 80)
  } else {
    // 退化方案：固定动画
    levelTimer = window.setInterval(() => {
      const fake = 0.3 + Math.random() * 0.5
      audioLevels.value = [...audioLevels.value.slice(-19), fake]
    }, 80)
  }

  // 录音时长
  durationTimer = window.setInterval(() => {
    recordingDurationMs.value = Date.now() - startTime
  }, 100)

  // 超时保护
  timeoutTimer = window.setTimeout(() => {
    if (isRecording.value) stopRecording(false)
  }, props.maxDurationMs)
}

function handleRecordingStop() {
  if (cancelled) {
    cleanup()
    return
  }
  const blob = new Blob(audioChunks, { type: mediaRecorder?.mimeType || 'audio/webm' })
  cleanup()
  if (blob.size === 0) {
    const msg = '录音为空'
    lastError.value = msg
    emit('error', msg)
    return
  }
  transcribe(blob)
}

function cleanup() {
  clearTimers()
  releaseStream()
  if (mediaRecorder) {
    try { mediaRecorder.ondataavailable = null; mediaRecorder.onstop = null } catch {}
    mediaRecorder = null
  }
  isRecording.value = false
  recording.value = false
  audioLevels.value = []
}

async function transcribe(blob: Blob) {
  isTranscribing.value = true
  emit('transcribe-start')
  try {
    let text: string
    if (props.customFetch) {
      const r = await props.customFetch(blob, blob.type)
      text = r.text
    } else {
      const form = new FormData()
      const ext = blob.type.includes('mp4') ? 'mp4' : blob.type.includes('ogg') ? 'ogg' : 'webm'
      form.append('file', blob, `audio.${ext}`)
      form.append('mime_type', blob.type || 'audio/webm')
      form.append('filename', `audio.${ext}`)
      form.append('language', props.language)
      if (props.apiBase) form.append('api_base', props.apiBase)
      if (props.apiKey) form.append('api_key', props.apiKey)

      const resp = await fetch('/api/dictation/transcribe', {
        method: 'POST',
        body: form,
      })
      const data = await resp.json()
      if (!data?.ok) {
        throw new Error(data?.error || `HTTP ${resp.status}`)
      }
      text = data.text || ''
    }

    // 追加或替换
    if (props.append) {
      const sep = props.text && !props.text.endsWith(' ') ? ' ' : ''
      emit('update:text', props.text + sep + text)
    } else {
      emit('update:text', text)
    }
    emit('transcribe-end', text)
  } catch (e: any) {
    const msg = e?.message || '转写失败'
    lastError.value = msg
    emit('error', msg)
    showToast({ type: 'fail', message: `转写失败: ${msg}` })
  } finally {
    isTranscribing.value = false
  }
}

function stopRecording(cancel = false) {
  if (!isRecording.value || !mediaRecorder) {
    if (mediaRecorder && mediaRecorder.state !== 'inactive') {
      try { mediaRecorder.stop() } catch {}
    }
    return
  }
  cancelled = cancel
  if (mediaRecorder.state !== 'inactive') {
    try { mediaRecorder.stop() } catch {}
  }
}

function handleClick() {
  // 移动端 click 即触发；桌面端有 mousedown/up
  if (isRecording.value) {
    stopRecording(false)
  } else {
    startRecording()
  }
}

onUnmounted(() => {
  cancelled = true
  clearTimers()
  if (mediaRecorder && mediaRecorder.state !== 'inactive') {
    try { mediaRecorder.stop() } catch {}
  }
  releaseStream()
})
</script>

<template>
  <button
    type="button"
    class="dictation-btn"
    :class="{
      'is-recording': isRecording,
      'is-transcribing': isTranscribing,
    }"
    :title="isRecording ? '松开结束' : isTranscribing ? '转写中…' : '按住说话'"
    :disabled="isTranscribing"
    @mousedown.prevent="startRecording"
    @mouseup.prevent="stopRecording(false)"
    @mouseleave="stopRecording(true)"
    @touchstart.prevent="startRecording"
    @touchend.prevent="stopRecording(false)"
    @click="handleClick"
  >
    <span v-if="isTranscribing" class="dictation-btn__spinner" />
    <span v-else-if="isRecording" class="dictation-btn__indicator">
      <span class="dictation-btn__dot" />
    </span>
    <span v-else class="dictation-btn__icon">🎤</span>

    <span v-if="isRecording" class="dictation-btn__waveform">
      <span
        v-for="(lv, i) in audioLevels"
        :key="i"
        class="dictation-btn__bar"
        :style="{ height: `${Math.max(4, lv * 24)}px` }"
      />
      <span class="dictation-btn__time">{{ durationText }}</span>
    </span>
  </button>
</template>

<style scoped>
.dictation-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 32px;
  min-width: 32px;
  padding: 0 6px;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-primary, #e0e0e0);
  cursor: pointer;
  font-size: 16px;
  transition: all 0.15s;
}

.dictation-btn:hover:not(:disabled) {
  background: var(--bg-hover, #333);
}

.dictation-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.dictation-btn.is-recording {
  background: rgba(229, 57, 53, 0.15);
  border-color: var(--danger, #e53935);
  color: var(--danger, #e53935);
}

.dictation-btn.is-transcribing {
  background: rgba(66, 165, 245, 0.15);
  border-color: var(--accent, #42a5f5);
  color: var(--accent, #42a5f5);
}

.dictation-btn__icon {
  display: inline-block;
}

.dictation-btn__indicator {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.dictation-btn__dot {
  width: 8px;
  height: 8px;
  background: var(--danger, #e53935);
  border-radius: 50%;
  animation: dictation-pulse 1s infinite;
}

@keyframes dictation-pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.4; transform: scale(0.8); }
}

.dictation-btn__waveform {
  display: inline-flex;
  align-items: center;
  gap: 1px;
  margin-left: 4px;
  height: 24px;
}

.dictation-btn__bar {
  width: 2px;
  background: var(--danger, #e53935);
  border-radius: 1px;
  transition: height 0.05s;
}

.dictation-btn__time {
  font-size: 11px;
  margin-left: 4px;
  font-variant-numeric: tabular-nums;
}

.dictation-btn__spinner {
  width: 14px;
  height: 14px;
  border: 2px solid var(--border, #444);
  border-top-color: var(--accent, #42a5f5);
  border-radius: 50%;
  animation: dictation-spin 0.8s linear infinite;
}

@keyframes dictation-spin {
  to { transform: rotate(360deg); }
}
</style>
