import { ref, onUnmounted } from 'vue'

export interface WebSocketMessage {
  type: string
  data: unknown
}

export function useWebSocket(url?: string) {
  const connected = ref(false)
  const lastMessage = ref<WebSocketMessage | null>(null)
  const reconnectCount = ref(0)

  let ws: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let messageHandlers: Map<string, Array<(data: unknown) => void>> = new Map()

  const wsUrl = url || `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}/ws`

  function connect() {
    if (ws?.readyState === WebSocket.OPEN || ws?.readyState === WebSocket.CONNECTING) return

    try {
      ws = new WebSocket(wsUrl)

      ws.onopen = () => {
        connected.value = true
        reconnectCount.value = 0
      }

      ws.onclose = () => {
        connected.value = false
        scheduleReconnect()
      }

      ws.onerror = () => {
        connected.value = false
      }

      ws.onmessage = (event) => {
        try {
          const msg: WebSocketMessage = JSON.parse(event.data)
          lastMessage.value = msg
          const handlers = messageHandlers.get(msg.type)
          if (handlers) {
            handlers.forEach((fn) => fn(msg.data))
          }
          const wildcardHandlers = messageHandlers.get('*')
          if (wildcardHandlers) {
            wildcardHandlers.forEach((fn) => fn(msg))
          }
        } catch {
          const textHandlers = messageHandlers.get('text')
          if (textHandlers) {
            textHandlers.forEach((fn) => fn(event.data))
          }
        }
      }
    } catch {
      scheduleReconnect()
    }
  }

  function scheduleReconnect() {
    if (reconnectCount.value >= 10) return
    const delay = Math.min(1000 * Math.pow(2, reconnectCount.value), 30000)
    reconnectCount.value++
    reconnectTimer = setTimeout(() => connect(), delay)
  }

  function disconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    reconnectCount.value = 10
    ws?.close()
    ws = null
    connected.value = false
  }

  function send(type: string, data: unknown = {}) {
    if (ws?.readyState !== WebSocket.OPEN) return false
    ws.send(JSON.stringify({ type, data }))
    return true
  }

  function on(type: string, handler: (data: unknown) => void) {
    if (!messageHandlers.has(type)) {
      messageHandlers.set(type, [])
    }
    messageHandlers.get(type)!.push(handler)
  }

  function off(type: string, handler: (data: unknown) => void) {
    const handlers = messageHandlers.get(type)
    if (handlers) {
      const idx = handlers.indexOf(handler)
      if (idx >= 0) handlers.splice(idx, 1)
    }
  }

  onUnmounted(() => {
    disconnect()
    messageHandlers.clear()
  })

  return {
    connected,
    lastMessage,
    reconnectCount,
    connect,
    disconnect,
    send,
    on,
    off,
  }
}
