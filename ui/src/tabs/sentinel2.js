import { info } from '../scripts/log';
import { listen } from '@tauri-apps/api/event';

export function initSentinel2Tab(invoke) {
  const panel = document.getElementById('panel-sentinel2');

  panel.innerHTML = `
    <div style="display:flex; align-items:center; gap:12px;">
      <button class="fetch-btn" id="s2-fetch">Fetch Scenes</button>
      <span class="status" id="s2-status"></span>
    </div>
    <div id="s2-progress-wrap" style="display:none; margin: 8px 0;">
      <div id="s2-progress-label" style="font-size:12px; margin-bottom:4px; opacity:0.7;"></div>
      <div style="background: var(--color-surface, #222); border-radius:4px; height:6px; overflow:hidden;">
        <div id="s2-progress-bar" style="height:100%; width:0%; background: var(--color-accent, #4a9eff); transition: width 0.3s ease;"></div>
      </div>
    </div>
    <div class="scene-grid" id="s2-grid"></div>
  `;

  const btn          = document.getElementById('s2-fetch');
  const status       = document.getElementById('s2-status');
  const grid         = document.getElementById('s2-grid');
  const progressWrap = document.getElementById('s2-progress-wrap');
  const progressBar  = document.getElementById('s2-progress-bar');
  const progressLabel = document.getElementById('s2-progress-label');

  function setProgress(pct, label) {
    progressWrap.style.display = 'block';
    progressBar.style.width = `${pct}%`;
    progressLabel.textContent = label;
  }

  function clearProgress() {
    progressWrap.style.display = 'none';
    progressBar.style.width = '0%';
    progressLabel.textContent = '';
  }

  btn.addEventListener('click', async () => {
    info("Starting data fetch", "sentinel2");

    btn.disabled = true;
    status.className = 'status';
    status.innerHTML = '<span class="spinner"></span> Fetching…';
    grid.innerHTML = '';
    setProgress(0, 'Starting…');

    const unlisten = await listen('sentinel2-progress', ({ payload }) => {
      const { step, current, total, scene_id } = payload;

      const labels = {
        init:        'Initialising client…',
        searching:   'Searching for scenes…',
        downloading: `Downloading scene ${current + 1} of ${total}…`,
        converting:  `Converting scene ${current + 1} of ${total} to PNG…`,
        done_scene:  `Scene ${current} of ${total} complete`,
        complete:    'All scenes loaded',
      };

      const pct =
        step === 'init' || step === 'searching' ? 5
        : step === 'complete' ? 100
        : Math.round(((current / total) * 90) + 5);

      setProgress(pct, labels[step] ?? step);
    });

    try {
      const scenes = await invoke('fetch_sentinel2', {
        username: import.meta.env?.VITE_CDSE_USERNAME  ?? '',
        password: import.meta.env?.VITE_CDSE_PASSWORD  ?? '',
        s3Access: import.meta.env?.VITE_CDSE_S3_ACCESS ?? '',
        s3Secret: import.meta.env?.VITE_CDSE_S3_SECRET ?? '',
      });

      if (scenes.length === 0) {
        status.textContent = 'No scenes found for this query.';
        clearProgress();
        return;
      }

      status.className = 'status ok';
      status.textContent = `${scenes.length} scene(s) loaded`;
      setTimeout(clearProgress, 2000);

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
      clearProgress();
    } finally {
      btn.disabled = false;
      unlisten();
    }
  });
}