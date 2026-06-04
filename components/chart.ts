export function renderChart(headers: string[], data: any[]): string {
  // Try to find a date column and a numeric column
  const labelCol = headers.find(h => h.toLowerCase().includes("date") || h.toLowerCase().includes("pathin")) || headers[0];
  const numCols = headers.filter(h => h !== labelCol && !isNaN(Number(data[0][h])));
  const dataCol = numCols.length > 0 ? numCols[0] : (headers.length > 1 ? headers[1] : headers[0]);

  const chartLabels = data.map(r => r[labelCol] || "").slice(0, 100);
  const chartValues = data.map(r => parseFloat(r[dataCol] || "0")).slice(0, 100);

  return `
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
}
