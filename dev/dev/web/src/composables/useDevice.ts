import { ref, computed, onMounted, onUnmounted } from 'vue'

export function useDevice() {
  const width = ref(window.innerWidth)
  const isMobile = computed(() => width.value < 600)
  const isDesktop = computed(() => !isMobile.value)

  function onResize() {
    width.value = window.innerWidth
  }

  onMounted(() => window.addEventListener('resize', onResize))
  onUnmounted(() => window.removeEventListener('resize', onResize))

  return {
    width,
    isMobile,
    isDesktop,
  }
}
