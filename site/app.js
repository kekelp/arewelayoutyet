const labels = { yes: '● Supported', partial: '◐ Partial', different: '≈ Different model', no: '○ Not supported', unknown: '? Unverified' };
const element = (tag, text, className) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
};
const link = (text, url) => {
  const node = element('a', text);
  node.href = url;
  return node;
};
async function loadJson(path) {
  const response = await fetch(path, { cache: 'no-cache' });
  if (!response.ok) throw new Error(`Could not load ${path} (${response.status})`);
  return response.json();
}
function renderFeatures(data) {
  const table = document.querySelector('#features');
  table.replaceChildren(element('caption', 'Capabilities of the pinned builds. Cells link to supporting APIs or source.'));
  const head = element('thead');
  const headings = element('tr');
  const title = element('th', 'Layout capability');
  title.scope = 'col';
  headings.append(title);
  for (const library of data.libraries) {
    const heading = element('th');
    heading.scope = 'col';
    heading.append(link(library.name, library.url), element('small', library.version));
    headings.append(heading);
  }
  head.append(headings);
  const body = element('tbody');
  const releaseRow = element('tr', undefined, 'release-date');
  const releaseHeading = element('th', 'Release date');
  releaseHeading.scope = 'row';
  releaseRow.append(releaseHeading);
  for (const library of data.libraries) {
    const cell = element('td');
    const date = element('time', library.releaseDate);
    date.dateTime = library.releaseDate;
    date.title = 'Publication date on crates.io for the listed version. Git-pinned builds may include newer changes.';
    cell.append(date);
    releaseRow.append(cell);
  }
  body.append(releaseRow);
  for (const feature of data.features) {
    const row = element('tr');
    const heading = element('th', feature.name);
    heading.scope = 'row';
    row.append(heading);
    feature.support.forEach((status, index) => {
      const cell = element('td', undefined, status);
      const library = data.libraries[index];
      const source = link(labels[status], feature.sources?.[library.id] ?? library.source);
      source.title = feature.note;
      source.setAttribute('aria-label', `${feature.name}: ${labels[status]}. ${feature.note}`);
      cell.append(source);
      row.append(cell);
    });
    body.append(row);
  }
  table.append(head, body);
}
function formatTime(nanoseconds) {
  if (nanoseconds >= 1e6) return `${(nanoseconds / 1e6).toFixed(2)} ms`;
  if (nanoseconds >= 1e3) return `${(nanoseconds / 1e3).toFixed(2)} µs`;
  return `${nanoseconds.toFixed(1)} ns`;
}
function renderBenchmarks(history, libraries) {
  if (history.schema !== 1 || !Array.isArray(history.runs)) throw new Error('Unsupported history format');
  const runs = [...history.runs].sort((first, second) => second.date.localeCompare(first.date));
  const status = document.querySelector('#benchmark-status');
  if (!runs.length) {
    status.textContent = 'No measurements published yet.';
    return;
  }
  const select = document.querySelector('#benchmark-run');
  select.replaceChildren();
  runs.forEach((run, index) => {
    const option = element('option', `${new Date(run.date).toISOString().replace('T', ' ').slice(0, 19)} UTC${index === 0 ? ' (latest)' : ''}`);
    option.value = String(index);
    select.append(option);
  });
  select.value = '0';
  const render = () => {
    const run = runs[Number(select.value)];
    const table = document.querySelector('#benchmark-results');
    table.replaceChildren(element('caption', 'Mean layout time, lower is faster. Ranges are 95% confidence intervals. Uncached layout only, adapters can differ in semantics.'));
    const head = element('thead');
    const headings = element('tr');
    const workloadHeading = element('th', 'Workload');
    workloadHeading.scope = 'col';
    headings.append(workloadHeading);
    for (const library of libraries) {
      const heading = element('th');
      heading.scope = 'col';
      heading.append(link(library.name, library.url), element('small', run.versions[library.id] ?? 'Not measured'));
      headings.append(heading);
    }
    head.append(headings);
    const body = element('tbody');
    const scenarios = [...new Set(run.results.map(result => result.scenario))].sort();
    for (const scenario of scenarios) {
      const results = run.results.filter(result => result.scenario === scenario && libraries.some(library => library.id === result.library));
      const fastestMean = Math.min(...results.map(result => result.mean_ns));
      const slowestMean = Math.max(...results.map(result => result.mean_ns));
      const row = element('tr');
      const heading = element('th', scenario.replaceAll('_', ' '));
      heading.scope = 'row';
      row.append(heading);
      for (const library of libraries) {
        const result = results.find(result => result.library === library.id);
        const cell = element('td');
        if (result) {
          const mean = element('span', formatTime(result.mean_ns));
          if (result.mean_ns === fastestMean) {
            mean.className = 'yes';
            mean.title = 'Lowest mean time in this row';
          } else if (result.mean_ns === slowestMean) {
            mean.className = 'worst';
            mean.title = 'Highest mean time in this row';
          }
          cell.append(mean, element('small', `${formatTime(result.lower_ns)} to ${formatTime(result.upper_ns)}`));
        } else {
          const missing = element('span', '/', 'small');
          missing.title = run.versions[library.id] ? 'Not supported by adapter' : 'Not benchmarked in this run';
          cell.append(missing);
        }
        row.append(cell);
      }
      body.append(row);
    }
    table.append(head, body);
    const details = document.querySelector('#run-details');
    details.replaceChildren(`${run.cpu} · ${run.rustc} · ${run.runner}`);
    if (run.url) details.append(' · ', link('CI run', run.url));
    status.textContent = `${runs.length} recorded ${runs.length === 1 ? 'run' : 'runs'}. Missing results are not zero time.`;
  };
  select.addEventListener('change', render);
  render();
  document.querySelector('#benchmark-content').hidden = false;
}
async function main() {
  let features;
  try {
    features = await loadJson('data/features.json');
    renderFeatures(features);
  } catch (error) {
    document.querySelector('#features caption').textContent = `Feature data unavailable. ${error.message}`;
  }
  try {
    const response = await fetch('data/history.json', { cache: 'no-cache' });
    if (response.status === 404) {
      document.querySelector('#benchmark-status').textContent = 'No measurements published yet. Run just fetch-data for local builds, or check the publishing workflow.';
      return;
    }
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    if (!features) throw new Error('Library definitions unavailable');
    renderBenchmarks(await response.json(), features.libraries);
  } catch (error) {
    document.querySelector('#benchmark-content').hidden = true;
    document.querySelector('#benchmark-status').textContent = `Measurements unavailable. ${error.message}`;
  }
}
main();
