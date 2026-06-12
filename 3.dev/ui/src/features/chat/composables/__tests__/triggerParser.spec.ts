/**
 * triggerParser 单元测试 - TASK-2.4 (2026-06-10)
 *
 * 覆盖:
 *   - @ 文件 trigger 解析
 *   - / 命令 trigger
 *   - /review 子模式识别
 *   - $ 技能 trigger
 *   - 词边界规则（@前面紧跟字母不算 trigger）
 *   - 中途空白结束 trigger
 *   - 空 query 也能匹配
 *   - 边界情况（光标 0 / cursor 超出范围）
 */
import { describe, it, expect } from 'vitest'
import { parseTriggerAtCursor } from './triggerParser'

describe('parseTriggerAtCursor', () => {
  it('parses @file trigger at start of input', () => {
    const r = parseTriggerAtCursor('@src', 4)
    expect(r).toEqual({ type: 'file', query: 'src', start: 0 })
  })

  it('parses @file with full path', () => {
    const r = parseTriggerAtCursor('See @src/utils/index.ts', 25)
    expect(r?.type).toBe('file')
    expect(r?.query).toBe('src/utils/index.ts')
    expect(r?.start).toBe(4)
  })

  it('parses /command trigger', () => {
    const r = parseTriggerAtCursor('Type /hel', 9)
    expect(r).toEqual({ type: 'command', query: 'hel', start: 5 })
  })

  it('parses /review sub-mode', () => {
    // /review 后面有空格 + 子命令
    const r = parseTriggerAtCursor('/review sec', 11)
    expect(r?.type).toBe('review')
    expect(r?.query).toBe('sec')
  })

  it('parses $skill trigger', () => {
    const r = parseTriggerAtCursor('Use $react', 10)
    expect(r).toEqual({ type: 'skill', query: 'react', start: 4 })
  })

  it('parses $skill with multi-word query', () => {
    // $ 后面可以是单词字符 + .
    const r = parseTriggerAtCursor('$python.dev', 11)
    expect(r?.type).toBe('skill')
    expect(r?.query).toBe('python.dev')
  })

  it('returns null when trigger is preceded by word char (no word boundary)', () => {
    // hello@src → @ 不是 trigger（前面是字母）
    const r = parseTriggerAtCursor('hello@src', 9)
    expect(r).toBeNull()
  })

  it('returns null when trigger is mid-text without preceding space', () => {
    // a/@ → 紧跟前一个 token，不是 trigger
    const r = parseTriggerAtCursor('a/@', 3)
    expect(r).toBeNull()
  })

  it('returns null for plain text without trigger', () => {
    expect(parseTriggerAtCursor('hello world', 11)).toBeNull()
  })

  it('returns null when cursor is 0', () => {
    expect(parseTriggerAtCursor('@something', 0)).toBeNull()
  })

  it('returns null when cursor is beyond text length', () => {
    expect(parseTriggerAtCursor('@abc', 100)).toBeNull()
  })

  it('handles trigger with empty query', () => {
    const r = parseTriggerAtCursor('text @', 6)
    expect(r).toEqual({ type: 'file', query: '', start: 5 })
  })

  it('handles trigger right after space', () => {
    const r = parseTriggerAtCursor('hi @main', 8)
    expect(r?.type).toBe('file')
    expect(r?.query).toBe('main')
    expect(r?.start).toBe(3)
  })

  it('closes trigger when whitespace appears', () => {
    // "react " 后面是空白，$ 解析时已结束
    const r = parseTriggerAtCursor('$react ', 7)
    expect(r).toBeNull()
  })

  it('handles trigger at the end of input', () => {
    const r = parseTriggerAtCursor('hello @', 7)
    expect(r).toEqual({ type: 'file', query: '', start: 6 })
  })

  it('preserves slashes in file query', () => {
    const r = parseTriggerAtCursor('@a/b/c/d', 7)
    expect(r?.query).toBe('a/b/c/d')
  })

  it('preserves dashes and underscores', () => {
    const r = parseTriggerAtCursor('@my-test_file', 13)
    expect(r?.query).toBe('my-test_file')
  })

  it('preserves dots in query', () => {
    const r = parseTriggerAtCursor('@component.tsx', 14)
    expect(r?.query).toBe('component.tsx')
  })
})
