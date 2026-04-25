export function initWeatherTab() {
  document.getElementById('panel-weather').innerHTML = `
    <div class="stub">
      <div class="stub-title">Weather / Met</div>
      <div class="stub-badge">COMING SOON</div>
      <p>Planned: Open-Meteo historical archive API — no auth required.</p>
    </div>
  `;
}