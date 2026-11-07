// Physis Studio — front-end runtime. Mirrors the Physis Console shell
// (header chips, nav tabs, status pill, toast, ? overlay) so both UIs behave
// the same way; everything below talks to the physis-core studio API.

const $ = id => document.getElementById(id);
const state = { selectedCell: null, editingName: null, tab: 'classify' };

function esc(s) {
  if (s == null) return '';
  return String(s)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;')
    .replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

// Cell keys are DOMAIN\0MODE on the wire; never show the NUL to a human.
const CELL_SEP = '\u0000';
const cellLabel = key => String(key || '').split(CELL_SEP).join(' × ');

async function api(path, body) {
  const res = await fetch(path, {
    method: body ? 'POST' : 'GET',
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(res.status + ' ' + (await res.text()).slice(0, 160));
  return res.json();
}

let toastTimer;
function toast(msg, isErr) {
  const t = $('toast');
  t.textContent = msg;
  t.className = 'toast show ' + (isErr ? 'err' : 'ok');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { t.className = 'toast'; }, 3200);
}

function setStatus(msg, isErr) {
  const ts = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  $('status').innerHTML = '<span class="' + (isErr ? 'err' : 'muted') + '">' + (isErr ? '⚠ ' : '') + esc(msg) + ' · ' + ts + '</span>';
}

// Any handler can be wrapped: failures land in the status pill and a toast
// instead of dying silently in the console.
function guard(label, fn) {
  return async (...args) => {
    try {
      await fn(...args);
    } catch (e) {
      setStatus(label + ' failed: ' + e.message, true);
      toast(label + ' failed', true);
    }
  };
}

function typing() {
  const tag = document.activeElement && document.activeElement.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT';
}

/* ── tabs ─────────────────────────────────────────────────────── */
const TABS = ['classify', 'grid', 'ontology', 'flow', 'coherence', 'corpus', 'discover', 'structure', 'quality', 'edition', 'communities'];

function showTab(name) {
  state.tab = name;
  history.replaceState(null, '', '#' + name);
  document.querySelectorAll('nav button[data-tab]').forEach(b => {
    b.classList.toggle('active', b.dataset.tab === name);
  });
  TABS.forEach(t => $('tab-' + t).classList.toggle('active', t === name));
  // Tabs are lazy: the grid and corpus calls are the expensive ones, so they
  // only run when their tab is actually shown.
  if (name === 'grid') loadGrid();
  if (name === 'ontology') { loadOntology(); if (!lab.loaded) loadLabMap(); else labResize(); }
  if (name === 'flow') startFlow();
  if (name === 'coherence') { if (!drawRadar.last) drawRadar(null); }
  if (name === 'corpus') { loadSnapshot(); loadNodes(); }
  if (name === 'quality') loadQuality();
  if (name === 'edition') loadEdition();
  if (name === 'communities') loadCommunities();
}

/* ── communities (9.4b) ───────────────────────────────────────── */
async function loadCommunities() {
  const el = $('communitiesList');
  try {
    const r = await api('/api/v1/communities');
    if (!r.communities.length) { el.innerHTML = '<div class="muted">No labeled nodes yet — ingest or scan to grow the graph.</div>'; return; }
    const sorted = r.communities.slice().sort((a, b) => b.members.length - a.members.length);
    el.innerHTML = sorted.map(c => `
      <div class="panel">
        <b>${esc(c.members.length)} nodes</b> · cohesion ${(c.cohesion * 100).toFixed(0)}%
        <div class="muted" style="margin-top:6px">${c.members.slice(0, 12).map(esc).join(' · ')}${c.members.length > 12 ? ' …' : ''}</div>
      </div>`).join('');
  } catch (e) {
    el.innerHTML = '<div class="muted">Failed to load communities: ' + esc(e.message) + '</div>';
  }
}
$('communitiesReload').onclick = () => loadCommunities();
document.querySelectorAll('nav button[data-tab]').forEach(b => {
  b.onclick = () => showTab(b.dataset.tab);
});

/* ── header / health ──────────────────────────────────────────── */
async function loadHealth() {
  try {
    const h = await api('/api/health');
    // Whether the embedder is semantic decides how much every number below is
    // worth, so say it on the chip itself. A tooltip does not count: nobody
    // hovers a header before trusting a score they can already read.
    const emb = $('chEmbedder');
    emb.innerHTML = 'embedder <b>' + esc(h.embedder) + '</b>'
      + (h.semantic ? '' : ' <span class="qual">· not semantic</span>');
    emb.className = 'chip' + (h.semantic ? ' good' : ' warn');
    emb.title = (h.semantic
      ? 'trained semantic model · ' + h.dimension + 'd'
      : 'deterministic feature hashing at ' + h.dimension + 'd — reproducible, but '
        + 'similarity is coarse and not meaning-based. Pass --model for real semantics.');
    $('chEntries').innerHTML = 'entries <b>' + h.entries + '</b>' + (h.custom ? ' <span class="muted">+' + h.custom + '</span>' : '');
    $('chCells').innerHTML = 'cells <b>' + h.cells + '</b>';
    $('chNodes').innerHTML = 'corpus <b>' + h.nodes + '</b>';
    // Coherence is a mean cosine over the corpus. Under a non-semantic embedder
    // it trends high for reasons that have nothing to do with meaning, so a bare
    // "0.99" reads as a great score when it is really an artifact of the model.
    const coh = $('chCoherence');
    coh.innerHTML = 'coherence <b>' + h.coherence_index.toFixed(2) + '</b>'
      + (h.semantic ? '' : ' <span class="qual">·&nbsp;?</span>');
    coh.className = 'chip' + (h.semantic ? '' : ' warn');
    coh.title = h.semantic
      ? 'mean pairwise cosine across the corpus'
      : 'mean pairwise cosine — inflated and not meaningful under a non-semantic embedder';
    $('chFailures').innerHTML = 'failures <b>' + h.failures + '</b>';
    $('helpPaths').innerHTML = 'ontology edits, corpus nodes and quality history are JSON files under <code>'
      + esc(h.data_dir) + '</code>';
    setStatus('engine ready · ' + h.cells + ' populated cells');
  } catch (e) {
    setStatus('engine unreachable: ' + e.message, true);
  }
}

/* ── classify ─────────────────────────────────────────────────── */
const SAMPLES = [
  'first layer adhesion failed on the nozzle after the filament change',
  'invoice reconciliation complete for March, two line items disputed',
  'monitor vibration on the press during startup and log the peaks',
  'drafted the onboarding guide for the new maintenance technician',
  'the pump seal was replaced during the planned shutdown window',
];

const doClassify = guard('classify', async () => {
  const text = $('classifyText').value.trim();
  if (!text) { toast('enter some text first', true); return; }
  const r = await api('/api/classify', { text });
  const best = r.best_entry;
  $('classifyMeta').textContent = r.query_embedding_dim + 'd vector'
    + (best ? ' · nearest entry ' + best.similarity.toFixed(3) + ' (' + best.domain + '×' + best.mode + ')' : '');

  const wrap = $('results');
  wrap.innerHTML = '';
  if (!r.results.length) {
    wrap.innerHTML = '<div class="muted">No populated cells scored — add ontology entries first.</div>';
  }
  const top = r.results.length ? r.results[0].adjusted_score : 1;
  r.results.forEach((res, i) => {
    const pct = Math.max(2, Math.round((res.adjusted_score / (top || 1)) * 100));
    const moved = Math.abs(res.adjusted_score - res.raw_score) > 0.001;
    const div = document.createElement('div');
    div.className = 'res' + (i === 0 ? ' top' : '');
    div.innerHTML =
      '<div class="head">'
      + '<span class="cell">' + esc(res.domain) + ' × ' + esc(res.mode) + '</span>'
      + (moved ? '<span class="tag" title="raw score before quality weighting">raw ' + res.raw_score.toFixed(3) + '</span>' : '')
      + '<span class="score">' + res.adjusted_score.toFixed(3) + '</span>'
      + '<button class="small good" title="right — boost this cell">✓</button>'
      + '<button class="small bad" title="wrong — penalize this cell">✕</button>'
      + '</div>'
      + '<div class="bar"><i style="width:' + pct + '%"></i></div>'
      + '<div class="meta">'
      + res.entries.map(e => '<span class="tag click" data-entry="' + esc(e) + '">' + esc(e) + '</span>').join('')
      + res.facets.map(f => '<span class="tag facet">' + esc(f) + '</span>').join('')
      + '</div>';
    const [good, bad] = div.querySelectorAll('button');
    const key = res.domain + CELL_SEP + res.mode;
    good.onclick = guard('boost', async () => {
      await api('/api/quality/pass', { cell: key });
      toast('boosted ' + res.domain + ' × ' + res.mode);
      loadHealth(); doClassify();
    });
    bad.onclick = guard('penalize', async () => {
      await api('/api/quality/fail', { feedback: 'wrong cell for: ' + text.slice(0, 120) });
      toast('penalized — re-scoring');
      loadHealth(); doClassify();
    });
    div.querySelectorAll('[data-entry]').forEach(t => {
      t.onclick = () => { showTab('ontology'); openEdit(t.dataset.entry); };
    });
    wrap.appendChild(div);
  });

  const rec = $('recall');
  rec.innerHTML = r.recall.length ? '' : '<div class="muted">No corpus nodes yet — scan a directory in the Corpus tab.</div>';
  r.recall.forEach(h => rec.appendChild(nodeItem(h, true)));
  setStatus('classified ' + text.length + ' chars against ' + r.results.length + ' cells');
});

$('btnClassify').onclick = doClassify;
$('btnClear').onclick = () => { $('classifyText').value = ''; $('results').innerHTML = ''; $('classifyMeta').textContent = ''; };
$('btnSample').onclick = loadSample;
function loadSample() {
  $('classifyText').value = SAMPLES[Math.floor(Math.random() * SAMPLES.length)];
  showTab('classify');
  doClassify();
}

/* ── grid ─────────────────────────────────────────────────────── */
let gridCache = null;

// The axes come from the ontology, not from a fixed 5×14 block: any domain or
// mode a config, an edited entry or a promoted proposal introduced gets its own
// row/column, flagged as an extra.
const loadGrid = guard('grid', async () => {
  const g = await api('/api/grid');
  gridCache = g;
  const axisLabel = a => '<span class="' + (a.extra ? 'extra' : '') + '" title="'
    + esc(a.name) + ' · ' + a.total + ' entries' + (a.extra ? ' · beyond the built-in axes' : '') + '">'
    + esc(a.name) + (a.extra ? ' ✦' : '') + '</span>';

  let html = '<table class="grid"><tr><th></th>' + g.modes.map(m => '<th>' + axisLabel(m) + '</th>').join('') + '</tr>';
  g.domains.forEach((d, di) => {
    html += '<tr><td class="rowlabel">' + axisLabel(d) + '</td>';
    g.modes.forEach((m, mi) => {
      const cell = g.cells[di * g.modes.length + mi];
      html += '<td class="cell" style="background:' + colorFor(cell.count) + ';color:' + (cell.count > 12 ? '#06121c' : '#f1f5ff')
        + '" data-domain="' + esc(d.name) + '" data-mode="' + esc(m.name) + '">' + cell.count + '</td>';
    });
    html += '</tr>';
  });
  $('gridwrap').innerHTML = html + '</table>';
  $('gridwrap').querySelectorAll('td.cell').forEach(td => {
    td.onclick = () => selectCell(td.dataset.domain, td.dataset.mode);
  });
  const extras = g.domains.filter(a => a.extra).length + g.modes.filter(a => a.extra).length;
  $('gridMeta').textContent = g.domains.length + ' domains × ' + g.modes.length + ' modes · '
    + g.cells.filter(c => c.count).length + ' populated'
    + (extras ? ' · ' + extras + ' axis(es) ✦ beyond the built-in grid' : '');
  if (state.selectedCell) selectCell(state.selectedCell.domain, state.selectedCell.mode);
});

function colorFor(n) {
  if (n === 0) return '#101a33';
  if (n <= 2) return '#153055';
  if (n <= 6) return '#1d5285';
  if (n <= 12) return '#2f86c7';
  return '#68e7ff';
}

function selectCell(d, m) {
  state.selectedCell = { domain: d, mode: m };
  $('gridwrap').querySelectorAll('td.cell').forEach(td => {
    td.classList.toggle('sel', td.dataset.domain === d && td.dataset.mode === m);
  });
  const cell = gridCache && gridCache.cells.find(c => c.domain === d && c.mode === m);
  $('cellTitle').textContent = d + ' × ' + m + ' · ' + (cell ? cell.entries.length : 0) + ' entries';
  const box = $('cellentries');
  if (!cell || !cell.entries.length) {
    box.innerHTML = '<div class="muted">Empty cell — a blind spot. Nothing here can ever be classified as '
      + esc(d) + ' × ' + esc(m) + '.</div>';
    return;
  }
  box.innerHTML = cell.entries.map(e => '<span class="tag click" data-entry="' + esc(e) + '">' + esc(e) + '</span>').join(' ');
  box.querySelectorAll('[data-entry]').forEach(t => { t.onclick = () => openEdit(t.dataset.entry); });
}

$('btnAdd').onclick = () => {
  const c = state.selectedCell || { domain: 'STUDY', mode: 'WORK' };
  openEdit(null, c.domain, c.mode);
};
$('btnNew').onclick = () => openEdit(null);

/* ── ontology browser ─────────────────────────────────────────── */
const loadOntology = guard('ontology', async () => {
  const o = await api('/api/ontology');
  $('ontMeta').textContent = o.entry_count + ' entries · ' + o.custom_count + ' edited here';
  const q = $('search').value.toLowerCase();
  const wrap = $('cats');
  wrap.innerHTML = '';
  let shown = 0;
  for (const cat of o.categories) {
    const entries = cat.entries.filter(e => !q
      || e.name.toLowerCase().includes(q)
      || (e.domain + '×' + e.mode).toLowerCase().includes(q)
      || e.hints.some(h => h.toLowerCase().includes(q)));
    if (!entries.length) continue;
    shown += entries.length;
    const div = document.createElement('div');
    div.className = 'cat';
    div.innerHTML = '<div class="name">' + esc(cat.name) + ' · ' + entries.length + '</div>';
    for (const e of entries) {
      const row = document.createElement('div');
      row.className = 'entryrow';
      row.innerHTML = '<span>' + esc(e.name) + (e.custom ? ' <span class="tag">edited</span>' : '') + '</span>'
        + '<span class="tag">' + esc(e.domain) + '×' + esc(e.mode) + '</span>';
      row.title = e.hints.join(', ');
      row.onclick = () => openEdit(e.name);
      div.appendChild(row);
    }
    wrap.appendChild(div);
  }
  if (!shown) wrap.innerHTML = '<div class="muted">No entries match “' + esc(q) + '”.</div>';
});
$('search').addEventListener('input', loadOntology);

/* ── corpus ───────────────────────────────────────────────────── */
function tile(v, l, title) {
  return '<div class="tile"' + (title ? ' title="' + esc(title) + '"' : '') + '>'
    + '<div class="v">' + v + '</div><div class="l">' + l + '</div></div>';
}

function renderSnapshot(s) {
  // asserted_index is a mean over the JUDGED nodes only. One failed verdict out
  // of 83 nodes renders as "-1.00", which reads as "the whole corpus is
  // failing" unless the denominator is on screen next to it. Same for the
  // coherence tiles, which say nothing about meaning under a coarse embedder.
  const judged = s.asserted_success + s.asserted_inert + s.asserted_failure;
  const idx = s.asserted_index === null || s.asserted_index === undefined
    ? '—'
    : s.asserted_index.toFixed(2);

  $('snapTiles').innerHTML =
    tile(s.total_nodes, 'nodes')
    + tile(s.coherence_index.toFixed(3), 'coherence index',
        'mean pairwise cosine across the corpus')
    + tile(s.high_coherence + ' / ' + s.mid_coherence + ' / ' + s.low_coherence, 'high / mid / low',
        'how densely each node sits among its neighbours — derived, not judged')
    + tile(s.asserted_success + ' / ' + s.asserted_inert + ' / ' + s.asserted_failure, 'worked / inert / failed',
        'your verdicts — the axis dreaming replays')
    + tile(
        judged === 0 ? '—' : idx + ' <span class="of">n=' + judged + '</span>',
        judged === 0 ? 'asserted index · none judged'
                     : 'asserted index · ' + judged + ' of ' + s.total_nodes + ' judged',
        'Mean of your verdicts, over judged nodes only. Unjudged nodes are not counted, '
          + 'so a single verdict swings this to its extreme.')
    + tile(s.dream_cycle_count, 'dreams');
}

const loadSnapshot = guard('snapshot', async () => renderSnapshot(await api('/api/core/snapshot')));

// One node row, reused by recall, search and the judge queue.
function nodeItem(n, compact) {
  const div = document.createElement('div');
  const cls = n.asserted === null || n.asserted === undefined ? 'warn' : (n.asserted > 0 ? 'good' : (n.asserted < 0 ? 'bad' : ''));
  div.className = 'item ' + cls;
  div.innerHTML = '<div class="l1">'
    + '<span class="s">' + n.score.toFixed(3) + '</span>'
    + '<span class="tag">density ' + n.coherence.toFixed(2) + '</span>'
    + (n.asserted === null || n.asserted === undefined
      ? '<span class="tag">unjudged</span>'
      : '<span class="tag">' + (n.asserted > 0 ? 'worked' : n.asserted < 0 ? 'failed' : 'inert') + '</span>')
    + '</div><div class="txt">' + esc(n.label) + '</div>';
  if (!compact) {
    const row = document.createElement('div');
    row.className = 'row tight';
    row.style.marginTop = '7px';
    [['success', '✓ worked', 'good'], ['inert', '– inert', ''], ['failure', '✕ failed', 'bad']].forEach(([v, label, kind]) => {
      const b = document.createElement('button');
      b.className = 'small ' + kind;
      b.textContent = label;
      b.onclick = guard('assert', async () => {
        const r = await api('/api/core/assert', { id: n.id, verdict: v });
        renderSnapshot(r.snapshot);
        toast('recorded: ' + label.slice(2));
        loadNodes(); loadHealth();
      });
      row.appendChild(b);
    });
    div.appendChild(row);
  }
  return div;
}

const loadNodes = guard('nodes', async () => {
  const r = await api('/api/core/nodes?filter=' + encodeURIComponent($('nodeFilter').value) + '&max=30');
  const box = $('nodeList');
  box.innerHTML = r.nodes.length ? '' : '<div class="muted">Nothing in this queue.</div>';
  r.nodes.forEach(n => box.appendChild(nodeItem(n, false)));
});
$('nodeFilter').onchange = loadNodes;
$('btnReloadNodes').onclick = loadNodes;

$('btnScan').onclick = guard('scan', async () => {
  const dir = $('scandir').value.trim();
  if (!dir) { toast('enter a directory path', true); return; }
  const btn = $('btnScan');
  btn.disabled = true; btn.textContent = 'Scanning…';
  try {
    const r = await api('/api/core/scan', { dir });
    $('scanStats').textContent = r.files + ' text files · ' + r.registered + ' passages registered'
      + (r.skipped ? ' · ' + r.skipped + ' file(s) skipped (empty or unreadable)' : '');
    renderSnapshot(r.snapshot);
    loadNodes(); loadHealth();
    toast('scanned ' + r.registered + ' documents');
  } finally {
    btn.disabled = false; btn.textContent = 'Scan';
  }
});

const corpusSearch = guard('search', async () => {
  const query = $('corpusQuery').value.trim();
  if (!query) { toast('enter a query', true); return; }
  const r = await api('/api/core/search', { query, max: 12 });
  const box = $('corpusHits');
  box.innerHTML = r.hits.length ? '' : '<div class="muted">No labelled nodes yet — scan a directory first.</div>';
  r.hits.forEach(h => box.appendChild(nodeItem(h, true)));
});
$('btnCorpusSearch').onclick = corpusSearch;
$('corpusQuery').addEventListener('keydown', e => { if (e.key === 'Enter') corpusSearch(); });

$('btnDream').onclick = guard('dream', async () => {
  const r = await api('/api/core/dream', {});
  renderSnapshot(r.snapshot);
  const box = $('dreamOut');
  if (!r.dreams.length) {
    box.innerHTML = '<div class="muted">Nothing to replay — every weak or failed node has already been dreamt.</div>';
  } else {
    box.innerHTML = '<div class="muted" style="margin-bottom:6px">' + r.dreams.length + ' replay(s)</div>'
      + r.dreams.map(d => '<div class="item ' + (d.prevented_failure ? 'bad' : 'good') + '">'
        + '<div class="l1"><span class="s">' + d.outcome.toFixed(1) + '</span>'
        + '<span class="tag">' + (d.prevented_failure ? 'prevented failure' : 'held up') + '</span>'
        + '<span class="tag">Δ ' + d.coherence_delta.toFixed(2) + '</span></div></div>').join('');
  }
  toast(r.dreams.length + ' node(s) replayed');
  loadHealth();
});

/* ── discover ─────────────────────────────────────────────────── */
/* ── structure (Demo A) ───────────────────────────────── */
$('btnMapRun').onclick = guard('structure map', async () => {
  const dir = ($('mapdir').value || 'examples/demo-corpus').trim();
  const btn = $('btnMapRun');
  btn.disabled = true; btn.textContent = 'Mapping…';
  try {
    const r = await api('/api/map', { dir });
    $('mapTiles').innerHTML =
      tile(r.docs, 'documents')
      + tile(r.repeats.length, 'patterns')
      + tile(r.differences.length, 'differences')
      + tile(r.contradictions.length, 'contradictions');
    const rep = $('mapRepeats');
    rep.innerHTML = '';
    (r.repeats || []).forEach(c => {
      const d = document.createElement('div');
      d.className = 'card';
      d.innerHTML = '<b>' + esc(c.name) + '</b> <span class="muted">' + c.count + ' instances · ' + esc((c.hints || []).join(', ')) + '</span>'
        + '<div class="muted">' + c.instances.slice(0, 8).map(x => esc(x.title)).join(', ') + '</div>';
      rep.appendChild(d);
    });
    if (!(r.repeats || []).length) rep.innerHTML = '<div class="muted">Nothing repeats at corpus scale here.</div>';
    const dif = $('mapDiffs');
    dif.innerHTML = '';
    (r.differences || []).slice(0, 8).forEach(x => {
      dif.innerHTML += '<div>• <b>' + esc(x.title) + '</b> <span class="muted">separation ' + (x.separation || 0).toFixed(2) + ' · nearest ' + esc(x.nearest_cluster || 'none') + '</span></div>';
    });
    (r.contradictions || []).slice(0, 6).forEach(p => {
      dif.innerHTML += '<div>⚠ <b>' + esc(p.a.title) + '</b> vs <b>' + esc(p.b.title) + '</b> <span class="muted">[' + esc(p.cluster) + ']</span></div>';
    });
    if (!(r.differences || []).length && !(r.contradictions || []).length)
      dif.innerHTML = '<div class="muted">No differences, no contradictions — a perfectly uniform corpus.</div>';
    $('mapHash').textContent = 'structure hash ' + (r.structure_hash || '').slice(0, 12)
      + ' · threshold ' + (r.threshold_used != null ? r.threshold_used.toFixed(2) : '?')
      + (r.auto_retuned ? ' (auto-retuned)' : '');
    toast(r.repeats.length + ' pattern(s), ' + r.differences.length + ' difference(s)');
  } catch (e) { toast('map failed: ' + e.message, true); }
  btn.disabled = false; btn.textContent = 'Run map';
});

$('btnIngest').onclick = guard('gap scan', async () => {
  const dir = $('ingestdir').value.trim();
  if (!dir) { toast('enter a directory path', true); return; }
  const btn = $('btnIngest');
  btn.disabled = true; btn.textContent = 'Scanning…';
  try {
    const r = await api('/api/ingest', { dir });
    const rep = r.report;
    $('ingestTiles').innerHTML =
      tile(r.files, 'files')
      + tile(r.lines, 'lines')
      + tile(rep.covered + ' / ' + rep.total, 'covered')
      + tile(rep.unmapped, 'unmapped')
      + tile(rep.threshold_used.toFixed(2) + (rep.auto_retuned ? '*' : ''), 'threshold' + (rep.auto_retuned ? ' (retuned)' : ''))
      + tile(rep.sim_mean.toFixed(2), 'mean similarity');
    const box = $('proposals');
    box.innerHTML = '';
    if (!rep.proposals.length) {
      box.innerHTML = '<div class="muted">No gaps — this corpus is already covered by the ontology.</div>';
    }
    rep.proposals.forEach(p => {
      const d = document.createElement('div');
      d.className = 'item warn';
      d.innerHTML = '<div class="l1"><b>' + esc(p.name) + '</b>'
        + '<span class="tag">' + esc(p.domain) + '×' + esc(p.mode) + '</span>'
        + '<span class="tag">' + p.count + ' texts</span>'
        + '<span class="tag">cov ' + p.coverage.toFixed(2) + '</span></div>'
        + '<div class="txt">hints: ' + esc(p.hints.join(', ')) + '</div>'
        + '<div class="txt">' + esc(p.samples.slice(0, 2).join(' | ')) + '</div>';
      const b = document.createElement('button');
      b.className = 'small';
      b.style.marginTop = '7px';
      b.textContent = 'Promote to entry';
      b.onclick = guard('promote', async () => {
        await api('/api/ingest/promote', {
          name: p.name, category: 'Discovered', domain: p.domain, mode: p.mode,
          hints: p.hints, samples: p.samples,
        });
        toast('promoted ' + p.name + ' — classifier rebuilt');
        d.remove();
        loadHealth();
      });
      d.appendChild(b);
      box.appendChild(d);
    });
    setStatus('gap scan: ' + rep.unmapped + ' unmapped of ' + rep.total);
  } finally {
    btn.disabled = false; btn.textContent = 'Find gaps';
  }
});

/* ── quality ──────────────────────────────────────────────────── */
const loadQuality = guard('quality', async () => {
  const q = await api('/api/quality');
  const weights = $('weights');
  const rows = [];
  for (const [cell, p] of Object.entries(q.penalties)) rows.push([cell, -p]);
  for (const [cell, b] of Object.entries(q.boosts)) rows.push([cell, b]);
  rows.sort((a, b) => Math.abs(b[1]) - Math.abs(a[1]));
  weights.innerHTML = rows.length ? '' : '<div class="muted">No cell has been judged yet.</div>';
  rows.forEach(([cell, w]) => {
    const div = document.createElement('div');
    div.className = 'item ' + (w < 0 ? 'bad' : 'good');
    div.innerHTML = '<div class="l1"><span class="s">' + (w > 0 ? '+' : '') + w.toFixed(2) + '</span>'
      + '<span class="cell">' + esc(cellLabel(cell)) + '</span></div>';
    const b = document.createElement('button');
    b.className = 'small good';
    b.textContent = '✓ it was right';
    b.style.marginTop = '7px';
    b.onclick = guard('boost', async () => {
      await api('/api/quality/pass', { cell });
      toast('boosted ' + cellLabel(cell));
      loadQuality(); loadHealth();
    });
    div.appendChild(b);
    weights.appendChild(div);
  });

  const list = $('qlist');
  list.innerHTML = q.failures.length ? '' : '<div class="muted">No failures recorded.</div>';
  q.failures.forEach(f => {
    const div = document.createElement('div');
    div.className = 'item bad';
    div.innerHTML = '<div class="l1"><span class="cell">' + esc(cellLabel(f.top_cell)) + '</span>'
      + '<span class="tag">sev ' + f.severity.toFixed(1) + '</span>'
      + '<span class="tag">' + esc(f.timestamp.slice(5, 16).replace('T', ' ')) + '</span></div>'
      + '<div class="txt">' + esc(f.feedback) + '</div>';
    list.appendChild(div);
  });
});

$('btnFail').onclick = guard('record failure', async () => {
  const text = $('failtext').value.trim();
  if (!text) { toast('describe what went wrong', true); return; }
  const r = await api('/api/quality/fail', { feedback: text });
  $('failtext').value = '';
  toast('recorded → ' + cellLabel(r.top_cell));
  loadQuality(); loadHealth();
});

/* ── entry editor ─────────────────────────────────────────────── */
// Collect the facet controls into a Facets object. Empty selections are
// OMITTED rather than sent as "": the Rust side parses each facet as an enum,
// and "" is not a variant — sending it would 422 the whole entry.
function facetsFromForm() {
  const f = {};
  const put = (key, id) => { const v = $(id).value.trim(); if (v) f[key] = v; };
  put('lifecycle', 'mLifecycle'); put('agency', 'mAgency');
  put('scale', 'mScale'); put('abstraction', 'mAbstraction');
  put('sub_domain', 'mSubDomain'); put('sub_mode', 'mSubMode');
  return f;
}

// Fallback axes for the very first paint, before /api/grid has answered. The
// live lists come from the ontology — see loadGrid — and the editor takes free
// text, so a genuinely new domain or mode can be introduced right here.
const DOMAINS = ['HEAL', 'CONSTRUCT', 'FABRICATE', 'BOND', 'STUDY'];
const MODES = ['LIFT', 'REST', 'WALK', 'WORK', 'CREATE', 'LEARN', 'DESTROY', 'SENSE', 'GUIDE', 'PLAY', 'BRAINSTORM', 'MAINTAIN', 'MOVE', 'PLAN'];

function axisNames(kind) {
  const fallback = kind === 'domains' ? DOMAINS : MODES;
  if (!gridCache) return fallback;
  return gridCache[kind].map(a => a.name);
}

// Keep the editor's suggestions honest even if the Grid tab was never opened.
function ensureAxes() {
  if (!gridCache) loadGrid();
}

function datalist(id, values) {
  return '<datalist id="' + id + '">' + values.map(v => '<option value="' + esc(v) + '">').join('') + '</datalist>';
}

function openEdit(name, domain, mode) {
  ensureAxes();
  state.editingName = name;
  const d = domain || 'STUDY';
  const m = mode || 'WORK';
  $('modal').innerHTML = `<h3>${name ? 'Edit entry' : 'New entry'}</h3>
    <p class="muted" style="margin:0 0 10px;font-size:12px">Hints are what the entry is embedded from — write the words a real document would use.</p>
    <label>Name</label><input type="text" id="mName" value="${esc(name || '')}" placeholder="e.g. invoice line check">
    <label>Category</label><input type="text" id="mCat" placeholder="e.g. Finance">
    ${datalist('dlDomains', axisNames('domains'))}${datalist('dlModes', axisNames('modes'))}
    <div class="g2">
      <div><label>Domain</label><input type="text" id="mDomain" list="dlDomains" value="${esc(d)}" placeholder="HEAL · CONSTRUCT · or a new one"></div>
      <div><label>Mode</label><input type="text" id="mMode" list="dlModes" value="${esc(m)}" placeholder="WORK · MAINTAIN · or a new one"></div>
    </div>
    <p class="muted" style="margin:0;font-size:11px">Type a domain or mode that doesn't exist yet and the grid grows a row or column for it.</p>
    <div class="g2">
      <div><label>Axis kind</label><input type="text" id="mAxisKind" placeholder="e.g. epistemic"></div>
      <div><label>Axis name</label><input type="text" id="mAxisName" placeholder="e.g. formal"></div>
    </div>
    <div class="g2">
      <div><label>Unit</label><input type="text" id="mUnit" placeholder="e.g. checks"></div>
      <div><label>Hints (comma separated)</label><input type="text" id="mHints" placeholder="reconcile, line item, payment"></div>
    </div>

    <label style="margin-top:14px">Facets · the dimensions the 2-D grid cannot hold</label>
    <!-- option values are the serde variant names of physis-core's facet
         enums; the label may read differently (Self → SelfActor) but the
         value must match or the entry fails to deserialize. -->
    <div class="g2">
      <div><label>Lifecycle</label><select id="mLifecycle">
        <option value="">—</option>
        <option value="Design">Design</option>
        <option value="Build">Build</option>
        <option value="Operate">Operate</option>
        <option value="Retire">Retire</option>
      </select></div>
      <div><label>Agency</label><select id="mAgency">
        <option value="">—</option>
        <option value="SelfActor">Self</option>
        <option value="Other">Other</option>
        <option value="Automated">Automated</option>
        <option value="Collective">Collective</option>
      </select></div>
    </div>
    <div class="g2">
      <div><label>Scale</label><select id="mScale">
        <option value="">—</option>
        <option value="Personal">Personal</option>
        <option value="Interpersonal">Interpersonal</option>
        <option value="Organizational">Organizational</option>
        <option value="Civil">Civil</option>
      </select></div>
      <div><label>Abstraction</label><select id="mAbstraction">
        <option value="">—</option>
        <option value="Concrete">Concrete</option>
        <option value="Abstract">Abstract</option>
      </select></div>
    </div>
    <div class="g2">
      <div><label>Sub-domain</label><input type="text" id="mSubDomain" placeholder="e.g. Repair"></div>
      <div><label>Sub-mode</label><input type="text" id="mSubMode" placeholder="e.g. Preventive"></div>
    </div>

    <div class="row" style="margin-top:16px">
      <button class="primary" id="mSave">Save</button>
      ${name ? '<button class="bad" id="mDel">Delete</button>' : ''}
      <button class="ghost" id="mCancel">Cancel</button>
    </div>`;
  $('modalBg').classList.add('show');
  $('mName').focus();

  if (name) {
    // Prefill from the ontology, facets included — a save writes the whole
    // DomainDef, so an unfilled control would silently blank a stored facet.
    api('/api/ontology').then(o => {
      for (const cat of o.categories) {
        for (const e of cat.entries) {
          if (e.name !== name) continue;
          $('mCat').value = e.category || '';
          $('mDomain').value = e.domain;
          $('mMode').value = e.mode;
          $('mAxisKind').value = e.axis_kind;
          $('mAxisName').value = e.axis_name;
          $('mUnit').value = e.unit;
          $('mHints').value = e.hints.join(', ');
          const f = e.facets || {};
          $('mLifecycle').value = f.lifecycle || '';
          $('mAgency').value = f.agency || '';
          $('mScale').value = f.scale || '';
          $('mAbstraction').value = f.abstraction || '';
          $('mSubDomain').value = f.sub_domain || '';
          $('mSubMode').value = f.sub_mode || '';
        }
      }
    });
  }

  const close = () => { $('modalBg').classList.remove('show'); state.editingName = null; };
  $('mCancel').onclick = close;
  $('mSave').onclick = guard('save', async () => {
    // Every upsert re-embeds the whole ontology, which takes about a second —
    // without this the modal just sits there looking ignored.
    const btn = $('mSave');
    btn.disabled = true;
    btn.textContent = 'Rebuilding classifier…';
    try {
      await api('/api/ontology/upsert', {
        name: $('mName').value.trim(), category: $('mCat').value.trim(),
        // Axes are case-sensitive cell keys; normalizing here stops "exchange"
        // and "EXCHANGE" becoming two rows of the same grid.
        domain: $('mDomain').value.trim().toUpperCase(), mode: $('mMode').value.trim().toUpperCase(),
        axis_kind: $('mAxisKind').value, axis_name: $('mAxisName').value,
        unit: $('mUnit').value, hints: $('mHints').value,
        facets: facetsFromForm(),
      });
    } finally {
      btn.disabled = false;
      btn.textContent = 'Save';
    }
    close();
    toast('saved — classifier rebuilt');
    refreshAll();
  });
  if (name) $('mDel').onclick = guard('delete', async () => {
    await api('/api/ontology/delete', { name });
    close();
    toast('deleted — classifier rebuilt');
    refreshAll();
  });
}
$('modalBg').addEventListener('click', e => {
  if (e.target === $('modalBg')) { $('modalBg').classList.remove('show'); state.editingName = null; }
});

/* ── help + shortcuts ─────────────────────────────────────────── */
$('btnHelp').onclick = () => $('help').classList.toggle('show');
$('helpClose').onclick = () => $('help').classList.remove('show');
$('help').addEventListener('click', e => { if (e.target === $('help')) $('help').classList.remove('show'); });

document.addEventListener('keydown', e => {
  if (e.key === 'Escape') {
    $('help').classList.remove('show');
    $('modalBg').classList.remove('show');
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault();
    if (state.tab === 'coherence') cohCheck();
    else doClassify();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
    e.preventDefault();
    const focusTarget = {
      classify: 'classifyText', ontology: 'search', corpus: 'corpusQuery',
      discover: 'ingestdir', quality: 'failtext',
    }[state.tab];
    if (focusTarget) $(focusTarget).focus();
    return;
  }
  if (typing() || e.ctrlKey || e.metaKey || e.altKey) return;
  if (e.key === '?') { e.preventDefault(); $('help').classList.toggle('show'); return; }
  if (e.key === 's' || e.key === 'S') { e.preventDefault(); loadSample(); return; }
  const n = parseInt(e.key, 10);
  if (n >= 1 && n <= TABS.length) showTab(TABS[n - 1]);
});

/* ── edition / pro ────────────────────────────────────────────── */
// Core's UI is the only place a Core user is looking, so it is the only place
// that can tell them Pro exists — or that they already have it installed. The
// server answers from the same detection the `physis` front door uses, so this
// panel and the CLI banner cannot disagree.
const loadEdition = guard('edition', async () => {
  const e = await api('/api/edition');

  $('editionTitle').textContent = e.has_pro ? 'Core + Pro' : 'Upgrade to Pro';
  $('editionLead').textContent = e.has_pro
    ? 'Both editions are installed. Core keeps the graph; Pro adds the operations layer on top of it.'
    : 'You are running the open Core engine. Pro is a separate, licensed product that '
      + 'builds on this same graph — nothing you have made here needs migrating.';

  // The nav label doubles as the signal, so the state is legible without
  // opening the tab.
  const nav = $('navEdition');
  nav.innerHTML = (e.has_pro ? 'Pro' : 'Upgrade') + ' <span class="k">7</span>';
  nav.classList.toggle('accent', !e.has_pro);

  // Not `state`: that name is the module-wide UI state object.
  const box = $('editionState');
  if (e.has_pro) {
    const rows = [];
    if (e.pro_cli) rows.push(['physis-pro', e.pro_cli, 'CLI — doctor, timeline, report, connectors']);
    if (e.pro_web) rows.push(['physis-pro-web', e.pro_web, 'Operations Console, Demo and Study dashboards']);
    box.innerHTML = rows.map(([name, path, what]) =>
      '<div class="item good">'
      + '<div class="l1"><span class="cell">' + esc(name) + '</span>'
      + '<span class="tag">installed</span></div>'
      + '<div class="txt">' + esc(what) + '</div>'
      + '<div class="txt"><code>' + esc(path) + '</code></div>'
      + '</div>').join('')
      + '<div class="muted" style="margin-top:10px">Start the dashboards with '
      + '<code>physis web</code>, then open <code>/ui</code>.</div>';
  } else {
    box.innerHTML =
      '<div class="item warn">'
      + '<div class="l1"><span class="cell">physis-pro</span>'
      + '<span class="tag">not installed</span></div>'
      + '<div class="txt">Core stays open and keeps working without it.</div>'
      + '</div>'
      + '<div class="muted" style="margin-top:10px">Where to get it: '
      + '<a href="' + esc(e.upgrade_url) + '" target="_blank" rel="noopener noreferrer">'
      + esc(e.upgrade_url) + '</a></div>';
  }

  $('editionAdds').innerHTML = (e.adds || []).map(a =>
    '<div class="item">'
    + '<div class="l1"><span class="cell">' + esc(a.title) + '</span></div>'
    + '<div class="txt">' + esc(a.detail) + '</div>'
    + '</div>').join('');
});

/* ══ LAB — embedding-space map ═════════════════════════════════ */
const DOMAIN_COLORS = ['#68e7ff', '#b49cff', '#72f0b2', '#ffc66d', '#ff718d',
  '#5ad0ff', '#9d8cff', '#63e6c0', '#ffb35a', '#f76ea6', '#7aa7ff', '#8ef2e2'];
const domainColor = (() => {
  const cache = {};
  let next = 0;
  return d => {
    if (!(d in cache)) cache[d] = DOMAIN_COLORS[(next++) % DOMAIN_COLORS.length];
    return cache[d];
  };
})();

const lab = {
  loaded: false, entries: [], corpus: [], probe: null,
  view: { x: 0, y: 0, k: 1 },   // pan offset + zoom
  hover: null,
};

function labCanvas() { return $('labMap'); }

function labResize() {
  const c = labCanvas();
  const wrap = $('mapwrap');
  c.width = wrap.clientWidth;
  c.height = Math.max(430, Math.min(560, window.innerHeight - 400));
  labDraw();
}

function loadLabMap() {
  return guard('lab map', async () => {
    const m = await api('/api/lab/map');
    lab.entries = m.entries;
    lab.corpus = m.corpus;
    lab.loaded = true;
    $('labMeta').textContent = m.entries.length + ' entries · ' + m.corpus.length + ' corpus nodes';
    labResize();
  })();
}

// Data coords are PCA outputs; map them into canvas space. X and Y scale
// independently (aspect-fill) — a semiotic map reads better filling a wide
// panel than letterboxed inside a square. Points beyond the extent (outliers)
// clamp to the frame instead of squishing everything else.
function labToScreen(x, y) {
  const c = labCanvas();
  const pad = 36;
  const extent = lab.extent || 1;
  const cx = Math.max(-extent, Math.min(extent, x));
  const cy = Math.max(-extent, Math.min(extent, y));
  const kx = (c.width / 2 - pad) / extent * lab.view.k;
  const ky = (c.height / 2 - pad) / extent * lab.view.k;
  return [
    c.width / 2 + cx * kx + lab.view.x,
    c.height / 2 - cy * ky + lab.view.y,
  ];
}

function labComputeExtent() {
  const all = [];
  for (const p of [...lab.entries, ...lab.corpus]) {
    all.push(Math.abs(p.x), Math.abs(p.y));
  }
  all.sort((a, b) => a - b);
  // 95th percentile: a handful of distant outliers must not shrink the
  // neighbourhoods that matter into a blob.
  const p95 = all[Math.floor(all.length * 0.95)] || 0.1;
  lab.extent = Math.max(0.05, p95 * 1.12);
}

function labDraw() {
  const c = labCanvas();
  if (!c.width || !lab.loaded) return;
  if (!lab.extent) labComputeExtent();
  document.body.dataset.labdbg = c.width + 'x' + c.height
    + ' ext=' + lab.extent.toFixed(3)
    + ' n=' + lab.entries.length + '/' + lab.corpus.length;
  const ctx = c.getContext('2d');
  ctx.clearRect(0, 0, c.width, c.height);

  // faint radial backdrop
  const g = ctx.createRadialGradient(c.width / 2, c.height / 2, 10, c.width / 2, c.height / 2, Math.max(c.width, c.height) / 2);
  g.addColorStop(0, 'rgba(104,231,255,.05)');
  g.addColorStop(1, 'rgba(0,0,0,0)');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, c.width, c.height);

  // corpus diamonds underneath
  ctx.lineWidth = 1;
  for (const p of lab.corpus) {
    const [sx, sy] = labToScreen(p.x, p.y);
    const r = 4;
    const v = p.verdict;
    ctx.strokeStyle = v == null ? 'rgba(137,151,184,.55)'
      : v > 0 ? 'rgba(114,240,178,.75)' : 'rgba(255,113,141,.75)';
    ctx.beginPath();
    ctx.moveTo(sx, sy - r); ctx.lineTo(sx + r, sy); ctx.lineTo(sx, sy + r); ctx.lineTo(sx - r, sy);
    ctx.closePath();
    ctx.stroke();
  }

  // ontology entries as glowing points
  for (const e of lab.entries) {
    const [sx, sy] = labToScreen(e.x, e.y);
    const col = domainColor(e.domain);
    ctx.fillStyle = col;
    ctx.shadowColor = col;
    ctx.shadowBlur = lab.hover && lab.hover.name === e.name ? 16 : 7;
    ctx.beginPath();
    ctx.arc(sx, sy, lab.hover && lab.hover.name === e.name ? 5 : 3, 0, 7);
    ctx.fill();
    ctx.shadowBlur = 0;
  }

  // probe marker: pulsing rings
  if (lab.probe) {
    const [sx, sy] = labToScreen(lab.probe.x, lab.probe.y);
    const t = performance.now() / 600;
    for (let i = 0; i < 2; i++) {
      const ph = (t + i * 0.5) % 1;
      ctx.strokeStyle = `rgba(180,156,255,${(1 - ph) * .8})`;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.arc(sx, sy, 4 + ph * 22, 0, 7);
      ctx.stroke();
    }
    ctx.fillStyle = '#b49cff';
    ctx.shadowColor = '#b49cff'; ctx.shadowBlur = 14;
    ctx.beginPath(); ctx.arc(sx, sy, 4.5, 0, 7); ctx.fill();
    ctx.shadowBlur = 0;
  }
}
// keep probe pulse alive while tab visible
setInterval(() => { if (state.tab === 'ontology' && lab.loaded && lab.probe) labDraw(); }, 60);

function labHit(mx, my) {
  let best = null, bd = 144;
  for (const e of lab.entries) {
    const [sx, sy] = labToScreen(e.x, e.y);
    const d = (sx - mx) ** 2 + (sy - my) ** 2;
    if (d < bd) { bd = d; best = { kind: 'entry', ...e }; }
  }
  for (const p of lab.corpus) {
    const [sx, sy] = labToScreen(p.x, p.y);
    const d = (sx - mx) ** 2 + (sy - my) ** 2;
    if (d < bd) { bd = d; best = { kind: 'corpus', ...p }; }
  }
  if (lab.probe) {
    const [sx, sy] = labToScreen(lab.probe.x, lab.probe.y);
    if ((sx - mx) ** 2 + (sy - my) ** 2 < bd) best = { kind: 'probe', label: lab.probe.text };
  }
  return best;
}

(function labInteract() {
  const c = () => labCanvas();
  document.addEventListener('DOMContentLoaded', () => {
    let dragging = false, moved = false, lx = 0, ly = 0;
    c().addEventListener('mousedown', e => { dragging = true; moved = false; lx = e.offsetX; ly = e.offsetY; });
    window.addEventListener('mouseup', () => { dragging = false; });
    c().addEventListener('mousemove', e => {
      if (dragging) {
        lab.view.x += e.offsetX - lx;
        lab.view.y += e.offsetY - ly;
        lx = e.offsetX; ly = e.offsetY; moved = true;
        labDraw();
        return;
      }
      const hit = labHit(e.offsetX, e.offsetY);
      lab.hover = hit && hit.kind === 'entry' ? hit : null;
      const tip = $('mapTip');
      if (hit) {
        const lines = hit.kind === 'entry'
          ? ['<b>' + esc(hit.name) + '</b>', '<span class="muted">' + esc(hit.domain) + ' × ' + esc(hit.mode) + '</span>']
          : hit.kind === 'corpus'
            ? ['<b>' + esc((hit.label || '').slice(0, 90)) + '</b>', '<span class="muted">node · coherence ' + (+hit.score).toFixed(2) + '</span>']
            : ['<b>' + esc((hit.label || '').slice(0, 90)) + '</b>', '<span class="muted">your probe</span>'];
        tip.innerHTML = lines.join('<br>');
        tip.style.display = 'block';
        tip.style.left = Math.min(e.offsetX + 14, c().clientWidth - 190) + 'px';
        tip.style.top = (e.offsetY + 12) + 'px';
        c().style.cursor = 'pointer';
      } else {
        tip.style.display = 'none';
        c().style.cursor = dragging ? 'grabbing' : 'default';
      }
      labDraw();
    });
    c().addEventListener('click', e => {
      if (moved) return;
      const hit = labHit(e.offsetX, e.offsetY);
      if (!hit) return;
      if (hit.kind === 'entry') openEdit(hit.name);
      else if (hit.kind === 'corpus') toast((hit.label || '').slice(0, 80));
    });
    c().addEventListener('wheel', e => {
      e.preventDefault();
      const k = lab.view.k * (e.deltaY < 0 ? 1.12 : 1 / 1.12);
      lab.view.k = Math.min(14, Math.max(0.4, k));
      labDraw();
    }, { passive: false });
    window.addEventListener('resize', () => { if (state.tab === 'ontology') labResize(); });
  });
})();

const runProbe = guard('probe', async () => {
  const text = $('probeText').value.trim();
  if (!text) { toast('type a probe text first', true); return; }
  const n = await api('/api/lab/neighbors', { text, k: 14 });

  // Project the probe into the same PCA frame: server-side neighbors give us
  // its nearest entries; place the marker at their weighted centroid.
  let px = 0, py = 0, wsum = 0;
  for (const nb of n.neighbors.slice(0, 6)) {
    const pt = lab.entries.find(e => e.name === nb.name);
    if (pt) { px += pt.x * nb.similarity; py += pt.y * nb.similarity; wsum += nb.similarity; }
  }
  lab.probe = wsum > 0 ? { x: px / wsum, y: py / wsum, text } : null;
  labDraw();

  $('neighbors').innerHTML =
    '<div class="muted" style="margin-bottom:8px">' + esc(text.slice(0, 120)) + '</div>'
    + n.neighbors.map(nb => {
      const pct = Math.round(Math.max(0, Math.min(1, (nb.similarity + 0.1))) * 100);
      return '<div class="nbrow" data-entry="' + esc(nb.name) + '">'
        + '<span class="nbname">' + esc(nb.name) + '</span>'
        + '<span class="nbcell">' + esc(nb.domain) + '×' + esc(nb.mode) + '</span>'
        + '<span class="nbbar"><i style="width:' + pct + '%;background:' + domainColor(nb.domain) + '"></i></span>'
        + '<span class="nbsim">' + nb.similarity.toFixed(3) + '</span></div>';
    }).join('');
  $('neighbors').querySelectorAll('[data-entry]').forEach(t => {
    t.onclick = () => openEdit(t.dataset.entry);
  });
});
$('btnProbe').onclick = runProbe;
$('probeText').addEventListener('keydown', e => { if (e.key === 'Enter') runProbe(); });

/* ══ FLOW — live graph + workflows ═════════════════════════════ */
const flow = { nodes: [], edges: [], conflicts: [], workflows: [], paused: false, sim: new Map() };

function flowRelColor(rel) {
  if (rel === 'Supports') return '#72f0b2';
  if (rel === 'Contradicts') return '#ff718d';
  return '#68a8ff';
}

async function startFlow() {
  try {
    const g = await api('/api/flow/graph');
    flow.conflicts = g.conflicts;
    flow.workflows = g.workflows;
    renderWorkflows();
    renderConflicts();

    // Reconcile with existing simulation state so refreshes keep layout.
    const keep = id => flow.sim.get(id);
    flow.nodes = [];
    for (const n of g.nodes) {
      const prev = keep(n.id);
      flow.sim.set(n.id, prev || {
        x: (Math.random() - 0.5) * 300, y: (Math.random() - 0.5) * 240,
        vx: 0, vy: 0,
      });
      flow.nodes.push({ ...n });
    }
    flow.edges = g.edges.filter(e =>
      flow.sim.has(e.source) && flow.sim.has(e.target));
    flowExtentFit();
    if (!flow.raf) flowLoop();
  } catch (e) {
    setStatus('flow load failed: ' + e.message, true);
  }
}

function flowExtentFit() {
  // Center the initial cloud inside the canvas.
  const c = $('flowCanvas');
  if (!c.width) {
    const wrap = $('flowwrap');
    c.width = wrap.clientWidth;
    c.height = Math.max(400, Math.min(540, window.innerHeight - 430));
  }
}

function flowStep() {
  const N = flow.nodes;
  const S = flow.sim;
  // pairwise repulsion (O(n²) is fine at corpus-tab sizes)
  for (let i = 0; i < N.length; i++) {
    const a = S.get(N[i].id);
    for (let j = i + 1; j < N.length; j++) {
      const b = S.get(N[j].id);
      let dx = a.x - b.x, dy = a.y - b.y;
      let d2 = dx * dx + dy * dy;
      if (d2 < 1) { d2 = 1; dx = Math.random(); dy = Math.random(); }
      const f = 900 / d2;
      const d = Math.sqrt(d2);
      a.vx += (dx / d) * f; a.vy += (dy / d) * f;
      b.vx -= (dx / d) * f; b.vy -= (dy / d) * f;
    }
  }
  // springs along edges
  for (const e of flow.edges) {
    const a = S.get(e.source), b = S.get(e.target);
    const dx = b.x - a.x, dy = b.y - a.y;
    const d = Math.max(1, Math.hypot(dx, dy));
    const f = (d - 90) * 0.004;
    a.vx += (dx / d) * f * d; a.vy += (dy / d) * f * d;
    b.vx -= (dx / d) * f * d; b.vy -= (dy / d) * f * d;
  }
  const c = $('flowCanvas');
  const bound = Math.min(c.width, c.height) / 2 / flow.zoom - 40;
  for (const n of N) {
    const s = S.get(n.id);
    s.vx += -s.x * 0.03; s.vy += -s.y * 0.03;   // spring to centre
    s.vx *= 0.82; s.vy *= 0.82;
    if (flow.dragId !== n.id) { s.x += Math.max(-4, Math.min(4, s.vx)); s.y += Math.max(-4, Math.min(4, s.vy)); }
    // keep the cloud inside the viewport — repulsion wins at the rim otherwise
    s.x = Math.max(-bound, Math.min(bound, s.x));
    s.y = Math.max(-bound, Math.min(bound, s.y));
  }
}

function flowTransform(x, y) {
  const c = $('flowCanvas');
  return [c.width / 2 + x * flow.zoom + flow.panX, c.height / 2 + y * flow.zoom + flow.panY];
}
flow.zoom = 1; flow.panX = 0; flow.panY = 0;

function flowDraw(now) {
  const c = $('flowCanvas');
  const ctx = c.getContext('2d');
  ctx.clearRect(0, 0, c.width, c.height);
  ctx.lineWidth = 1;

  // edges + travelling particles
  for (const e of flow.edges) {
    const a = flow.sim.get(e.source), b = flow.sim.get(e.target);
    const [ax, ay] = flowTransform(a.x, a.y);
    const [bx, by] = flowTransform(b.x, b.y);
    const col = flowRelColor(e.relation);
    ctx.globalAlpha = 0.30;
    ctx.strokeStyle = col;
    ctx.beginPath(); ctx.moveTo(ax, ay); ctx.lineTo(bx, by); ctx.stroke();
    ctx.globalAlpha = 1;

    // particle position derived from wall time — no per-edge state needed
    const speed = 140;                       // px/s along the line
    const len = Math.max(1, Math.hypot(bx - ax, by - ay));
    const phase = ((now / 1000) * speed + e.source.hashCode()) % len;
    const t = phase / len;
    const px = ax + (bx - ax) * t, py = ay + (by - ay) * t;
    ctx.fillStyle = col;
    ctx.shadowColor = col; ctx.shadowBlur = 8;
    ctx.beginPath(); ctx.arc(px, py, 2.2, 0, 7); ctx.fill();
    ctx.shadowBlur = 0;
  }

  // nodes
  for (const n of flow.nodes) {
    const s = flow.sim.get(n.id);
    const [x, y] = flowTransform(s.x, s.y);
    const isH = n.kind === 'hypothesis';
    const failed = n.kind === 'node' && n.verdict != null && n.verdict < 0;
    const good = n.kind === 'node' && n.verdict != null && n.verdict > 0;
    const col = isH ? '#b49cff' : failed ? '#ff718d' : good ? '#72f0b2' : '#68e7ff';

    if (isH) {
      // hexagon for hypotheses
      ctx.fillStyle = 'rgba(11,16,35,.9)';
      ctx.strokeStyle = col; ctx.lineWidth = 1.6;
      ctx.beginPath();
      for (let i = 0; i < 6; i++) {
        const ang = Math.PI / 6 + (i / 6) * 2 * Math.PI;
        const px = x + Math.cos(ang) * 9, py = y + Math.sin(ang) * 9;
        i ? ctx.lineTo(px, py) : ctx.moveTo(px, py);
      }
      ctx.closePath(); ctx.fill(); ctx.stroke();
    } else {
      ctx.fillStyle = col;
      ctx.shadowColor = col;
      ctx.shadowBlur = flow.hoverId === n.id ? 18 : 8;
      ctx.beginPath(); ctx.arc(x, y, 5, 0, 7); ctx.fill();
      ctx.shadowBlur = 0;
    }
  }
}

function flowLoop() {
  flow.raf = requestAnimationFrame(flowLoop);
  if (state.tab !== 'flow') return;         // idle when hidden
  if (!flow.paused) flowStep();
  flowDraw(performance.now());
}

// tiny string hash for stable particle phases
Object.defineProperty(String.prototype, 'hashCode', {
  value: function () {
    let h = 0;
    for (let i = 0; i < this.length; i++) h = (h * 31 + this.charCodeAt(i)) | 0;
    return Math.abs(h);
  },
});

(function flowInteract() {
  document.addEventListener('DOMContentLoaded', () => {
    const c = $('flowCanvas');
    let down = false, dragNode = null, lx = 0, ly = 0, moved = false;
    c.addEventListener('mousedown', e => {
      down = true; moved = false; lx = e.offsetX; ly = e.offsetY;
      dragNode = flowPick(e.offsetX, e.offsetY);
      if (dragNode) flow.dragId = dragNode.id;
    });
    window.addEventListener('mouseup', () => { down = false; flow.dragId = null; });
    c.addEventListener('mousemove', e => {
      if (down && !dragNode) {
        flow.panX += e.offsetX - lx; flow.panY += e.offsetY - ly;
        lx = e.offsetX; ly = e.offsetY; moved = true;
        return;
      }
      if (down && dragNode) {
        const s = flow.sim.get(dragNode.id);
        s.x += (e.offsetX - lx) / flow.zoom; s.y += (e.offsetY - ly) / flow.zoom;
        s.vx = 0; s.vy = 0;
        lx = e.offsetX; ly = e.offsetY; moved = true;
        return;
      }
      const hit = flowPick(e.offsetX, e.offsetY);
      flow.hoverId = hit ? hit.id : null;
      const tip = $('flowTip');
      if (hit) {
        const detail = hit.kind === 'hypothesis'
          ? '<span class="muted">hypothesis · ' + esc(hit.status) + ' · fitness ' + (+hit.fitness).toFixed(2) + '</span>'
          : '<span class="muted">node · coherence ' + (+hit.score).toFixed(2)
            + (hit.cell ? ' · ' + esc(hit.cell) : '') + '</span>';
        tip.innerHTML = '<b>' + esc((hit.label || '').slice(0, 100)) + '</b><br>' + detail;
        tip.style.display = 'block';
        tip.style.left = Math.min(e.offsetX + 14, c.clientWidth - 210) + 'px';
        tip.style.top = (e.offsetY + 12) + 'px';
        c.style.cursor = 'pointer';
      } else {
        tip.style.display = 'none';
        c.style.cursor = 'default';
      }
    });
    c.addEventListener('wheel', e => {
      e.preventDefault();
      flow.zoom = Math.min(4, Math.max(0.25, flow.zoom * (e.deltaY < 0 ? 1.12 : 1 / 1.12)));
    }, { passive: false });
  });
})();

function flowPick(mx, my) {
  for (const n of flow.nodes) {
    const s = flow.sim.get(n.id);
    const [x, y] = flowTransform(s.x, s.y);
    if ((x - mx) ** 2 + (y - my) ** 2 < 121) return n;
  }
  return null;
}

$('btnFlowPause').onclick = () => {
  flow.paused = !flow.paused;
  $('btnFlowPause').textContent = flow.paused ? 'Resume' : 'Pause';
};
$('btnDemoFlow').onclick = guard('sample workflow', async () => {
  await api('/api/flow/processes/demo', {});
  toast('sample workflow recorded');
  startFlow();
});

const TASK_COLORS = {
  Pending: '#8997b8', InProgress: '#68e7ff', Completed: '#72f0b2',
  Blocked: '#ffc66d', Failed: '#ff718d', Intervened: '#b49cff',
};

function renderWorkflows() {
  if (!flow.workflows.length) {
    $('workflows').innerHTML = '<div class="muted">No cycles recorded yet — load a sample workflow or record one via the API.</div>';
    return;
  }
  $('workflows').innerHTML = flow.workflows.map(wf => {
    const taskCount = (wf.tasks || []).length;
    const stages = [
      ['PLAN', taskCount + ' tasks'],
      ['MEASURE', wf.measurements + ' (' + wf.nominal_measurements + ' nom.)'],
      ['DEVIATE', wf.deviations ? wf.deviations + ' · sev ' + (+wf.max_severity).toFixed(2) : 'clean'],
      ['INTERVENE', wf.interventions ? wf.interventions + (wf.open_interventions ? ' · ' + wf.open_interventions + ' open' : '') : '—'],
      ['OUTCOME', wf.outcome ? (wf.outcome.success ? 'success' : 'failed') : 'pending'],
    ];
    const stageRow = stages.map(([k, v], i) =>
      '<div class="stage">'
      + '<div class="stagename">' + k + '</div>'
      + '<div class="stageval">' + esc(v) + '</div>'
      + (i < stages.length - 1 ? '<div class="stagearrow">▸</div>' : '')
      + '</div>').join('');
    const tasks = (wf.tasks || []).map(t =>
      '<span class="taskchip" style="border-color:'
      + (TASK_COLORS[t.state] || '#888') + '55;color:'
      + (TASK_COLORS[t.state] || '#888') + '" title="' + esc(t.id) + '">'
      + esc(t.title) + '</span>').join('');
    const outcomeBadge = wf.outcome
      ? '<span class="obadge ' + (wf.outcome.success ? 'good' : 'bad') + '">'
        + (wf.outcome.success ? '✓ ' : '✕ ') + esc(wf.outcome.summary || '') + '</span>'
      : '';
    return '<div class="wfcard"><div class="l1"><b>' + esc(wf.plan_name || wf.id) + '</b>'
      + outcomeBadge + '</div>'
      + '<div class="stagerow">' + stageRow + '</div>'
      + (tasks ? '<div class="taskrow">' + tasks + '</div>' : '')
      + '</div>';
  }).join('');
}

function renderConflicts() {
  if (!flow.conflicts.length) {
    $('conflicts').innerHTML = '<div class="muted">None registered.</div>';
    return;
  }
  $('conflicts').innerHTML = flow.conflicts.map(c =>
    '<div class="item bad"><div class="l1"><span class="cell">'
    + esc(c.resolution) + '</span></div>'
    + '<div class="txt">A: ' + esc(c.a) + '</div>'
    + '<div class="txt">B: ' + esc(c.b) + '</div>'
    + (c.explanation ? '<div class="txt muted">' + esc(c.explanation) + '</div>' : '')
    + '</div>').join('');
}

/* ══ COHERENCE — radar checker ═════════════════════════════════ */
const DIMS = ['semantic', 'ontological', 'logical', 'empirical', 'procedural', 'temporal'];

const COH_CLAIMS = [
  'The spindle bearing failed because lubrication intervals were extended past 2000 hours.',
  'Quarterly revenue increased because the marketing campaign went viral.',
  'Coolant pressure loss was caused by a clogged filter downstream of the pump.',
  'The new hire completed forklift certification before starting warehouse shifts.',
  'This defect came from the supplier lot, not from our welding process.',
];

async function cohCheck() {
  const text = $('cohText').value.trim();
  if (!text) { toast('enter a claim to check', true); return; }
  const btn = $('btnCohCheck');
  btn.disabled = true;
  btn.textContent = 'Checking…';
  try {
    const r = await api('/api/coherence/check', { text });
    drawRadar(r.profile);
    renderCohResult(r);
    $('cohMeta').textContent = 'composite ' + r.composite.toFixed(3);
  } finally {
    btn.disabled = false;
    btn.textContent = 'Check coherence';
  }
}

function radarGeom(value, idx, cx, cy, R) {
  const ang = -Math.PI / 2 + (idx / 6) * 2 * Math.PI;
  return [cx + Math.cos(ang) * R * value, cy + Math.sin(ang) * R * value];
}

function drawRadar(profile) {
  const svg = $('radar');
  const cx = 160, cy = 158, R = 108;
  let grid = '';
  for (const frac of [0.25, 0.5, 0.75, 1.0]) {
    const pts = DIMS.map((_, i) => radarGeom(frac, i, cx, cy, R).map(v => v.toFixed(1)).join(','));
    grid += '<polygon points="' + pts.join(' ') + '" fill="none" stroke="rgba(135,157,219,.18)" stroke-width="1"/>';
  }
  const spokes = DIMS.map((d, i) => {
    const [x, y] = radarGeom(1, i, cx, cy, R);
    const [lx, ly] = radarGeom(1.24, i, cx, cy, R);
    return '<line x1="' + cx + '" y1="' + cy + '" x2="' + x + '" y2="' + y + '" stroke="rgba(135,157,219,.15)"/>'
      + '<text x="' + lx + '" y="' + ly + '" fill="#8997b8" font-size="9.5" font-family="var(--mono)" '
      + 'letter-spacing=".08em" text-anchor="middle" dominant-baseline="middle">' + d.toUpperCase() + '</text>';
  }).join('');
  svg.innerHTML = grid + spokes;

  // No profile yet: show the neutral hexagon so the panel never looks broken.
  if (!profile) {
    const pts = DIMS.map(() => 0.5).map((v, i) => radarGeom(v, i, cx, cy, R).map(u => u.toFixed(1)).join(','));
    svg.innerHTML += '<polygon points="' + pts.join(' ') + '" fill="rgba(135,157,219,.08)"'
      + ' stroke="rgba(135,157,219,.3)" stroke-width="1.5" stroke-dasharray="4 4"/>';
    return;
  }

  const target = DIMS.map(d => Math.max(0.04, profile[d] ?? 0.5));
  // animate from zero (or previous) toward target
  const from = drawRadar.last || DIMS.map(() => 0);
  drawRadar.last = target;
  const t0 = performance.now();
  const animate = now => {
    const t = Math.min(1, (now - t0) / 550);
    const ease = 1 - Math.pow(1 - t, 3);
    const cur = target.map((v, i) => from[i] + (v - from[i]) * ease);
    const pts = cur.map((v, i) => radarGeom(v, i, cx, cy, R).map(u => u.toFixed(1)).join(','));
    svg.innerHTML = grid + spokes
      + '<polygon points="' + pts.join(' ') + '" fill="url(#radarFill)" stroke="#68e7ff" stroke-width="2"'
      + ' style="filter:drop-shadow(0 0 10px rgba(104,231,255,.45))"/>'
      + cur.map((v, i) => {
        const [x, y] = radarGeom(v, i, cx, cy, R);
        return '<circle cx="' + x + '" cy="' + y + '" r="3.2" fill="#68e7ff"/>';
      }).join('')
      + '<defs><radialGradient id="radarFill" cx="50%" cy="50%" r="65%">'
      + '<stop offset="0%" stop-color="rgba(180,156,255,.34)"/>'
      + '<stop offset="100%" stop-color="rgba(104,231,255,.10)"/></radialGradient></defs>';
    if (t < 1) requestAnimationFrame(animate);
  };
  requestAnimationFrame(animate);
}

const VERDICT_STYLE = {
  coherent: ['COHERENT', 'good'],
  tension: ['TENSION', 'warn'],
  incoherent: ['INCOHERENT', 'bad'],
  contradicted: ['CONTRADICTED', 'bad'],
};

function bar(label, v, danger) {
  const pct = Math.round(v * 100);
  const col = danger ? (v < 0.45 ? 'var(--red)' : 'var(--amber)') : 'var(--accent)';
  return '<div class="dimbar"><span class="dl">' + label + '</span>'
    + '<span class="db"><i style="width:' + pct + '%;background:' + col + '"></i></span>'
    + '<span class="dv">' + v.toFixed(3) + '</span></div>';
}

function renderCohResult(r) {
  const [label, cls] = VERDICT_STYLE[r.verdict] || [r.verdict.toUpperCase(), 'warn'];
  $('verdictBox').innerHTML = '<span class="vbadge ' + cls + '">' + label + '</span>'
    + '<span class="vscore">' + r.composite.toFixed(3) + '</span>';

  $('dimBars').innerHTML =
    bar('semantic fit', r.profile.semantic)
    + bar('ontological ground', r.profile.ontological)
    + bar('logical consistency', r.profile.logical, true)
    + bar('empirical support', r.profile.empirical)
    + bar('process alignment', r.profile.procedural)
    + bar('temporal sanity', r.profile.temporal)
    + '<div class="dimbar"><span class="dl muted">contradiction pressure</span>'
    + '<span class="db"><i style="width:' + Math.round(r.contradiction_pressure * 100) + '%;background:var(--amber)"></i></span>'
    + '<span class="dv">' + r.contradiction_pressure.toFixed(3) + '</span></div>';

  $('cohRecall').innerHTML = r.recall.length
    ? r.recall.map(h => '<div class="item"><div class="l1"><span class="cell">'
      + (+h.score).toFixed(3) + '</span></div><div class="txt">' + esc((h.label || '').slice(0, 110)) + '</div></div>').join('')
    : '<span class="muted">no recalled nodes — scan a corpus first</span>';

  $('cohProcHits').innerHTML = r.process_hits.length
    ? r.process_hits.map(p => '<span class="taskchip" style="border-color:#72f0b255;color:#72f0b2">' + esc(p) + '</span>').join(' ')
    : '<span class="muted">no workflow steps matched</span>';
}

$('btnCohCheck').onclick = guard('coherence check', cohCheck);
$('btnCohSample').onclick = () => {
  $('cohText').value = COH_CLAIMS[Math.floor(Math.random() * COH_CLAIMS.length)];
};

/* ── boot ─────────────────────────────────────────────────────── */
function refreshAll() {
  loadHealth();
  if (state.tab === 'grid') loadGrid();
  if (state.tab === 'ontology') { loadOntology(); if (lab.loaded) labResize(); }
  if (state.tab === 'flow') startFlow();
  if (state.tab === 'quality') loadQuality();
}
loadHealth();
// Deep links: #flow, #coherence, #lab(=ontology)… open the studio on a tab.
const bootHash = (location.hash || '').replace('#', '');
if (TABS.includes(bootHash)) showTab(bootHash);
