<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  type View = "form" | "scanning" | "results";
  type ScanMode = "light" | "deep";

  interface PortResult {
    port: number;
    protocol: string;
    status: string;
    service: string | null;
    product: string | null;
    version: string | null;
  }

  interface ScanResult {
    target: string;
    resolvedIp: string;
    hostname: string | null;
    osGuess: string | null;
    startMs: number;
    endMs: number;
    openCount: number;
    ports: PortResult[];
  }

  interface ScanProgress {
    scanned: number;
    total: number;
    port: number;
    isOpen: boolean;
  }

  const TOP_100: number[] = [
    20, 21, 22, 23, 25, 26, 53, 80, 81, 110, 111, 113, 135, 139, 143, 179,
    199, 443, 444, 445, 465, 514, 515, 587, 631, 993, 995, 1025, 1026, 1027,
    1028, 1029, 1433, 1720, 1723, 1900, 2000, 2001, 2049, 2121, 2717, 3000,
    3128, 3306, 3389, 3986, 4443, 4899, 5000, 5060, 5432, 5631, 5666, 5800,
    5900, 6000, 6001, 6443, 6646, 7070, 7777, 8000, 8008, 8009, 8080, 8081,
    8443, 8888, 9000, 9090, 9100, 9200, 9999, 10000, 11211, 15672, 27017,
    32768, 49152, 49153, 49154, 49155, 49156, 49157,
  ];

  function getPortList(selection: string, preset: string, custom: string): number[] {
    if (selection === "custom") {
      const ports = new Set<number>();
      for (const part of custom.split(",")) {
        const t = part.trim();
        if (t.includes("-")) {
          const [s, e] = t.split("-").map(Number);
          if (!isNaN(s) && !isNaN(e))
            for (let p = Math.max(1, s); p <= Math.min(65535, e); p++) ports.add(p);
        } else {
          const p = parseInt(t);
          if (!isNaN(p) && p >= 1 && p <= 65535) ports.add(p);
        }
      }
      return Array.from(ports).sort((a, b) => a - b);
    }
    if (preset === "top100") return TOP_100;
    if (preset === "top1000") return Array.from({ length: 1024 }, (_, i) => i + 1);
    return Array.from({ length: 65535 }, (_, i) => i + 1);
  }

  // State
  let view: View = $state("form");
  let scanMode: ScanMode = $state("light");
  let target = $state("");
  let detectService = $state(false);
  let detectOs = $state(false);
  let protocol = $state("TCP");
  let portSelection = $state("common");
  let commonPreset = $state("top100");
  let customPorts = $state("");
  let progress = $state({ scanned: 0, total: 0 });
  let result: ScanResult | null = $state(null);
  let scanError = $state("");

  function setScanMode(mode: ScanMode) {
    scanMode = mode;
    if (mode === "light") {
      detectService = false;
      detectOs = false;
      commonPreset = "top100";
    } else {
      detectService = true;
      detectOs = false;
      commonPreset = "top1000";
    }
  }

  function resetDefaults() {
    target = "";
    detectService = false;
    detectOs = false;
    protocol = "TCP";
    portSelection = "common";
    commonPreset = "top100";
    customPorts = "";
    scanError = "";
    scanMode = "light";
  }

  async function startScan() {
    if (!target.trim()) return;
    const ports = getPortList(portSelection, commonPreset, customPorts);
    if (ports.length === 0) { scanError = "No valid ports specified."; return; }

    scanError = "";
    progress = { scanned: 0, total: ports.length };
    view = "scanning";
    result = null;

    const unlisten = await listen<ScanProgress>("scan-progress", (e) => {
      progress = { scanned: e.payload.scanned, total: e.payload.total };
    });

    try {
      const r = await invoke<ScanResult>("scan_ports", {
        config: {
          target: target.trim(),
          ports,
          protocol,
          detectService,
          detectOs,
          timeoutMs: scanMode === "deep" ? 1500 : 800,
          concurrency: commonPreset === "full" ? 500 : 150,
        },
      });
      result = r;
      view = "results";
    } catch (e) {
      scanError = String(e);
      view = "form";
    } finally {
      unlisten();
    }
  }

  function newScan() {
    view = "form";
    result = null;
    scanError = "";
  }

  function formatTime(ms: number): string {
    const d = new Date(ms);
    const month = d.toLocaleString("en-US", { month: "short" });
    const day = String(d.getDate()).padStart(2, "0");
    const year = d.getFullYear();
    const h = String(d.getHours()).padStart(2, "0");
    const m = String(d.getMinutes()).padStart(2, "0");
    return `${month} ${day}, ${year} – ${h}:${m}`;
  }

  function formatDuration(startMs: number, endMs: number): string {
    const secs = Math.round((endMs - startMs) / 1000);
    if (secs < 60) return `${secs}s`;
    return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  }

  let progressPct = $derived(
    progress.total > 0 ? Math.round((progress.scanned / progress.total) * 100) : 0
  );
