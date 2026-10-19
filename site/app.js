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
function badge(description, url) {
  const image = element('img', undefined, 'metric-badge');
  image.alt = description;
  image.src = url;
  image.height = 20;
  image.loading = 'lazy';
  image.decoding = 'async';
  image.addEventListener('error', () => image.replaceWith(element('span', '/')));
  return image;
}
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
  const downloadsRow = element('tr', undefined, 'downloads');
  const downloadsHeading = element('th', 'crates.io downloads');
  downloadsHeading.scope = 'row';
  downloadsRow.append(downloadsHeading);
  for (const library of data.libraries) {
    const cell = element('td');
    const crateLink = link(undefined, `${library.url}/downloads`);
    crateLink.title = `All-time downloads of the ${library.id} Rust crate across all versions`;
    crateLink.append(badge(`Total crates.io downloads for ${library.id}`,
      `https://img.shields.io/crates/d/${encodeURIComponent(library.id)}?style=flat&color=555&labelColor=333&label=total`));
    cell.append(crateLink);
    downloadsRow.append(cell);
  }
  body.append(downloadsRow);
  const usedByRow = element('tr', undefined, 'used-by');
  const usedByHeading = element('th', 'Used by');
  usedByHeading.scope = 'row';
  usedByRow.append(usedByHeading);
  for (const library of data.libraries) {
    const cell = element('td');
    library.usedBy.forEach((project, index) => {
      if (index > 0) cell.append(', ');
      const projectLink = link(project.name, project.url);
      if (project.note) projectLink.title = project.note;
      cell.append(projectLink);
      if (project.note) cell.append(element('small', project.note));
    });
    if (!library.usedBy.length) cell.append('/');
    usedByRow.append(cell);
  }
  body.append(usedByRow);
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
function renderBenchmarkChart(run, libraries, scenarios) {
  const namespace = 'http://www.w3.org/2000/svg';
  const svgElement = (tag, attributes = {}, text) => {
    const node = document.createElementNS(namespace, tag);
    for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, value);
    if (text !== undefined) node.textContent = text;
    return node;
  };
  const left = 150;
  const top = 34;
  const columnWidth = 72;
  const rowHeight = 32;
  const cellWidth = columnWidth - 4;
  const cellHeight = rowHeight - 4;
  const width = left + scenarios.length * columnWidth + 12;
  const labelsHeight = Math.max(0, ...scenarios.map(scenario => scenario.length)) * 4.5 + 20;
  const height = top + libraries.length * rowHeight + labelsHeight;
  const svg = svgElement('svg', { width, height, viewBox: `0 0 ${width} ${height}`, role: 'img', 'aria-labelledby': 'heatmap-title heatmap-description' });
  svg.style.setProperty('--heatmap-width', `${width}px`);
  svg.append(
    svgElement('title', { id: 'heatmap-title' }, 'Layout benchmark heatmap'),
    svgElement('desc', { id: 'heatmap-description' }, 'Benchmarks on the X axis, libraries on the Y axis. Cells show mean layout times. Green is fastest and red is slowest within each benchmark. Exact values and confidence intervals are also available in the results table.'),
    svgElement('text', { x: left - 12, y: 22, 'text-anchor': 'end', fill: '#a1a1aa', 'font-size': 11 }, 'Libraries'),
    svgElement('text', { x: left + scenarios.length * columnWidth / 2, y: 22, 'text-anchor': 'middle', fill: '#a1a1aa', 'font-size': 11 }, 'Benchmarks'),
  );
  libraries.forEach((library, index) => {
    svg.append(svgElement('text', { x: left - 12, y: top + index * rowHeight + 18, 'text-anchor': 'end', fill: '#e4e4e7', 'font-size': 12 }, library.name));
  });
  scenarios.forEach((scenario, columnIndex) => {
    const results = run.results.filter(result => result.scenario === scenario && libraries.some(library => library.id === result.library));
    const minimum = Math.min(...results.map(result => result.mean_ns));
    const maximum = Math.max(...results.map(result => result.mean_ns));
    const positionX = left + columnIndex * columnWidth;
    svg.append(svgElement('text', {
      transform: `translate(${positionX + cellWidth / 2}, ${top + libraries.length * rowHeight + 8}) rotate(-45)`,
      'text-anchor': 'end', fill: '#a1a1aa', 'font-size': 11,
    }, scenario.replaceAll('_', ' ')));
    libraries.forEach((library, rowIndex) => {
      const result = results.find(result => result.library === library.id);
      const positionY = top + rowIndex * rowHeight;
      let fill = '#222';
      if (result) {
        const relative = maximum === minimum ? 0 : (result.mean_ns - minimum) / (maximum - minimum);
        const start = relative <= 0.5 ? [29, 87, 59] : [48, 48, 48];
        const end = relative <= 0.5 ? [48, 48, 48] : [113, 55, 55];
        const progress = relative <= 0.5 ? relative * 2 : (relative - 0.5) * 2;
        fill = `rgb(${start.map((channel, index) => Math.round(channel + (end[index] - channel) * progress)).join(',')})`;
      }
      const cell = svgElement('g');
      const description = result
        ? `${library.name}, ${scenario.replaceAll('_', ' ')}: ${formatTime(result.mean_ns)} mean, 95% confidence interval ${formatTime(result.lower_ns)} to ${formatTime(result.upper_ns)}`
        : `${library.name}, ${scenario.replaceAll('_', ' ')}: ${run.versions[library.id] ? 'not supported by adapter' : 'not measured in this run'}`;
      cell.append(
        svgElement('title', {}, description),
        svgElement('rect', { x: positionX, y: positionY, width: cellWidth, height: cellHeight, rx: 3, fill }),
        svgElement('text', { x: positionX + cellWidth / 2, y: positionY + 18, 'text-anchor': 'middle', fill: result ? '#f4f4f5' : '#a1a1aa', 'font-size': 11 }, result ? formatTime(result.mean_ns) : '/'),
      );
      svg.append(cell);
    });
  });
  document.querySelector('#benchmark-chart').replaceChildren(svg);
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
    renderBenchmarkChart(run, libraries, scenarios);
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
  document.querySelector('#benchmark-content').hidden = false;
  render();
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
