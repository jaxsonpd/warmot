export function initSentinel2Tab(invoke) {
  const panel  = document.getElementById('panel-sentinel2');

  panel.innerHTML = `
    <div style="display:flex; align-items:center; gap:12px;">
      <button class="fetch-btn" id="s2-fetch">Fetch Scenes</button>
      <span class="status" id="s2-status"></span>
    </div>
    <div class="scene-grid" id="s2-grid"></div>
  `;

  const btn    = document.getElementById('s2-fetch');
  const status = document.getElementById('s2-status');
  const grid   = document.getElementById('s2-grid');

  btn.addEventListener('click', async () => {
    btn.disabled = true;
    status.className = 'status';
    status.innerHTML = '<span class="spinner"></span> Fetching…';
    grid.innerHTML = '';

    try {
      // Credentials from env — for now prompt once, or hard-code for dev.
      // In production these come from a settings panel / secure store.
      const scenes = await invoke('fetch_sentinel2', {
        username:  import.meta.env?.VITE_CDSE_USERNAME  ?? '',
        password:  import.meta.env?.VITE_CDSE_PASSWORD  ?? '',
        s3Access:  import.meta.env?.VITE_CDSE_S3_ACCESS ?? '',
        s3Secret:  import.meta.env?.VITE_CDSE_S3_SECRET ?? '',
      });

      if (scenes.length === 0) {
        status.textContent = 'No scenes found for this query.';
        return;
      }

      status.className = 'status ok';
      status.textContent = `${scenes.length} scene(s) loaded`;

      for (const scene of scenes) {
        const card = document.createElement('div');
        card.className = 'scene-card';
        card.innerHTML = `
          <img src="data:image/png;base64,${scene.png_b64}" alt="${scene.id}" loading="lazy" />
          <div class="card-meta">
            <strong>${scene.datetime.slice(0, 10)}</strong>
            <span>Cloud cover: ${scene.cloud_cover.toFixed(1)}%</span>
            <span style="word-break:break-all; font-size:11px;">${scene.id}</span>
          </div>
        `;
        grid.appendChild(card);
      }
    } catch (err) {
      status.className = 'status error';
      status.textContent = `Error: ${err}`;
    } finally {
      btn.disabled = false;
    }
  });
}