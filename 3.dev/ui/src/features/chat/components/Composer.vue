<script setup lang="ts">
/**
 * Composer.vue - 消息输入器（含图片附件 + 听写 + 拖拽 + 粘贴）
 *
 * TASK-2.3 (2026-06-10) 抽取与强化
 *
 * 功能:
 *   1. 多行文本输入（autosize）
 *   2. 图片附件（点击 / 拖拽 / Ctrl+V 粘贴，3 通道）
 *   3. 缩略图预览 + 单张删除
 *   4. 语音听写按钮（hold-to-talk，由父组件控制状态）
 *   5. 发送 / 停止切换
 *   6. 多供应商 vision 协议（暴露 dataUrl 列表给父组件）
 */
import { ref, computed } from 'vue'
import { showToast } from 'vant'
import { useDevice } from '@/composables/useDevice'

export interface AttachedImage {
  id: string
  dataUrl: string
  name: string
  size: number
}

interface Props {
  modelValue: string
  /** 是否正在 streaming（显示停止按钮替代发送） */
  isStreaming?: boolean
  /** 父组件控制的录音状态（来自 DictationButton） */
  isRecording?: boolean
  /** 录音实时音量（0-1）列表 */
  audioLevels?: number[]
  /** 录音时长（ms） */
  recordingDurationMs?: number
  /** 当前激活的提供商类型（用于多模态提示） */
  providerType?: 'anthropic' | 'openai' | 'ollama' | 'zhipu' | 'openrouter' | 'unknown'
  /** 是否禁用整个输入框 */
  disabled?: boolean
  /** 最大图片数量 */
  maxImages?: number
  /** 最大图片大小（字节） */
  maxImageSize?: number
  /** 父组件注入的 attachedImages（v-model） */
  attachedImages: AttachedImage[]
}

const props = withDefaults(defineProps<Props>(), {
  isStreaming: false,
  isRecording: false,
  audioLevels: () => [],
  recordingDurationMs: 0,
  providerType: 'unknown',
  disabled: false,
  maxImages: 4,
  maxImageSize: 5 * 1024 * 1024,
})

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'update:attachedImages', val: AttachedImage[]): void
  (e: 'send'): void
  (e: 'stop'): void
  (e: 'start-recording'): void
  (e: 'stop-recording', cancel: boolean): void
  (e: 'autocomplete', payload: { text: string; cursor: number }): void
  (e: 'keydown', ev: KeyboardEvent): void
}>()

const { isMobile } = useDevice()
const imageInputRef = ref<HTMLInputElement | null>(null)
const isDragOver = ref(false)

const text = computed({
  get: () => props.modelValue,
  set: (v: string) => emit('update:modelValue', v),
})

const canSend = computed(() => props.modelValue.trim().length > 0 || props.attachedImages.length > 0)
const canPickMore = computed(() => props.attachedImages.length < props.maxImages)

const supportsVision = computed(() => {
  // 2026-06-10 TASK-2.3：vision 兼容性判断
  return ['anthropic', 'openai', 'openrouter'].includes(props.providerType)
})

const ollamaNoVision = computed(() => props.providerType === 'ollama')

function pickImages() {
  if (!canPickMore.value) {
    showToast(`最多 ${props.maxImages} 张图片`)
    return
  }
  imageInputRef.value?.click()
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = () => reject(reader.error || new Error('FileReader error'))
    reader.readAsDataURL(file)
  })
}

