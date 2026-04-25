import { invoke } from '@tauri-apps/api/core';
import { initSentinel2Tab } from './tabs/sentinel2.js';
import { initSentinel1Tab } from './tabs/sentinel1.js';
import { initWeatherTab }   from './tabs/weather.js';
import { initAisTab }       from './tabs/ais.js';

// ── Tab switching ─────────────────────────────────────────────────────────────
document.querySelectorAll('.tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.panel').forEach(p => p.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById(`panel-${btn.dataset.tab}`).classList.add('active');
  });
});

// ── Query bar → Rust state ────────────────────────────────────────────────────
async function applyQuery() {
  await invoke('update_query', {
    params: {
      lon:        parseFloat(document.getElementById('q-lon').value),
      lat:        parseFloat(document.getElementById('q-lat').value),
      radius_deg: parseFloat(document.getElementById('q-radius').value),
      date_from:  document.getElementById('q-from').value,
      date_to:    document.getElementById('q-to').value,
    }
  });
}

document.getElementById('btn-apply').addEventListener('click', applyQuery);

// ── Boot ──────────────────────────────────────────────────────────────────────
await applyQuery(); // seed Rust state with defaults on load

initSentinel2Tab(invoke);
initSentinel1Tab();
initWeatherTab();
initAisTab();