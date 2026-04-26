const entries = document.getElementById('log-entries');
let activeLevel = 'all';
let allLogs = [];

function pad(n) { return String(n).padStart(2, '0'); }
function ts() {
  const d = new Date();
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${String(d.getMilliseconds()).padStart(3,'0')}`;
}

export function log(level, msg, detail = '') {
  const entry = { ts: ts(), level, msg, detail };
  allLogs.push(entry);
  if (activeLevel === 'all' || activeLevel === level) renderEntry(entry, true);
}

export const info  = (msg, detail) => log('info',  msg, detail);
export const warn  = (msg, detail) => log('warn',  msg, detail);
export const error = (msg, detail) => log('error', msg, detail);
export const debug = (msg, detail) => log('debug', msg, detail);

function renderEntry(entry, scroll) {
  const row = document.createElement('div');
  row.className = 'log-entry';
  row.dataset.level = entry.level;
  row.innerHTML = `
    <span class="log-ts">${entry.ts}</span>
    <span class="log-level ${entry.level}">${entry.level}</span>
    <span class="log-msg">${entry.msg}${entry.detail
      ? ' <span class="dim">' + entry.detail + '</span>' : ''}</span>`;
  entries.appendChild(row);
  if (scroll) entries.scrollTop = entries.scrollHeight;
}

function rerender() {
  entries.innerHTML = '';
  allLogs.forEach(e => {
    if (activeLevel === 'all' || activeLevel === e.level) renderEntry(e, false);
  });
  entries.scrollTop = entries.scrollHeight;
}

document.querySelectorAll('.log-filter-btn[data-level]').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.log-filter-btn[data-level]').forEach(b =>
      b.classList.remove('active'));
    btn.classList.add('active');
    activeLevel = btn.dataset.level;
    rerender();
  });
});

document.getElementById('log-clear').addEventListener('click', () => {
  allLogs = [];
  entries.innerHTML = '';
});