</script>

{#if view === "form"}
<div class="app">

  <!-- Header -->
  <header class="header">
    <div class="header-left">
      <div class="app-icon">
        <svg width="28" height="28" viewBox="0 0 28 28" fill="none">
          <circle cx="14" cy="14" r="10" stroke="#4f46e5" stroke-width="2"/>
          <circle cx="14" cy="14" r="5" stroke="#4f46e5" stroke-width="2"/>
          <circle cx="14" cy="14" r="2" fill="#4f46e5"/>
          <line x1="14" y1="4" x2="14" y2="1" stroke="#4f46e5" stroke-width="2" stroke-linecap="round"/>
          <line x1="14" y1="27" x2="14" y2="24" stroke="#4f46e5" stroke-width="2" stroke-linecap="round"/>
          <line x1="4" y1="14" x2="1" y2="14" stroke="#4f46e5" stroke-width="2" stroke-linecap="round"/>
          <line x1="27" y1="14" x2="24" y2="14" stroke="#4f46e5" stroke-width="2" stroke-linecap="round"/>
        </svg>
      </div>
      <div class="header-text">
        <div class="app-title">Port Scanner</div>
        <div class="app-subtitle">Network vulnerability assessment tool</div>
      </div>
    </div>
    <div class="scan-mode-tabs">
      <button
        class="mode-tab"
        class:active={scanMode === "light"}
        onclick={() => setScanMode("light")}
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
          <path d="M7 0L8.5 5H13L9.2 8L10.5 13L7 10L3.5 13L4.8 8L1 5H5.5L7 0Z"/>
        </svg>
        Light Scan
      </button>
      <button
        class="mode-tab"
        class:active={scanMode === "deep"}
        onclick={() => setScanMode("deep")}
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.5">
          <circle cx="6" cy="6" r="4"/>
          <line x1="9" y1="9" x2="13" y2="13"/>
        </svg>
        Deep Scan
      </button>
    </div>
  </header>

  <!-- Main -->
  <main class="main">

    <!-- TARGET IDENTIFICATION -->
    <section class="section">
      <div class="section-header">
        <span class="section-label">TARGET IDENTIFICATION</span>
        <div class="section-line"></div>
      </div>
      <div class="target-input-wrap">
        <svg class="target-icon" width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="#9ca3af" stroke-width="1.5">
          <circle cx="9" cy="9" r="7"/>
          <path d="M9 2C9 2 12 5.5 12 9C12 12.5 9 16 9 16C9 16 6 12.5 6 9C6 5.5 9 2 9 2Z"/>
          <line x1="2" y1="9" x2="16" y2="9"/>
        </svg>
        <input
          id="target-input"
          class="target-input"
          type="text"
          placeholder="Enter IP address (e.g., 192.168.1.1) or Hostname"
          bind:value={target}
          onkeydown={(e) => e.key === "Enter" && target.trim() && startScan()}
        />
        <span class="target-badge">IPv4 / IPv6</span>
      </div>
      {#if scanError}
        <p class="error-msg">{scanError}</p>
      {/if}
    </section>

    <!-- SCAN PARAMS + PROTOCOL -->
    <div class="two-col">

      <!-- Scan Parameters -->
      <div class="col">
        <div class="section-header">
          <span class="section-label">SCAN PARAMETERS</span>
          <div class="section-line"></div>
        </div>
        <div class="param-cards">
          <button
            class="param-card"
            class:selected={detectService}
            onclick={() => detectService = !detectService}
          >
            <div class="param-checkbox" class:checked={detectService}>
              {#if detectService}
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="white" stroke-width="2">
                  <polyline points="1.5,5 4,7.5 8.5,2.5"/>
                </svg>
              {/if}
            </div>
            <div class="param-text">
              <div class="param-title">Detect service version</div>
              <div class="param-desc">Determine application protocols and version numbers.</div>
            </div>
          </button>
          <button
            class="param-card"
            class:selected={detectOs}
            onclick={() => detectOs = !detectOs}
          >
            <div class="param-checkbox" class:checked={detectOs}>
              {#if detectOs}
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="white" stroke-width="2">
                  <polyline points="1.5,5 4,7.5 8.5,2.5"/>
                </svg>
              {/if}
            </div>
            <div class="param-text">
              <div class="param-title">Detect operating system</div>
              <div class="param-desc">Enable OS fingerprinting and device type detection.</div>
            </div>
          </button>
        </div>
      </div>

      <!-- Protocol Selection -->
      <div class="col">
        <div class="section-header">
          <span class="section-label">PROTOCOL SELECTION</span>
          <div class="section-line"></div>
        </div>
        <div class="param-cards">
          <button
            class="param-card proto-card"
            class:selected={protocol === "TCP"}
            onclick={() => protocol = "TCP"}
          >
            <div class="proto-radio" class:checked={protocol === "TCP"}></div>
            <div class="param-title">TCP Handshake</div>
            <span class="default-badge">Default</span>
          </button>
          <button
            class="param-card proto-card"
            class:selected={protocol === "UDP"}
            onclick={() => protocol = "UDP"}
          >
            <div class="proto-radio" class:checked={protocol === "UDP"}></div>
            <div class="param-title">UDP Packet Scan</div>
          </button>
        </div>
      </div>

    </div>

    <!-- PORT RANGE CONFIGURATION -->
    <section class="section">
      <div class="section-header">
        <span class="section-label">PORT RANGE CONFIGURATION</span>
        <div class="section-line"></div>
      </div>
      <div class="port-range-card">
        <div class="port-radios">
          <button class="port-radio-item" onclick={() => portSelection = "common"}>
            <div class="proto-radio" class:checked={portSelection === "common"}></div>
            <span class:port-radio-active={portSelection === "common"}>Common Ports Profile</span>
          </button>
          <button class="port-radio-item" onclick={() => portSelection = "custom"}>
            <div class="proto-radio" class:checked={portSelection === "custom"}></div>
            <span class:port-radio-active={portSelection === "custom"}>Custom Port List</span>
          </button>
        </div>

        {#if portSelection === "common"}
          <div class="select-wrap">
            <select class="port-select" bind:value={commonPreset}>
              <option value="top100">Top 100 Critical Ports (Standard)</option>
              <option value="top1000">Top 1000 Ports (Extended)</option>
              <option value="full">Full Range (1–65535)</option>
            </select>
            <svg class="select-chevron" width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#6b7280" stroke-width="1.5">
              <polyline points="4,6 8,10 12,6"/>
            </svg>
          </div>
        {:else}
          <input
            class="custom-ports-input"
            type="text"
            placeholder="e.g. 22, 80, 443 or 1-1024"
            bind:value={customPorts}
          />
        {/if}
      </div>
    </section>

  </main>

  <!-- Footer -->
  <footer class="footer">
    <div class="footer-status">
      <span class="status-dot"></span>
      <span class="status-text">System Ready</span>
      <span class="footer-divider">|</span>
      <span class="version-text">v3.0.1_ENT</span>
    </div>
    <div class="footer-actions">
      <button class="reset-btn" onclick={resetDefaults}>Reset Defaults</button>
      <button
        class="scan-btn"
        class:disabled={!target.trim()}
        disabled={!target.trim()}
        onclick={startScan}
      >
        <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
          <path d="M3 2l9 5-9 5V2z"/>
        </svg>
        Initialize Scan
      </button>
    </div>
  </footer>

</div>

{:else if view === "scanning"}
<div class="app scanning-app">
  <header class="header">
    <div class="header-left">
      <div class="app-icon">
        <svg width="28" height="28" viewBox="0 0 28 28" fill="none">
          <circle cx="14" cy="14" r="10" stroke="#4f46e5" stroke-width="2"/>
          <circle cx="14" cy="14" r="5" stroke="#4f46e5" stroke-width="2"/>
          <circle cx="14" cy="14" r="2" fill="#4f46e5"/>
        </svg>
      </div>
      <div class="header-text">
        <div class="app-title">Port Scanner</div>
        <div class="app-subtitle">Network vulnerability assessment tool</div>
      </div>
    </div>
  </header>
  <main class="main scanning-main">
    <div class="scanning-card">
      <div class="scanning-label">Scanning target</div>
      <div class="scanning-target">{target}</div>
      <div class="progress-track">
        <div class="progress-fill" style="width:{progressPct}%"></div>
      </div>
      <div class="scanning-stats">
        <span>{progress.scanned} / {progress.total} ports checked</span>
        <span class="pct-label">{progressPct}%</span>
      </div>
    </div>
  </main>
</div>

{:else if view === "results" && result}
<div class="app results-app">

  <header class="header">
    <div class="header-left">
      <div class="app-icon">
        <svg width="28" height="28" viewBox="0 0 28 28" fill="none">
          <circle cx="14" cy="14" r="10" stroke="#4f46e5" stroke-width="2"/>
          <circle cx="14" cy="14" r="5" stroke="#4f46e5" stroke-width="2"/>
          <circle cx="14" cy="14" r="2" fill="#4f46e5"/>
        </svg>
      </div>
      <div class="header-text">
        <div class="app-title">Port Scanner</div>
        <div class="app-subtitle">Scan complete — {result.target}</div>
      </div>
    </div>
    <button class="mode-tab active" onclick={newScan}>← New Scan</button>
  </header>

  <main class="main results-main">

    <!-- Summary -->
    <section class="section">
      <div class="section-header">
        <span class="section-label">SCAN SUMMARY</span>
        <div class="section-line"></div>
      </div>
      <div class="summary-grid">
        <div class="summary-cell">
          <div class="summary-big">{result.openCount}</div>
          <div class="summary-sub">open ports</div>
        </div>
        <div class="summary-cell">
          <div class="summary-label">Target</div>
          <div class="summary-val">{result.resolvedIp}</div>
        </div>
        <div class="summary-cell">
          <div class="summary-label">Start time</div>
          <div class="summary-val">{formatTime(result.startMs)}</div>
        </div>
        <div class="summary-cell">
          <div class="summary-label">Duration</div>
          <div class="summary-val">{formatDuration(result.startMs, result.endMs)}</div>
        </div>
      </div>
    </section>

    <!-- Results table -->
    <section class="section">
      <div class="section-header">
        <span class="section-label">RESULTS</span>
        <div class="section-line"></div>
      </div>

      {#if result.ports.length === 0}
        <div class="no-results">No open ports found on {result.resolvedIp}.</div>
      {:else}
        <div class="table-wrap">
          <table class="results-table">
            <thead>
              <tr>
                <th>Port</th>
                <th>Protocol</th>
                <th>State</th>
                <th>Service</th>
                <th>Product</th>
                <th>Version</th>
              </tr>
            </thead>
            <tbody>
              {#each result.ports as port}
                <tr>
                  <td class="td-port">{port.port}</td>
                  <td>{port.protocol}</td>
                  <td><span class="state-open"><span class="dot-green"></span>open</span></td>
                  <td class="td-service">{port.service ?? "—"}</td>
                  <td>{port.product ?? "—"}</td>
                  <td class="td-version">{port.version ?? "—"}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

  </main>

  <footer class="footer">
    <div class="footer-status">
      <span class="status-dot"></span>
      <span class="status-text">Scan Complete</span>
    </div>
    <div class="footer-actions">
      <button class="reset-btn" onclick={newScan}>New Scan</button>
    </div>
  </footer>

</div>
{/if}

<style>
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: #f9fafb;
    color: #111827;
    font-family: Inter, system-ui, -apple-system, sans-serif;
    font-size: 14px;
    -webkit-font-smoothing: antialiased;
  }

  /* ── APP SHELL ── */
  .app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: #f9fafb;
  }

  /* ── HEADER ── */
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 28px;
    background: #fff;
    border-bottom: 1px solid #e5e7eb;
    flex-shrink: 0;
  }

  .header-left { display: flex; align-items: center; gap: 14px; }

  .app-icon {
    width: 48px;
    height: 48px;
    background: #eef2ff;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .app-title { font-size: 17px; font-weight: 700; color: #111827; }
  .app-subtitle { font-size: 12px; color: #9ca3af; margin-top: 1px; }

  /* Scan mode tabs */
  .scan-mode-tabs { display: flex; gap: 2px; background: #f3f4f6; border-radius: 8px; padding: 3px; }

  .mode-tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 16px;
    border-radius: 6px;
    border: none;
    background: transparent;
    color: #6b7280;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
  }
  .mode-tab:hover { color: #374151; }
  .mode-tab.active {
    background: #fff;
    color: #4f46e5;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1), 0 0 0 1px rgba(79,70,229,0.2);
  }

  /* ── MAIN ── */
  .main {
    flex: 1;
    padding: 28px;
    display: flex;
    flex-direction: column;
    gap: 28px;
    max-width: 960px;
    width: 100%;
    margin: 0 auto;
  }

  /* ── SECTIONS ── */
  .section { display: flex; flex-direction: column; gap: 14px; }

  .section-header {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .section-label {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: #6b7280;
    white-space: nowrap;
  }

  .section-line {
    flex: 1;
    height: 1px;
    background: #e5e7eb;
  }

  /* ── TARGET INPUT ── */
  .target-input-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    padding: 14px 16px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .target-input-wrap:focus-within {
    border-color: #4f46e5;
    box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.08);
  }

  .target-icon { flex-shrink: 0; }

  .target-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    font-size: 14px;
    color: #111827;
    min-width: 0;
  }
  .target-input::placeholder { color: #9ca3af; }

  .target-badge {
    font-size: 11px;
    font-weight: 500;
    color: #6b7280;
    background: #f3f4f6;
    border: 1px solid #e5e7eb;
    border-radius: 5px;
    padding: 3px 8px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .error-msg { font-size: 13px; color: #dc2626; }

  /* ── TWO COLUMN ── */
  .two-col {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 28px;
  }

  .col { display: flex; flex-direction: column; gap: 14px; }

  /* ── PARAM CARDS ── */
  .param-cards { display: flex; flex-direction: column; gap: 10px; }

  .param-card {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    background: #fff;
    border: 1.5px solid #e5e7eb;
    border-radius: 10px;
    padding: 16px;
    cursor: pointer;
    transition: border-color 0.15s, box-shadow 0.15s;
    user-select: none;
    text-align: left;
    width: 100%;
    font-family: inherit;
    font-size: inherit;
  }
  .param-card:hover { border-color: #c7d2fe; }
  .param-card.selected {
    border-color: #4f46e5;
    box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.07);
  }

  .param-checkbox {
    width: 16px;
    height: 16px;
    border: 1.5px solid #d1d5db;
    border-radius: 4px;
    flex-shrink: 0;
    margin-top: 1px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, border-color 0.15s;
  }
  .param-checkbox.checked {
    background: #4f46e5;
    border-color: #4f46e5;
  }

  .param-text { display: flex; flex-direction: column; gap: 3px; }
  .param-title { font-size: 14px; font-weight: 600; color: #111827; }
  .param-desc { font-size: 12px; color: #6b7280; line-height: 1.4; }

  /* Protocol cards */
  .proto-card {
    flex-direction: row;
    align-items: center;
  }

  .proto-radio {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid #d1d5db;
    flex-shrink: 0;
    position: relative;
    transition: border-color 0.15s;
  }
  .proto-radio::after {
    content: "";
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%) scale(0);
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #4f46e5;
    transition: transform 0.15s;
  }
  .proto-radio.checked {
    border-color: #4f46e5;
  }
  .proto-radio.checked::after {
    transform: translate(-50%, -50%) scale(1);
  }

  .default-badge {
    margin-left: auto;
    font-size: 11px;
    font-weight: 500;
    color: #6b7280;
    background: #f3f4f6;
    border: 1px solid #e5e7eb;
    border-radius: 4px;
    padding: 2px 7px;
  }

  /* ── PORT RANGE ── */
  .port-range-card {
    background: #fff;
    border: 1.5px solid #e5e7eb;
    border-radius: 10px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .port-radios {
    display: flex;
    align-items: center;
    gap: 28px;
  }

  .port-radio-item {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
    font-size: 14px;
    font-weight: 500;
    color: #6b7280;
    transition: color 0.15s;
    background: transparent;
    border: none;
    padding: 0;
    font-family: inherit;
  }
  .port-radio-active { color: #111827; }

  .select-wrap { position: relative; }

  .port-select {
    width: 100%;
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 12px 40px 12px 14px;
    font-size: 14px;
    color: #374151;
    appearance: none;
    outline: none;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .port-select:focus { border-color: #4f46e5; }

  .select-chevron {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    pointer-events: none;
  }

  .custom-ports-input {
    width: 100%;
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 8px;
    padding: 12px 14px;
    font-size: 14px;
    color: #374151;
    outline: none;
    transition: border-color 0.15s;
  }
  .custom-ports-input::placeholder { color: #9ca3af; }
  .custom-ports-input:focus { border-color: #4f46e5; }

  /* ── FOOTER ── */
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 28px;
    background: #fff;
    border-top: 1px solid #e5e7eb;
    flex-shrink: 0;
  }

  .footer-status { display: flex; align-items: center; gap: 8px; }

  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #22c55e;
    box-shadow: 0 0 0 2px rgba(34,197,94,0.2);
  }

  .status-text { font-size: 13px; font-weight: 500; color: #374151; }
  .footer-divider { color: #d1d5db; font-size: 13px; }
  .version-text { font-size: 13px; color: #9ca3af; }

  .footer-actions { display: flex; align-items: center; gap: 12px; }

  .reset-btn {
    background: transparent;
    border: none;
    color: #6b7280;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    padding: 8px 12px;
    border-radius: 6px;
    transition: color 0.15s, background 0.15s;
  }
  .reset-btn:hover { color: #374151; background: #f3f4f6; }

  .scan-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #4f46e5;
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 10px 22px;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, box-shadow 0.15s;
    box-shadow: 0 1px 3px rgba(79,70,229,0.3);
  }
  .scan-btn:hover:not(.disabled) {
    background: #4338ca;
    box-shadow: 0 2px 6px rgba(79,70,229,0.4);
  }
  .scan-btn.disabled { opacity: 0.5; cursor: not-allowed; }

  /* ── SCANNING VIEW ── */
  .scanning-app { background: #f9fafb; }
  .scanning-main { align-items: center; justify-content: center; }

  .scanning-card {
    width: 100%;
    max-width: 480px;
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    padding: 32px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    text-align: center;
  }

  .scanning-label { font-size: 11px; font-weight: 700; letter-spacing: 0.08em; color: #6b7280; text-transform: uppercase; }
  .scanning-target { font-size: 18px; font-weight: 700; color: #111827; }

  .progress-track {
    height: 6px;
    background: #e5e7eb;
    border-radius: 3px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #4f46e5, #818cf8);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .scanning-stats {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: #9ca3af;
  }
  .pct-label { font-weight: 600; color: #4f46e5; }

  /* ── RESULTS VIEW ── */
  .results-app { background: #f9fafb; }
  .results-main { max-width: 960px; }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    overflow: hidden;
  }

  .summary-cell {
    padding: 20px 22px;
    border-right: 1px solid #e5e7eb;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .summary-cell:last-child { border-right: none; }

  .summary-big { font-size: 30px; font-weight: 800; color: #4f46e5; line-height: 1; }
  .summary-sub { font-size: 12px; color: #9ca3af; }
  .summary-label { font-size: 11px; color: #9ca3af; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600; }
  .summary-val { font-size: 13px; color: #374151; font-weight: 500; }

  .no-results {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    padding: 32px;
    text-align: center;
    color: #6b7280;
    font-size: 14px;
  }

  .table-wrap {
    background: #fff;
    border: 1px solid #e5e7eb;
    border-radius: 10px;
    overflow: hidden;
  }

  .results-table { width: 100%; border-collapse: collapse; font-size: 13px; }

  .results-table th {
    text-align: left;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #9ca3af;
    padding: 12px 16px;
    border-bottom: 1px solid #e5e7eb;
    background: #f9fafb;
  }

  .results-table td {
    padding: 13px 16px;
    border-bottom: 1px solid #f3f4f6;
    color: #374151;
    vertical-align: middle;
  }
  .results-table tr:last-child td { border-bottom: none; }
  .results-table tr:hover td { background: #fafafa; }

  .td-port { font-weight: 600; color: #111827; font-variant-numeric: tabular-nums; }
  .td-service { color: #4f46e5; font-weight: 500; }
  .td-version { font-size: 12px; color: #9ca3af; }

  .state-open { display: inline-flex; align-items: center; gap: 6px; }

  .dot-green {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #22c55e;
    box-shadow: 0 0 0 2px rgba(34,197,94,0.2);
    flex-shrink: 0;
  }
</style>
