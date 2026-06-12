const WORD_CHAR = /[A-Za-z0-9_\-./]/

function parse(text, cursor) {
  if (cursor <= 0 || cursor > text.length) return null
  let i = cursor - 1
  while (i >= 0 && /\s/.test(text[i])) i--
  if (i < 0) return null
  const ch = text[i]
  let triggerIdx = -1
  let type = null
  if (ch === '@') { triggerIdx = i; type = 'file' }
  else if (ch === '$') { triggerIdx = i; type = 'skill' }
  else if (ch === '/') { triggerIdx = i; type = 'command' }
  else return null
  if (triggerIdx > 0) {
    const prev = text[triggerIdx - 1]
    if (WORD_CHAR.test(prev)) return null
  }
  let qEnd = triggerIdx + 1
  while (qEnd < cursor && WORD_CHAR.test(text[qEnd])) qEnd++
  if (qEnd < cursor) return null
  return { type, query: text.slice(triggerIdx + 1, qEnd), start: triggerIdx }
}

console.log('result:', parse('@src', 4))
console.log('result:', parse('text @', 6))
console.log('result:', parse('@', 1))