async function addImageFile(file: File) {
  if (!file.type.startsWith('image/')) {
    showToast(`跳过非图片文件: ${file.name}`)
    return
  }
  if (file.size > props.maxImageSize) {
    showToast(`图片过大 (${(file.size / 1024 / 1024).toFixed(1)}MB > ${(props.maxImageSize / 1024 / 1024).toFixed(0)}MB): ${file.name}`)
    return
  }
  try {
    const dataUrl = await readFileAsDataUrl(file)
    const newImage: AttachedImage = {
      id: `img-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      dataUrl,
      name: file.name,
      size: file.size,
    }
    emit('update:attachedImages', [...props.attachedImages, newImage])
  } catch (e: any) {
    showToast(`读取失败: ${e.message}`)
  }
}

async function handleImageFiles(files: FileList | File[]) {
  const list = Array.from(files)
  for (const file of list) {
    if (props.attachedImages.length >= props.maxImages) {
      showToast(`最多 ${props.maxImages} 张图片`)
      break
    }
    await addImageFile(file)
  }
}

function onImageInputChange(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    handleImageFiles(target.files)
    target.value = ''
  }
}

function removeImage(id: string) {
  emit(
    'update:attachedImages',
    props.attachedImages.filter((i) => i.id !== id),
  )
}

function handleImagePaste(e: ClipboardEvent) {
  if (!e.clipboardData) return
  const items = e.clipboardData.items
  const files: File[] = []
  for (let i = 0; i < items.length; i++) {
    const it = items[i]
    if (it.kind === 'file' && it.type.startsWith('image/')) {
      const f = it.getAsFile()
      if (f) files.push(f)
    }
  }
  if (files.length > 0) {
    e.preventDefault()
    handleImageFiles(files)
  }
}

function handleImageDrop(e: DragEvent) {
  e.preventDefault()
  isDragOver.value = false
  if (!e.dataTransfer) return
  const files: File[] = []
  for (let i = 0; i < e.dataTransfer.files.length; i++) {
    const f = e.dataTransfer.files[i]
    if (f.type.startsWith('image/')) files.push(f)
  }
  if (files.length > 0) {
    handleImageFiles(files)
  }
}

function handleSend() {
  if (ollamaNoVision.value && props.attachedImages.length > 0) {
    showToast('当前 Ollama 模型不支持 vision，请切换到 Claude/GPT 等 vision 模型')
    return
  }
  if (!canSend.value || props.disabled) return
  emit('send')
}

function handleStop() {
  emit('stop')
}

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  emit('autocomplete', { text: target.value, cursor: target.selectionStart ?? 0 })
}

function onKeydown(e: KeyboardEvent) {
  emit('keydown', e)
}

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000)
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
}
</script>

<template>
  <div
    class="composer"
    :class="{ 'is-mobile': isMobile, 'is-drag-over': isDragOver }"
    @drop="handleImageDrop"
    @dragover.prevent="isDragOver = true"
    @dragleave="isDragOver = false"
  >
    <!-- 缩略图预览 -->
    <div v-if="attachedImages.length > 0" class="composer__thumbs">
      <div v-for="img in attachedImages" :key="img.id" class="composer__thumb">
        <img :src="img.dataUrl" :alt="img.name" />
        <button class="composer__thumb-remove" type="button" @click="removeImage(img.id)">×</button>
        <span class="composer__thumb-name">{{ img.name }}</span>
      </div>
    </div>

    <!-- 拖拽覆盖层 -->
    <div v-if="isDragOver" class="composer__dropzone-hint">
      释放以添加图片
    </div>

    <!-- 工具栏行 -->
    <div class="composer__row">
      <button
        v-if="isMobile"
        type="button"
        class="composer__icon-btn"
        title="会话"
        @click="$emit('open-sessions')"
      >
        💬
      </button>
      <button
        type="button"
        class="composer__icon-btn"
        :class="{ disabled: !canPickMore }"
        title="添加图片"
        :disabled="!canPickMore"
        @click="pickImages"
      >
        🖼️
      </button>
      <input
        ref="imageInputRef"
        type="file"
        accept="image/*"
        multiple
        style="display: none"
        @change="onImageInputChange"
      />

      <!-- 录音状态/按钮 -->
      <div v-if="isRecording" class="composer__recording">
        <span class="composer__recording-dot" />
        <div class="composer__waveform">
          <span
            v-for="(lv, i) in audioLevels"
            :key="i"
            class="composer__waveform-bar"
            :style="{ height: `${Math.max(6, lv * 24)}px` }"
          />
        </div>
        <span class="composer__recording-time">{{ formatDuration(recordingDurationMs) }}</span>
      </div>
      <button
        v-else
        type="button"
        class="composer__icon-btn"
        :disabled="disabled"
        title="按住说话"
        @mousedown.prevent="$emit('start-recording')"
        @mouseup.prevent="$emit('stop-recording', false)"
        @mouseleave="$emit('stop-recording', true)"
        @touchstart.prevent="$emit('start-recording')"
        @touchend.prevent="$emit('stop-recording', false)"
      >
        🎤
      </button>

      <textarea
        v-model="text"
        :rows="1"
        class="composer__textarea"
        :placeholder="supportsVision
          ? '输入消息...（支持 / 命令、@ 文件、Ctrl+V 图片）'
          : '输入消息...（当前模型不支持 vision 图片）'"
        :disabled="disabled"
        @input="onInput"
        @keydown="onKeydown"
        @paste="handleImagePaste"
      />

      <button
        v-if="isStreaming"
        type="button"
        class="composer__send composer__send--stop"
        title="停止"
        @click="handleStop"
      >
        ⏹
      </button>
      <button
        v-else
        type="button"
        class="composer__send"
        :class="{ 'is-disabled': !canSend || disabled }"
        :disabled="!canSend || disabled"
        title="发送"
        @click="handleSend"
      >
        ▶
      </button>
    </div>

    <!-- vision 兼容性提示 -->
    <div v-if="ollamaNoVision && attachedImages.length > 0" class="composer__vision-warn">
      ⚠️ 当前 Ollama 模型不支持 vision，将以纯文本发送
    </div>
  </div>
</template>

<style scoped>
.composer {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px 8px;
  background: var(--bg-secondary, #1e1e1e);
  border-top: 1px solid var(--border, #333);
  transition: background 0.2s;
}

.composer.is-drag-over {
  background: var(--accent-bg, rgba(66, 165, 245, 0.1));
  outline: 2px dashed var(--accent, #42a5f5);
  outline-offset: -4px;
}

.composer__thumbs {
  display: flex;
  gap: 6px;
  padding: 4px 0;
  flex-wrap: wrap;
}

.composer__thumb {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 60px;
}

.composer__thumb img {
  width: 60px;
  height: 60px;
  object-fit: cover;
  border-radius: 4px;
  border: 1px solid var(--border, #444);
}

.composer__thumb-remove {
  position: absolute;
  top: -4px;
  right: -4px;
  width: 18px;
  height: 18px;
  background: var(--danger, #e53935);
  color: white;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  font-size: 12px;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.composer__thumb-name {
  margin-top: 2px;
  font-size: 10px;
  color: var(--text-tertiary, #999);
  max-width: 60px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.composer__dropzone-hint {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(66, 165, 245, 0.2);
  color: white;
  font-weight: 500;
  pointer-events: none;
  z-index: 10;
}

.composer__row {
  display: flex;
  align-items: flex-end;
  gap: 4px;
}

.composer__icon-btn,
.composer__send {
  flex: 0 0 auto;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  border-radius: 4px;
  color: var(--text-primary, #e0e0e0);
  cursor: pointer;
  font-size: 18px;
  transition: all 0.15s;
}

.composer__icon-btn:hover:not(:disabled),
.composer__send:hover:not(:disabled) {
  background: var(--bg-hover, #333);
}

.composer__icon-btn:disabled,
.composer__send:disabled,
.composer__icon-btn.disabled,
.composer__send.is-disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.composer__send {
  background: var(--accent, #42a5f5);
  color: white;
}

.composer__send.is-disabled,
.composer__send:disabled {
  background: var(--bg-tertiary, #333);
}

.composer__send--stop {
  background: var(--danger, #e53935);
}

.composer__textarea {
  flex: 1;
  min-height: 32px;
  max-height: 96px;
  padding: 6px 8px;
  background: var(--bg-primary, #1a1a1a);
  border: 1px solid var(--border, #444);
  border-radius: 4px;
  color: inherit;
  font-family: inherit;
  font-size: 14px;
  line-height: 1.4;
  resize: none;
  outline: none;
  transition: border-color 0.15s;
}

.composer__textarea:focus {
  border-color: var(--accent, #42a5f5);
}

.composer__textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.composer__recording {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 6px;
  height: 32px;
  background: var(--danger-bg, rgba(229, 57, 53, 0.15));
  border: 1px solid var(--danger, #e53935);
  border-radius: 4px;
  color: var(--danger, #e53935);
  font-size: 12px;
  flex: 0 1 auto;
}

.composer__recording-dot {
  width: 8px;
  height: 8px;
  background: var(--danger, #e53935);
  border-radius: 50%;
  animation: composer-pulse 1s infinite;
}

@keyframes composer-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.composer__waveform {
  display: flex;
  align-items: center;
  gap: 1px;
  height: 24px;
}

.composer__waveform-bar {
  width: 2px;
  background: var(--danger, #e53935);
  border-radius: 1px;
  transition: height 0.05s;
}

.composer__recording-time {
  font-variant-numeric: tabular-nums;
  font-size: 11px;
}

.composer__vision-warn {
  padding: 4px 8px;
  background: rgba(255, 152, 0, 0.1);
  color: #ff9800;
  border-radius: 4px;
  font-size: 11px;
}

.composer.is-mobile .composer__row {
  gap: 2px;
}

.composer.is-mobile .composer__icon-btn,
.composer.is-mobile .composer__send {
  width: 36px;
  height: 36px;
  font-size: 16px;
}

.composer.is-mobile .composer__textarea {
  font-size: 16px; /* 防止 iOS 缩放 */
}
</style>
