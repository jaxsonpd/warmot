<script setup>
import { onMounted, onUnmounted, nextTick, ref, watch } from 'vue'
import L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import markerIcon2x from 'leaflet/dist/images/marker-icon-2x.png'
import markerIcon from 'leaflet/dist/images/marker-icon.png'
import markerShadow from 'leaflet/dist/images/marker-shadow.png'
import { useTheme } from '../composables/useTheme.js'

// Vite doesn't resolve Leaflet's default marker icon URLs the way a plain
// script tag would — point them at the bundled assets explicitly, once.
delete L.Icon.Default.prototype._getIconUrl
L.Icon.Default.mergeOptions({
  iconRetinaUrl: markerIcon2x,
  iconUrl: markerIcon,
  shadowUrl: markerShadow,
})

const props = defineProps({
  // App.vue now keeps this component mounted permanently (v-show, not
  // v-if) so the pan/zoom/marker state survives switching to the Data tab
  // and back. `active` just tells us when we've been made visible again,
  // since a container that was `display: none` reports a stale size.
  active: { type: Boolean, default: true },
})

const coordinates = defineModel('coordinates', { default: '' })

const { theme } = useTheme()
const mapContainer = ref(null)
let map = null
let marker = null

function formatLatLng(latlng) {
  return `${latlng.lat.toFixed(4)}, ${latlng.lng.toFixed(4)}`
}

function placeMarker(latlng) {
  if (marker) {
    marker.setLatLng(latlng)
  } else {
    marker = L.marker(latlng).addTo(map)
  }
}

function parseCoordinates(value) {
  const [lat, lng] = value.split(',').map((n) => parseFloat(n.trim()))
  return Number.isFinite(lat) && Number.isFinite(lng) ? { lat, lng } : null
}

onMounted(() => {
  // Centered on Europe by default — swap for wherever you search most.
  map = L.map(mapContainer.value, { center: [50, 10], zoom: 4 })

  // Esri's World Street Map tiles — unlike OSM's standard tiles, these are
  // labeled in English everywhere rather than in each place's local
  // language/script. No API key required. Note the {z}/{y}/{x} order.
  L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Street_Map/MapServer/tile/{z}/{y}/{x}', {
    maxZoom: 19,
    attribution: 'Tiles &copy; Esri — Source: Esri, DeLorme, NAVTEQ, USGS, Intermap, iPC, NRCAN, Esri Japan, METI, Esri China (Hong Kong), TomTom',
  }).addTo(map)

  map.on('click', (e) => {
    placeMarker(e.latlng)
    coordinates.value = formatLatLng(e.latlng)
  })

  // If ControlBar already has coordinates typed in (or set by a previous
  // click before this tab was mounted), start centered there.
  const existing = coordinates.value && parseCoordinates(coordinates.value)
  if (existing) {
    map.setView(existing, 8)
    placeMarker(existing)
  }

  requestAnimationFrame(() => map.invalidateSize())
})

onUnmounted(() => {
  map?.remove()
  map = null
  marker = null
})

watch(
  () => props.active,
  (isActive) => {
    if (isActive) {
      // The container was display:none a moment ago and reported a 0x0
      // size to Leaflet; wait for it to actually be laid out, then fix up.
      nextTick(() => map?.invalidateSize())
    }
  }
)

// Leaflet/OSM don't ship a dark tile set, so invert the raster tiles for
// dark mode rather than pulling in a second (often paid) tile provider.
watch(
  theme,
  (value) => {
    mapContainer.value?.classList.toggle('map-dark', value === 'dark')
  },
  { immediate: true }
)
</script>

<template>
  <div ref="mapContainer" class="map"></div>
</template>

<style scoped>
.map {
  width: 100%;
  height: 100%;
}

.map.map-dark :deep(.leaflet-tile-pane) {
  filter: invert(1) hue-rotate(180deg) brightness(0.95) contrast(0.9);
}

.map.map-dark :deep(.leaflet-marker-icon),
.map.map-dark :deep(.leaflet-marker-shadow) {
  filter: invert(1) hue-rotate(180deg);
}
</style>