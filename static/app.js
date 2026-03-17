(function () {
  'use strict';

  const API_TRANSFORM = '/api/transform';
  const API_AUTH_CHECK = '/api/auth-check';
  const STORAGE_API_KEY = 'cloudshift_api_key';

  const el = {
    source: document.getElementById('source'),
    language: document.getElementById('language'),
    source_cloud: document.getElementById('source_cloud'),
    path_hint: document.getElementById('path_hint'),
    transformBtn: document.getElementById('transform-btn'),
    status: document.getElementById('status'),
    resultPanel: document.getElementById('result-panel'),
    resultMeta: document.getElementById('result-meta'),
    emptyState: document.getElementById('empty-state'),
    resultContent: document.getElementById('result-content'),
    diffView: document.getElementById('diff-view'),
    patternList: document.getElementById('pattern-list'),
    resultPatternsWrap: document.getElementById('result-patterns-wrap'),
    warningList: document.getElementById('warning-list'),
    resultWarningsWrap: document.getElementById('result-warnings-wrap'),
    settingsBtn: document.getElementById('settings-btn'),
    settingsModal: document.getElementById('settings-modal'),
    apiKeyInput: document.getElementById('api-key'),
    settingsCancel: document.getElementById('settings-cancel'),
    settingsSave: document.getElementById('settings-save'),
    apiKeyBadge: document.getElementById('api-key-badge'),
    settingsTestKey: document.getElementById('settings-test-key'),
    settingsTestResult: document.getElementById('settings-test-result'),
  };

  function getApiKey() {
    return localStorage.getItem(STORAGE_API_KEY) || '';
  }

  function refreshApiKeyBadge() {
    if (!el.apiKeyBadge) return;
    const key = (getApiKey() || '').trim();
    el.apiKeyBadge.textContent = key ? 'API key: set' : 'API key: not set';
  }

  function setStatus(msg, type) {
    el.status.textContent = msg || '';
    el.status.className = 'status' + (type ? ' ' + type : '');
  }

  function setBusy(busy) {
    el.transformBtn.disabled = busy;
    el.status.setAttribute('aria-busy', busy ? 'true' : 'false');
  }

  function renderDiff(diffText) {
    if (!diffText || !diffText.trim()) {
      el.diffView.innerHTML = '';
      return;
    }
    const lines = diffText.split('\n');
    const frag = document.createDocumentFragment();
    for (const line of lines) {
      const span = document.createElement('span');
      span.className = 'line';
      if (line.startsWith('+') && !line.startsWith('+++')) {
        span.classList.add('add');
        span.textContent = line;
      } else if (line.startsWith('-') && !line.startsWith('---')) {
        span.classList.add('remove');
        span.textContent = line;
      } else {
        span.classList.add('context');
        span.textContent = line;
      }
      span.appendChild(document.createTextNode('\n'));
      frag.appendChild(span);
    }
    el.diffView.innerHTML = '';
    el.diffView.appendChild(frag);
  }

  function renderPatterns(patterns) {
    el.patternList.innerHTML = '';
    if (!patterns || patterns.length === 0) {
      el.resultPatternsWrap.classList.add('hidden');
      return;
    }
    el.resultPatternsWrap.classList.remove('hidden');
    for (const p of patterns) {
      const li = document.createElement('li');
      const id = typeof p.pattern_id === 'string' ? p.pattern_id : (p.pattern_id && p.pattern_id[0]) || 'Pattern';
      li.textContent = id;
      if (p.source_text || p.replacement_text) {
        const desc = document.createElement('span');
        desc.style.display = 'block';
        desc.style.fontSize = '0.75rem';
        desc.style.color = 'var(--text-muted)';
        desc.style.marginTop = '0.25rem';
        desc.textContent = [p.source_text, p.replacement_text].filter(Boolean).join(' → ');
        li.appendChild(desc);
      }
      el.patternList.appendChild(li);
    }
  }

  function renderWarnings(warnings) {
    el.warningList.innerHTML = '';
    if (!warnings || warnings.length === 0) {
      el.resultWarningsWrap.classList.add('hidden');
      return;
    }
    el.resultWarningsWrap.classList.remove('hidden');
    for (const w of warnings) {
      const li = document.createElement('li');
      li.setAttribute('data-severity', (w.severity || 'info').toLowerCase());
      li.textContent = w.message || '';
      el.warningList.appendChild(li);
    }
  }

  function confidenceClass(c) {
    if (c == null) return '';
    if (typeof c === 'number') {
      if (c >= 0.8) return 'high';
      if (c >= 0.5) return 'medium';
      return 'low';
    }
    const s = String(c).toLowerCase();
    if (s === 'high') return 'high';
    if (s === 'medium' || s === 'medium_low') return 'medium';
    return 'low';
  }

  function confidenceLabel(c) {
    if (c == null) return '—';
    if (typeof c === 'number') return Math.round(c * 100) + '%';
    return String(c);
  }

  function showResult(data) {
    el.emptyState.classList.add('hidden');
    el.resultContent.classList.remove('hidden');

    const confidence = data.confidence;
    const confidenceStr = confidenceLabel(confidence);
    const confidenceEl = document.createElement('span');
    confidenceEl.className = 'confidence-badge ' + confidenceClass(confidence);
    confidenceEl.textContent = 'Confidence: ' + confidenceStr;

    el.resultMeta.innerHTML = '';
    el.resultMeta.appendChild(confidenceEl);

    renderDiff(data.diff || '');
    renderPatterns(data.patterns || []);
    renderWarnings(data.warnings || []);
  }

  function showEmpty() {
    el.emptyState.classList.remove('hidden');
    el.resultContent.classList.add('hidden');
  }

  async function transform() {
    const source = el.source.value;
    if (!source.trim()) {
      setStatus('Enter source code to transform.', 'error');
      return;
    }

    setStatus('Transforming…');
    setBusy(true);

    const body = {
      source: source,
      language: el.language.value,
    };
    if (el.source_cloud.value && el.source_cloud.value !== 'any') {
      body.source_cloud = el.source_cloud.value;
    }
    if (el.path_hint.value.trim()) {
      body.path_hint = el.path_hint.value.trim();
    }

    const headers = {
      'Content-Type': 'application/json',
    };
    const apiKey = (getApiKey() || '').trim();
    if (apiKey) {
      headers['X-API-Key'] = apiKey;
    }

    try {
      const res = await fetch(API_TRANSFORM, {
        method: 'POST',
        headers,
        body: JSON.stringify(body),
      });

      const text = await res.text();
      let data;
      try {
        data = JSON.parse(text);
      } catch (_) {
        data = null;
      }

      if (!res.ok) {
        setStatus(data && data.message ? data.message : res.status + ' ' + (data || text || res.statusText), 'error');
        if (res.status === 401) {
          setStatus('Unauthorized. Add an API key in Settings if using the direct URL.', 'error');
        }
        return;
      }

      showResult(data);
      setStatus('Done.', 'success');
    } catch (err) {
      setStatus('Request failed: ' + (err.message || 'Network error'), 'error');
      console.error(err);
    } finally {
      setBusy(false);
    }
  }

  function openSettings() {
    el.apiKeyInput.value = getApiKey();
    el.settingsModal.showModal();
  }

  function closeSettings() {
    el.settingsModal.close();
  }

  function saveSettings() {
    const key = el.apiKeyInput.value.trim();
    if (key) {
      localStorage.setItem(STORAGE_API_KEY, key);
    } else {
      localStorage.removeItem(STORAGE_API_KEY);
    }
    refreshApiKeyBadge();
    closeSettings();
  }

  async function testKey() {
    const key = (el.apiKeyInput.value || '').trim();
    el.settingsTestResult.textContent = '';
    if (!key) {
      el.settingsTestResult.textContent = 'Enter a key first.';
      el.settingsTestResult.className = 'settings-test-result error';
      return;
    }
    el.settingsTestResult.textContent = 'Checking…';
    el.settingsTestResult.className = 'settings-test-result';
    try {
      const res = await fetch(API_AUTH_CHECK, {
        method: 'GET',
        headers: { 'X-API-Key': key },
      });
      if (res.ok) {
        el.settingsTestResult.textContent = 'Key valid.';
        el.settingsTestResult.className = 'settings-test-result success';
      } else if (res.status === 401) {
        el.settingsTestResult.textContent = 'Key invalid or not set on server.';
        el.settingsTestResult.className = 'settings-test-result error';
      } else if (res.status === 404) {
        el.settingsTestResult.textContent = '404 — ensure load balancer routes /api/* to Cloud Run.';
        el.settingsTestResult.className = 'settings-test-result error';
      } else {
        el.settingsTestResult.textContent = res.status + ' ' + res.statusText;
        el.settingsTestResult.className = 'settings-test-result error';
      }
    } catch (err) {
      el.settingsTestResult.textContent = 'Request failed: ' + (err.message || 'network error');
      el.settingsTestResult.className = 'settings-test-result error';
    }
  }

  el.transformBtn.addEventListener('click', transform);
  el.settingsBtn.addEventListener('click', openSettings);
  el.settingsCancel.addEventListener('click', closeSettings);
  el.settingsSave.addEventListener('click', saveSettings);
  if (el.settingsTestKey) el.settingsTestKey.addEventListener('click', testKey);
  el.settingsModal.addEventListener('cancel', closeSettings);
  el.settingsModal.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') closeSettings();
  });

  refreshApiKeyBadge();
  showEmpty();
})();
