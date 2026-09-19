import { ref, watch } from 'vue'

const STORAGE_KEY = 'warmot-theme'

function getInitialTheme() {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored === 'light' || stored === 'dark') return stored
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function applyTheme(value) {
  document.documentElement.setAttribute('data-theme', value)
}

// Module-level state: every component that calls useTheme() shares the
// same ref, so the toggle in MenuBar affects the whole app with no store lib.
const theme = ref(getInitialTheme())
applyTheme(theme.value)

watch(theme, (value) => {
  applyTheme(value)
  localStorage.setItem(STORAGE_KEY, value)
})

export function useTheme() {
  function toggleTheme() {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
  }

  function setTheme(value) {
    theme.value = value
  }

  return { theme, toggleTheme, setTheme }
}