import { serve, file } from "bun";
import { $ } from "bun";
import { join } from "path";
import { readFileSync, existsSync } from "fs";

const PORT = 3000;

function parseCSV(csvText: string): any[] {
  const lines = csvText.trim().split("\n");
  if (lines.length === 0) return [];
  const headers = lines[0].split(",");
  return lines.slice(1).map(line => {
    const values = line.split(",");
    const obj: any = {};
    headers.forEach((h, i) => {
      obj[h.trim()] = values[i]?.trim();
    });
    return obj;
  });
}

serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);

    // Serve static files
    if (url.pathname === "/") {
      return new Response(file("public/index.html"));
    }
    if (url.pathname === "/style.css") {
      return new Response(file("public/style.css"));
    }

    // Handle search API for HTMX
    if (req.method === "POST" && url.pathname === "/api/search") {
      const formData = await req.formData();
      const query = formData.get("q")?.toString() || "";

      if (!query) {
        return new Response("<div class='error'>Please enter a query.</div>", { headers: { "Content-Type": "text/html" } });
      }

      try {
        console.log(`Executing search for: ${query}`);
        // Run stockgo binary with commands piped via stdin
        // -S select [query]
        // -S export downloads/export.csv
        // exit
        const input = `-S select ${query}\n-S export downloads/export.csv\nexit\n`;
        
        await $`echo ${input} | ./target/debug/stockgo`.quiet();

        const csvPath = "downloads/export.csv";
        if (!existsSync(csvPath)) {
          return new Response("<div class='error'>Search completed but no export.csv found.</div>", { headers: { "Content-Type": "text/html" } });
        }

        const csvContent = readFileSync(csvPath, "utf-8");
        const data = parseCSV(csvContent);

        if (data.length === 0) {
          return new Response("<div class='info'>No results found for your query.</div>", { headers: { "Content-Type": "text/html" } });
        }

        // Build HTML table and chart script
        const headers = Object.keys(data[0]);
        
        let html = `<div class="results-container">`;
        
        // Data table
        html += `<div class="table-container"><table><thead><tr>`;
        headers.forEach(h => html += `<th>${h}</th>`);
        html += `</tr></thead><tbody>`;
        data.slice(0, 100).forEach(row => {
          html += `<tr>`;
          headers.forEach(h => html += `<td>${row[h]}</td>`);
          html += `</tr>`;
        });
        html += `</tbody></table></div>`;

        // Chart.js integration
        // Try to find a date column and a numeric column
        const labelCol = headers.find(h => h.toLowerCase().includes("date") || h.toLowerCase().includes("pathin")) || headers[0];
        const numCols = headers.filter(h => h !== labelCol && !isNaN(Number(data[0][h])));
        const dataCol = numCols.length > 0 ? numCols[0] : (headers.length > 1 ? headers[1] : headers[0]);

        const chartLabels = data.map(r => r[labelCol] || "").slice(0, 100);
        const chartValues = data.map(r => parseFloat(r[dataCol] || "0")).slice(0, 100);

        html += `
          <div class="chart-container">
            <canvas id="resultsChart"></canvas>
          </div>
          <script>
            if (window.currentChart) {
              window.currentChart.destroy();
            }
            const ctx = document.getElementById('resultsChart').getContext('2d');
            window.currentChart = new Chart(ctx, {
              type: 'line',
              data: {
                labels: ${JSON.stringify(chartLabels)},
                datasets: [{
                  label: '${dataCol}',
                  data: ${JSON.stringify(chartValues)},
                  borderColor: '#10b981',
                  backgroundColor: 'rgba(16, 185, 129, 0.2)',
                  borderWidth: 2,
                  tension: 0.3,
                  fill: true
                }]
              },
              options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                  legend: { labels: { color: '#e2e8f0' } }
                },
                scales: {
                  x: { ticks: { color: '#94a3b8' }, grid: { color: 'rgba(148, 163, 184, 0.1)' } },
                  y: { ticks: { color: '#94a3b8' }, grid: { color: 'rgba(148, 163, 184, 0.1)' } }
                }
              }
            });
          </script>
        `;

        html += `</div>`;
        return new Response(html, { headers: { "Content-Type": "text/html" } });
        
      } catch (err: any) {
        console.error(err);
        return new Response(`<div class='error'>Execution failed: ${err.message}</div>`, { headers: { "Content-Type": "text/html" } });
      }
    }

    return new Response("Not Found", { status: 404 });
  }
});

console.log(`Server listening on http://localhost:${PORT}`);
