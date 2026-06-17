<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface PttStatus {
  active: boolean;
  pid: number | null;
}

interface PttConfig {
  ptt_key: string;
  remap_key: string;
  source: string | null;
}

const status = ref<PttStatus>({ active: false, pid: null });
const config = ref<PttConfig>({ ptt_key: "grave", remap_key: "f13", source: null });
const keys = ref<string[]>([]);
const sources = ref<string[]>([]);
const loading = ref(false);
const saving = ref(false);
const showSettings = ref(false);

async function refreshStatus() {
  try {
    status.value = await invoke("get_status");
  } catch (e) {
    console.error("Failed to get status:", e);
  }
}

async function loadConfig() {
  try {
    config.value = await invoke("get_config");
  } catch (e) {
    console.error("Failed to load config:", e);
  }
}

async function loadKeys() {
  try {
    keys.value = await invoke("get_keys");
  } catch (e) {
    console.error("Failed to load keys:", e);
  }
}

async function loadSources() {
  try {
    sources.value = await invoke("get_sources");
  } catch (e) {
    console.error("Failed to load sources:", e);
  }
}

async function togglePtt() {
  loading.value = true;
  try {
    status.value = await invoke("toggle");
  } catch (e) {
    console.error("Failed to toggle:", e);
  }
  loading.value = false;
}

async function saveConfig() {
  saving.value = true;
  try {
    config.value = await invoke("save_config_command", { config: config.value });
  } catch (e) {
    console.error("Failed to save config:", e);
  }
  saving.value = false;
}

function formatKey(key: string): string {
  return key.charAt(0).toUpperCase() + key.slice(1);
}

onMounted(() => {
  refreshStatus();
  loadConfig();
  loadKeys();
  loadSources();
  setInterval(refreshStatus, 2000);
});
</script>

<template>
  <div class="app">
    <header class="header">
      <div class="header-row">
        <h1>PTT</h1>
        <button class="settings-btn" :class="{ active: showSettings }" @click="showSettings = !showSettings">
          󰄽
        </button>
      </div>
      <p class="subtitle">Push-to-Talk</p>
    </header>

    <main class="main">
      <button
        class="toggle-btn"
        :class="{ active: status.active }"
        :disabled="loading"
        @click="togglePtt"
      >
        <span class="icon">{{ status.active ? "󰍬" : "󰍭" }}</span>
        <span class="label">{{ status.active ? "Active" : "Inactive" }}</span>
      </button>

      <div class="status-card">
        <div class="status-row">
          <span class="status-label">State</span>
          <span class="status-value" :class="{ active: status.active }">
            {{ status.active ? "ON" : "OFF" }}
          </span>
        </div>
        <div class="status-row">
          <span class="status-label">Daemon</span>
          <span class="status-value">{{ status.pid ?? "—" }}</span>
        </div>
        <div class="status-row">
          <span class="status-label">PTT Key</span>
          <span class="status-value">{{ formatKey(config.ptt_key) }}</span>
        </div>
        <div class="status-row">
          <span class="status-label">Remap</span>
          <span class="status-value">{{ formatKey(config.remap_key) }}</span>
        </div>
        <div class="status-row">
          <span class="status-label">Mic</span>
          <span class="status-value">{{ status.active ? "Hold to talk" : "Open" }}</span>
        </div>
      </div>

      <div v-if="showSettings" class="settings-card">
        <h3>Configuration</h3>

        <div class="field">
          <label>PTT Key</label>
          <select v-model="config.ptt_key">
            <option v-for="key in keys" :key="key" :value="key">{{ formatKey(key) }}</option>
          </select>
        </div>

        <div class="field">
          <label>Remap To</label>
          <select v-model="config.remap_key">
            <option v-for="key in keys" :key="key" :value="key">{{ formatKey(key) }}</option>
          </select>
        </div>

        <div class="field">
          <label>Audio Source</label>
          <select v-model="config.source">
            <option :value="null">Default</option>
            <option v-for="src in sources" :key="src" :value="src">{{ src }}</option>
          </select>
        </div>

        <button class="save-btn" :disabled="saving" @click="saveConfig">
          {{ saving ? "Saving..." : "Save" }}
        </button>
      </div>
    </main>

    <footer class="footer">
      <span>Hold {{ formatKey(config.ptt_key) }} to talk</span>
    </footer>
  </div>
</template>

<style>
:root {
  font-family: "JetBrainsMono Nerd Font", Inter, system-ui, sans-serif;
  font-size: 14px;
  color: #dee3e6;
  background-color: #0f1416;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  min-height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
}

#app {
  width: 100%;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
}
</style>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  padding: 32px;
  width: 100%;
  max-width: 400px;
}

.header {
  text-align: center;
  width: 100%;
}

.header-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}

.header h1 {
  font-size: 28px;
  font-weight: 700;
  color: #88d1ec;
  letter-spacing: 2px;
}

.settings-btn {
  background: none;
  border: 1px solid #40484c;
  border-radius: 8px;
  color: #8a9296;
  padding: 4px 8px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
}

.settings-btn:hover,
.settings-btn.active {
  border-color: #88d1ec;
  color: #88d1ec;
}

.subtitle {
  color: #8a9296;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 3px;
  margin-top: 4px;
}

.main {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
  width: 100%;
}

.toggle-btn {
  width: 130px;
  height: 130px;
  border-radius: 50%;
  border: 2px solid #40484c;
  background: #1b2023;
  color: #8a9296;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.3s ease;
}

.toggle-btn:hover {
  border-color: #88d1ec;
  background: #252b2d;
}

.toggle-btn.active {
  border-color: #a6e3a1;
  background: #1a2e1a;
  color: #a6e3a1;
  box-shadow: 0 0 30px rgba(166, 227, 161, 0.15);
}

.toggle-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.icon {
  font-size: 44px;
  line-height: 1;
}

.label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 2px;
}

.status-card,
.settings-card {
  width: 100%;
  background: #1b2023;
  border: 1px solid #40484c;
  border-radius: 12px;
  padding: 14px 18px;
}

.settings-card h3 {
  color: #88d1ec;
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 12px;
}

.status-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 7px 0;
}

.status-row + .status-row {
  border-top: 1px solid #252b2d;
}

.status-label {
  color: #8a9296;
  font-size: 12px;
}

.status-value {
  color: #dee3e6;
  font-size: 12px;
  font-weight: 500;
}

.status-value.active {
  color: #a6e3a1;
}

.field {
  margin-bottom: 10px;
}

.field label {
  display: block;
  color: #8a9296;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 4px;
}

.field select {
  width: 100%;
  padding: 8px 10px;
  background: #0f1416;
  border: 1px solid #40484c;
  border-radius: 6px;
  color: #dee3e6;
  font-family: inherit;
  font-size: 12px;
  outline: none;
  cursor: pointer;
}

.field select:focus {
  border-color: #88d1ec;
}

.save-btn {
  width: 100%;
  padding: 8px;
  background: #88d1ec;
  border: none;
  border-radius: 6px;
  color: #003544;
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.2s;
}

.save-btn:hover {
  opacity: 0.9;
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.footer {
  color: #40484c;
  font-size: 11px;
  text-align: center;
}
</style>
