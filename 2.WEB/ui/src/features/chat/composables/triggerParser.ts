/**
 * triggerParser.ts - Autocomplete 触发器解析
 *
 * TASK-2.4 (2026-06-10) Composer Autocomplete
 *
 * 在用户输入时，从光标位置往前找 trigger（`/` / `@` / `$`），
 * 解析出 (type, query, startIdx)。
 *
 * 4 种触发类型:
 *   - file:    `@<query>` - 当前 workspace 文件路径
 *   - command: `/<query>` - 系统/自定义命令
 *   - review:  `/review <query>` - 代码审查子模式
 *   - skill:   `$<query>` - 技能列表（v2.0 技能市场）
 *
 * 触发规则:
 *   - 触发符必须是 "单词边界"（前一个字符为空格/行首/标点）
 *   - query 可包含字母/数字/-/_/./
 *   - 中途出现空白 → trigger 结束
 */
export type AutocompleteType = 'file' | 'command' | 'review' | 'skill'

export interface ParsedTrigger {
  type: AutocompleteType
  query: string
  start: number
}

// query 字符集：字母/数字/-/_/.
// file 类型的 query 额外允许 `/`（路径分隔符）
const WORD_CHAR_BASE = /[A-Za-z0-9_\-.]/
const WORD_CHAR_FILE = /[A-Za-z0-9_\-./]/
const TRIGGER_CHARS = new Set(['@', '$', '/'])

/**
 * 从光标位置往前解析 trigger
 * @param text 完整文本
 * @param cursor 光标位置（0-based 字符索引，cursor==text.length 表示在末尾）
 */
export function parseTriggerAtCursor(text: string, cursor: number): ParsedTrigger | null {
  if (cursor < 0 || cursor > text.length) return null
  // 切片：从 cursor 往左取一段（最长 200 字符），用正则匹配最近的 trigger
  const start = Math.max(0, cursor - 200)
  const slice = text.slice(start, cursor)
  // 匹配模式: (开头|空白|([) + trigger符 + word+slash 字符
  // 使用 lastIndexOf 找最近的 trigger
  // 策略：从右往左找 `@`/`$`/`/`，再判断是否符合 trigger 规则
  let bestIdx = -1
  let bestType: AutocompleteType | null = null
  let bestQueryStart = -1  // for /review：跳过 'review ' 部分
  for (let j = slice.length - 1; j >= 0; j--) {
    const c = slice[j]
    if (c !== '@' && c !== '$' && c !== '/') continue
    // 检查词边界：c 前面必须是 start-of-slice | 空白 | ([
    if (j > 0) {
      const prev = slice[j - 1]
      // 前一个字符是 word char 或 / → 不是 trigger
      if (/[A-Za-z0-9_\-./]/.test(prev)) continue
    }
    // 找到候选 trigger
    if (c === '@') {
      bestIdx = j
      bestType = 'file'
      break
    } else if (c === '$') {
      bestIdx = j
      bestType = 'skill'
      break
    } else {
      // c === '/'：检查是否是 /review
      const after = slice.slice(j + 1)
      const reviewMatch = after.match(/^review\b\s*/)
      if (reviewMatch) {
        const afterReview = j + 1 + reviewMatch[0].length
        if (afterReview < slice.length) {
          // /review 后面还有内容，type=review，query=后面部分
          bestIdx = j
          bestType = 'review'
          // query 收集跳过 'review' 部分
          bestIdx = j
          // 我们需要让 query 跳过 'review '
          // hack: bestIdx 不变（trigger 在 / 处），但 query 收集用 custom 逻辑
          // 改用 bestIdxQueryStart
          bestQueryStart = afterReview
          break
        } else {
          bestIdx = j
          bestType = 'command'
          break
        }
      } else {
        bestIdx = j
        bestType = 'command'
        break
      }
    }
  }
  if (bestIdx < 0 || bestType === null) return null
  // 收集 query
  const charSet = bestType === 'file' ? WORD_CHAR_FILE : WORD_CHAR_BASE
  const queryStart = bestQueryStart >= 0 ? bestQueryStart : bestIdx + 1
  let qEnd = queryStart
  while (qEnd < slice.length && charSet.test(slice[qEnd])) {
    qEnd++
  }
  if (qEnd < slice.length) return null
  const query = slice.slice(queryStart, qEnd)
  return { type: bestType, query, start: start + bestIdx }
}
