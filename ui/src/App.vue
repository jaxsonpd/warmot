<script setup>
import { ref } from 'vue'
import { useTheme } from './composables/useTheme.js'
import MapView from './components/MapView.vue'

// Everything lives inline in this file for now. As we build MenuBar.vue,
// TabBar.vue, ControlBar.vue, MapPanel.vue and DataPanel.vue, each one
// will lift a section out of here (and out of this file's <style>) — the
// markup and classes are written so that move is a straight cut-and-paste.

const { theme, toggleTheme } = useTheme()

const activeTab = ref('map')
const coordinates = ref('')
const startDate = ref('')
const endDate = ref('')
const layer = ref('optical')

function showAbout() {
  window.alert('warmot — a viewer for Copernicus satellite imagery.')
}
</script>

<template>
  <div class="app">
    <!-- will become MenuBar.vue -->
    <header class="menu-bar">
      <nav class="menu-items">
        <button class="menu-item" type="button">File</button>
        <button class="menu-item" type="button" @click="showAbout">About</button>
      </nav>
      <button
        class="theme-toggle"
        type="button"
        :aria-label="theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'"
        @click="toggleTheme"
      >
        {{ theme === 'dark' ? '☾' : '☀' }}
      </button>
    </header>

    <!-- will become TabBar.vue -->
    <nav class="tab-bar">
      <button
        type="button"
        class="tab"
        :class="{ active: activeTab === 'map' }"
        @click="activeTab = 'map'"
      >
        Map
      </button>
      <button
        type="button"
        class="tab"
        :class="{ active: activeTab === 'data' }"
        @click="activeTab = 'data'"
      >
        Data
      </button>
    </nav>

    <!-- will become ControlBar.vue -->
    <div class="control-bar">
      <label v-if="activeTab !== 'map'" class="field">
        <span class="field-label">Coordinates</span>
        <input v-model="coordinates" type="text" class="field-input mono" placeholder="lat, lon" />
      </label>

      <div class="field">
        <span class="field-label">Dates</span>
        <div class="date-range">
          <input v-model="startDate" type="date" class="field-input mono" />
          <span class="date-sep">–</span>
          <input v-model="endDate" type="date" class="field-input mono" />
        </div>
      </div>

      <div class="field">
        <span class="field-label">Layer</span>
        <div class="segmented" role="radiogroup" aria-label="Imagery layer">
          <button
            type="button"
            class="segment"
            :class="{ active: layer === 'optical' }"
            @click="layer = 'optical'"
          >
            Sat image
          </button>
          <button
            type="button"
            class="segment"
            :class="{ active: layer === 'radar' }"
            @click="layer = 'radar'"
          >
            Radar
          </button>
        </div>
      </div>
    </div>

    <!-- will become MapPanel.vue / DataPanel.vue -->
    <section class="panel">
      <div class="viewport" :class="{ 'viewport--map': activeTab === 'map' }">
        <MapView v-show="activeTab === 'map'" v-model:coordinates="coordinates" :active="activeTab === 'map'" />
        <p v-show="activeTab !== 'map'" class="placeholder">
          Data view — results for {{ coordinates || 'no coordinates yet' }} ·
          {{ layer === 'radar' ? 'Sentinel-1' : 'Sentinel-2' }} ·
          {{ startDate || '—' }} to {{ endDate || '—' }}
        </p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* menu bar */
.menu-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 36px;
  padding: 0 var(--space-2);
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.menu-items {
  display: flex;
  gap: var(--space-1);
}
.menu-item {
  background: none;
  border: none;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius);
  color: var(--text);
}
.menu-item:hover {
  background: var(--surface-raised);
}
.theme-toggle {
  background: none;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text);
}
.theme-toggle:hover {
  background: var(--surface-raised);
}

/* tab bar */
.tab-bar {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-2) var(--space-2) 0;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.tab {
  background: var(--surface);
  border: 1px solid var(--border);
  border-bottom: none;
  border-radius: var(--radius) var(--radius) 0 0;
  padding: var(--space-2) var(--space-4);
  color: var(--text-muted);
  transform: translateY(1px);
}
.tab.active {
  color: var(--text);
  background: var(--surface-raised);
  border-color: var(--accent);
  box-shadow: inset 0 2px 0 var(--accent);
}

/* control bar */
.control-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--space-4);
  padding: var(--space-3);
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.field-label {
  font-size: 12px;
  color: var(--text-muted);
}
.field-input {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-1) var(--space-2);
  color: var(--text);
  min-width: 150px;
}
.field-input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.mono {
  font-family: var(--font-mono);
}
.date-range {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}
.date-range .field-input {
  min-width: 130px;
}
.date-sep {
  color: var(--text-muted);
}
.segmented {
  display: flex;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}
.segment {
  background: var(--bg);
  border: none;
  border-right: 1px solid var(--border);
  padding: var(--space-1) var(--space-3);
  color: var(--text-muted);
}
.segment:last-child {
  border-right: none;
}
.segment.active {
  background: var(--accent-dim);
  color: var(--accent);
}

/* panel / viewport */
.panel {
  flex: 1;
  padding: var(--space-3);
  overflow: auto;
  min-height: 0;
}
.viewport {
  height: 100%;
  min-height: 320px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  overflow: hidden;
}
.viewport--map {
  display: block;
}
.placeholder {
  max-width: 40ch;
  color: var(--text-muted);
  padding: var(--space-4);
  line-height: 1.8;
}
</style>