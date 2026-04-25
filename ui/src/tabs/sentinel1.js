export function initSentinel1Tab() {
  document.getElementById('panel-sentinel1').innerHTML = `
    <div class="stub">
      <div class="stub-title">Sentinel-1 SAR</div>
      <div class="stub-badge">COMING SOON</div>
      <p>Wire up the Sentinel-1 GRD collection in <code>src-tauri/src/commands/sentinel1.rs</code></p>
    </div>
  `;
}