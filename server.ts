import { serve, file } from "bun";
import { $ } from "bun";
import { join } from "path";
import { readFileSync, existsSync } from "fs";
import { renderTable } from "./components/table";
import { renderChart } from "./components/chart";
import { renderAdBanner } from "./components/ad_banner";

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

function isPremium(req: Request): boolean {
  const cookie = req.headers.get("cookie") || "";
  return cookie.includes("premium=true");
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

    // Toggle Premium API
    if (req.method === "POST" && url.pathname === "/api/toggle-premium") {
      const currentlyPremium = isPremium(req);
      const newStatus = !currentlyPremium;
      return new Response(
        `<span class="premium-status">Premium: ${newStatus ? 'ON' : 'OFF'}</span>`,
        {
          headers: {
            "Content-Type": "text/html",
            "Set-Cookie": `premium=${newStatus}; Path=/; SameSite=Lax`
          }
        }
      );
    }

    // Export CSV API
    if (req.method === "GET" && url.pathname === "/api/export") {
      if (!isPremium(req)) {
        return new Response("Premium subscription required for data export.", { status: 403 });
      }
      const csvPath = "downloads/export.csv";
      if (!existsSync(csvPath)) {
        return new Response("No data available to export. Run a search first.", { status: 404 });
      }
      const csvContent = readFileSync(csvPath, "utf-8");
      return new Response(csvContent, {
        headers: {
          "Content-Type": "text/csv",
          "Content-Disposition": `attachment; filename="stockgo_export.csv"`
        }
      });
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

        const headers = Object.keys(data[0]);
        const premium = isPremium(req);
        
        let html = `<div class="results-container">`;
        
        // Monetization: Inject Ad Banner if not premium
        if (!premium) {
          html += renderAdBanner();
        }

        // Export Button (Premium Feature)
        if (premium) {
          html += `
            <div class="premium-actions">
              <a href="/api/export" target="_blank" class="export-btn">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
                Export CSV
              </a>
            </div>
          `;
        } else {
          html += `
            <div class="premium-actions locked">
              <button disabled class="export-btn locked-btn" title="Exporting is a Premium Feature">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect><path d="M7 11V7a5 5 0 0 1 10 0v4"></path></svg>
                Export CSV (Premium)
              </button>
            </div>
          `;
        }
        
        // Assemble decoupled components
        html += renderChart(headers, data);
        html += renderTable(headers, data);
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
