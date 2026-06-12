<script setup lang="ts">
/**
 * UploadSkillDialog — 上传自定义技能
 * 2026-06-09 TASK-4.5
 *
 * 表单：id/name/version/author/category/description/prompt_template/tags
 */
import { ref, watch } from 'vue'
import { showToast } from 'vant'
import type { Skill } from '@/api'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  'update:show': [v: boolean]
  submit: [meta: Partial<Skill> & { id: string; name: string; version: string; category: string; prompt_template: string }]
}>()

const form = ref({
  id: '',
  name: '',
  version: '0.1.0',
  author: '',
  category: 'other',
  tagsInput: '',
  description: '',
  prompt_template: '',
})
const submitting = ref(false)

const CATEGORIES = [
  'scaffold', 'refactor', 'test', 'doc', 'review',
  'debug', 'deploy', 'data', 'ui', 'workflow', 'other',
]

watch(() => props.show, (v) => {
  if (v) {
    // 重置
    form.value = {
      id: '',
      name: '',
      version: '0.1.0',
      author: '',
      category: 'other',
      tagsInput: '',
      description: '',
      prompt_template: '',
    }
    submitting.value = false
  }
})

function onIdInput() {
  // 自动把 ID 转小写 + 中划线
  form.value.id = form.value.id.toLowerCase().replace(/[^a-z0-9\-_]/g, '-')
}

function validate(): string | null {
  const idPattern = /^[a-z0-9][a-z0-9\-_]{0,63}$/
  if (!idPattern.test(form.value.id)) {
    return 'ID 必须是小写字母+数字+中划线，且以字母或数字开头'
  }
  if (!form.value.name.trim()) return '请输入名称'
  if (!form.value.version.trim()) return '请输入版本号'
  if (!form.value.description.trim()) return '请输入描述'
  if (!form.value.prompt_template.trim()) return '请输入 prompt 模板'
  return null
}

async function onSubmit() {
  const err = validate()
  if (err) {
    showToast(err)
    return
  }
  submitting.value = true
  try {
    const meta: Partial<Skill> & { id: string; name: string; version: string; category: string; prompt_template: string } = {
      id: form.value.id,
      name: form.value.name,
      version: form.value.version,
      author: form.value.author || 'User',
      category: form.value.category,
      tags: form.value.tagsInput
        .split(/[,\s]+/)
        .map(s => s.trim())
        .filter(Boolean),
      description: form.value.description,
      prompt_template: form.value.prompt_template,
    }
    emit('submit', meta)
    emit('update:show', false)
  } finally {
    submitting.value = false
  }
}

function onCancel() {
  emit('update:show', false)
}
</script>

<template>
  <van-dialog
    :show="show"
    @update:show="(v: boolean) => emit('update:show', v)"
    title="上传自定义技能"
    :show-cancel-button="true"
    :style="{ width: '90vw', maxWidth: '600px' }"
    @confirm="onSubmit"
    @cancel="onCancel"
  >
    <div class="upload-form">
      <div class="form-group">
        <label class="form-label">技能 ID * <span class="form-hint">(小写字母+数字+中划线)</span></label>
        <input
          v-model="form.id"
          class="form-input"
          placeholder="my-custom-skill"
          @input="onIdInput"
        />
      </div>

      <div class="form-group">
        <label class="form-label">名称 *</label>
        <input
          v-model="form.name"
          class="form-input"
          placeholder="我的自定义技能"
        />
      </div>

      <div class="form-row">
        <div class="form-group">
          <label class="form-label">版本 *</label>
          <input
            v-model="form.version"
            class="form-input"
            placeholder="0.1.0"
          />
        </div>
        <div class="form-group">
          <label class="form-label">作者</label>
          <input
            v-model="form.author"
            class="form-input"
            placeholder="留空则用 User"
          />
        </div>
      </div>

      <div class="form-group">
        <label class="form-label">分类 *</label>
        <select v-model="form.category" class="form-select">
          <option v-for="c in CATEGORIES" :key="c" :value="c">{{ c }}</option>
        </select>
      </div>

      <div class="form-group">
        <label class="form-label">标签 <span class="form-hint">(逗号或空格分隔)</span></label>
        <input
          v-model="form.tagsInput"
          class="form-input"
          placeholder="frontend, react, helper"
        />
      </div>

      <div class="form-group">
        <label class="form-label">简介 *</label>
        <textarea
          v-model="form.description"
          class="form-textarea"
          rows="2"
          placeholder="一句话描述这个技能做什么"
        />
      </div>

      <div class="form-group">
        <label class="form-label">Prompt 模板 * <span class="form-hint">(用 <code v-pre>{{variable}}</code> 表示变量)</span></label>
        <textarea
          v-model="form.prompt_template"
          class="form-textarea form-code"
          rows="6"
          placeholder="请基于以下需求：&#10;&#10;{{requirement}}"
        />
      </div>
    </div>
  </van-dialog>
</template>

<style scoped>
.upload-form {
  padding: 16px;
  max-height: 60vh;
  overflow-y: auto;
}

.form-group {
  margin-bottom: 12px;
}

.form-row {
  display: flex;
  gap: 10px;
}

.form-row .form-group {
  flex: 1;
}

.form-label {
  display: block;
  font-size: 12px;
  font-weight: 500;
  color: var(--yj-text-secondary, #aaa);
  margin-bottom: 4px;
}

.form-hint {
  font-weight: 400;
  color: var(--yj-text-secondary, #666);
  font-size: 11px;
}

.form-input,
.form-select,
.form-textarea {
  width: 100%;
  background: var(--yj-bg-base, #0d0d0d);
  border: 1px solid var(--yj-border, #2a2a2a);
  border-radius: 4px;
  padding: 8px 10px;
  color: var(--yj-text-primary, #ddd);
  font-size: 13px;
  box-sizing: border-box;
  font-family: inherit;
}

.form-input:focus,
.form-select:focus,
.form-textarea:focus {
  outline: none;
  border-color: var(--yj-accent, #7c3aed);
}

.form-textarea {
  resize: vertical;
  min-height: 50px;
}

.form-code {
  font-family: 'Cascadia Code', 'Consolas', monospace;
  font-size: 12px;
  line-height: 1.5;
}
</style>
