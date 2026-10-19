const labels = { yes: '● Supported', different: '◐ Different model', no: '○ Not supported', unknown: '? Unverified' };
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
  const response = await fetch(path);
  if (!response.ok) throw new Error(`Could not load ${path} (${response.status})`);
  return response.json();
}
function renderFeatures(data) {
  const table = document.querySelector('#features');
  table.replaceChildren(element('caption', 'Capabilities in the pinned versions. Cells link to each library’s API.'));
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
  for (const feature of data.features) {
    const row = element('tr');
    const heading = element('th', feature.name);
    heading.scope = 'row';
    row.append(heading);
    feature.support.forEach((status, index) => {
      const cell = element('td', undefined, status);
      const source = link(labels[status], data.libraries[index].source);
      source.title = feature.note;
      source.setAttribute('aria-label', `${feature.name}: ${labels[status]}. ${feature.note}`);
      cell.append(source);
      row.append(cell);
    });
    body.append(row);
  }
  table.append(head, body);
}
async function main() {
  try {
    renderFeatures(await loadJson('data/features.json'));
  } catch (error) {
    document.querySelector('#features caption').textContent = `Feature data unavailable. ${error.message}`;
  }
}
main();
