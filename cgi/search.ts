import { $ } from "bun";
import { readFileSync, existsSync } from "fs";
import { renderTable } from "../components/table";
import { renderChart } from "../components/chart";

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

// Minimal RESP client for ServerGo cache
async function respCommand(cmd: string, ...args: string[]): Promise<string | null> {
  return new Promise((resolve) => {
    let responseData = Buffer.alloc(0);
    Bun.connect({
      hostname: "127.0.0.1",
      port: 6379,
      socket: {
        data(socket, data) {
          responseData = Buffer.concat([responseData, data]);
          const str = responseData.toString("utf-8");
          if (str.startsWith("$-1\r\n")) {
            socket.end();
            resolve(null);
          } else if (str.startsWith("$")) {
            const firstN = str.indexOf("\r\n");
            if (firstN > 0) {
              const len = parseInt(str.substring(1, firstN), 10);
              const start = firstN + 2;
              if (responseData.length >= start + len + 2) {
                socket.end();
                resolve(responseData.subarray(start, start + len).toString("utf-8"));
              }
            }
          } else if (str.startsWith("+") || str.startsWith("-")) {
            socket.end();
            const firstN = str.indexOf("\r\n");
            resolve(str.substring(1, firstN));
          }
        },
        error(socket, error) {
          socket.end();
          resolve(null);
        },
        end(socket) {
          if (responseData.length === 0) resolve(null);
        },
      },
    }).then(socket => {
      let req = `*${args.length + 1}\r\n$${Buffer.byteLength(cmd)}\r\n${cmd}\r\n`;
      for (const arg of args) {
        req += `$${Buffer.byteLength(arg)}\r\n${arg}\r\n`;
      }
      socket.write(req);
    }).catch(() => {
      resolve(null); // Fallback if RESP server is down
    });
  });
}

async function main() {
  try {
    const rawBody = await Bun.readableStreamToText(Bun.stdin.stream());
    const params = new URLSearchParams(rawBody);
    const query = params.get("q") || "";

    if (!query) {
      console.log("<div class='error'>Please enter a query.</div>");
      return;
    }

    const cacheKey = `cache:search:${query}`;

    // 1. Try hitting the L2 Caching Layer directly (O(1) memory lookup via dualcache-ff)
    const cachedHtml = await respCommand("GET", cacheKey);
    if (cachedHtml) {
      console.log(cachedHtml);
      return;
    }

    // 2. Cache Miss: We execute the heavy task using stockgo CLI
    const stockgoDir = Bun.fileURLToPath(new URL("../", import.meta.url));
    const stockgoBin = `${stockgoDir}/target/debug/stockgo`;
    const exportCsvPath = `${stockgoDir}/downloads/export.csv`;
    const input = `-S\nselect ${query}\nexport ${exportCsvPath}\nexit\nexit\n`;
    
    const proc = Bun.spawn([stockgoBin, "--skip-update"], {
      cwd: stockgoDir,
      stdin: "pipe",
      stdout: "ignore",
      stderr: "ignore",
    });
    proc.stdin.write(input);
    proc.stdin.flush();
    proc.stdin.end();
    await proc.exited;

    let html = "";
    if (!existsSync(exportCsvPath)) {
      html = "<div class='error'>Search completed but no export.csv found.</div>";
    } else {
      const csvContent = readFileSync(exportCsvPath, "utf-8");
      const data = parseCSV(csvContent);

      if (data.length === 0) {
        html = "<div class='info'>No results found for your query.</div>";
      } else {
        const headers = Object.keys(data[0]);
        html = `<div class="results-container">\n`;
        html += renderChart(headers, data) + "\n";
        html += renderTable(headers, data) + "\n";
        html += `</div>`;
      }
    }

    // 3. Save the result back into cdDB / dualcache-ff cache for subsequent O(1) lookups
    await respCommand("PUT", cacheKey, html);

    console.log(html);

  } catch (err: any) {
    console.log(`<div class='error'>Execution failed: ${err.message}</div>`);
  }
}

main();
