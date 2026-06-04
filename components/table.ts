export function renderTable(headers: string[], data: any[]): string {
  let html = `<div class="table-container"><table><thead><tr>`;
  headers.forEach(h => html += `<th>${h}</th>`);
  html += `</tr></thead><tbody>`;
  
  data.slice(0, 100).forEach(row => {
    html += `<tr>`;
    headers.forEach(h => html += `<td>${row[h]}</td>`);
    html += `</tr>`;
  });
  
  html += `</tbody></table></div>`;
  return html;
}
