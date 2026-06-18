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
const loading = ref(false);
const saving = ref(false);

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
  setInterval(refreshStatus, 2000);
});
</script>

<template>
  <div class="app">
    <div class="columns">
      <div class="col-left">
        <div class="group">
          <span class="group-label">Push-to-Talk</span>
          <div class="card">
            <button class="toggle-row" :class="{ active: status.active }" :disabled="loading" @click="togglePtt">
              <div class="toggle-label">
                <span class="toggle-title">{{ status.active ? "Enabled" : "Disabled" }}</span>
                <span class="toggle-sub">{{ status.active ? "Microphone muted" : "Microphone open" }}</span>
              </div>
              <div class="toggle-track">
                <div class="toggle-thumb"></div>
              </div>
            </button>
          </div>
        </div>

        <div class="group">
          <span class="group-label">Status</span>
          <div class="card">
            <div class="list-row">
              <span class="row-label">Daemon</span>
              <span class="row-value">{{ status.pid ?? "—" }}</span>
            </div>
            <div class="list-row">
              <span class="row-label">PTT Key</span>
              <kbd class="kbd">{{ formatKey(config.ptt_key) }}</kbd>
            </div>
            <div class="list-row last">
              <span class="row-label">Remap</span>
              <kbd class="kbd">{{ formatKey(config.remap_key) }}</kbd>
            </div>
          </div>
        </div>
      </div>

      <div class="col-right">
        <div class="group">
          <span class="group-label">Configuration</span>
          <div class="card">
            <div class="field-row">
              <label class="field-label">PTT Key</label>
              <div class="select-wrap">
                <select v-model="config.ptt_key">
                  <option v-for="key in keys" :key="key" :value="key">{{ formatKey(key) }}</option>
                </select>
                <svg class="select-arrow" width="10" height="10" viewBox="0 0 12 12" fill="none">
                  <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
            </div>
            <div class="field-row last">
              <label class="field-label">Remap To</label>
              <div class="select-wrap">
                <select v-model="config.remap_key">
                  <option v-for="key in keys" :key="key" :value="key">{{ formatKey(key) }}</option>
                </select>
                <svg class="select-arrow" width="10" height="10" viewBox="0 0 12 12" fill="none">
                  <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              </div>
            </div>
          </div>
        </div>

        <div class="group">
          <span class="group-label">How to use</span>
          <div class="card">
            <p class="help-text">Hold <kbd class="kbd inline">{{ formatKey(config.ptt_key) }}</kbd> to unmute your microphone while PTT is enabled.</p>
          </div>
        </div>

        <div class="actions">
          <button class="btn-suggested" :disabled="saving" @click="saveConfig">
            {{ saving ? "Saving\u2026" : "Apply" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
:root {
  font-family: "Cantarell", "Noto Sans", system-ui, sans-serif;
  font-size: 13px;
  line-height: 1.5;
  color: #ffffff;
  background-color: #242424;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
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
  width: 100%;
  height: 100vh;
  display: flex;
}

.columns {
  display: flex;
  gap: 16px;
  width: 100%;
  padding: 16px 20px;
}

.col-left,
.col-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.group {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.group-label {
  font-size: 11px;
  font-weight: 700;
  color: #929292;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding-left: 2px;
}

.card {
  background: #303030;
  border: 1px solid #3d3d3d;
  border-radius: 12px;
  overflow: hidden;
}

.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 12px 16px;
  background: transparent;
  border: none;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s ease;
}

.toggle-row:hover {
  background: rgba(255, 255, 255, 0.03);
}

.toggle-row:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.toggle-row.active .toggle-title {
  color: #3584e4;
}

.toggle-row.active .toggle-track {
  background: #3584e4;
}

.toggle-row.active .toggle-thumb {
  transform: translateX(18px);
  background: #ffffff;
}

.toggle-label {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.toggle-title {
  font-size: 14px;
  font-weight: 600;
  color: #ffffff;
  transition: color 0.15s ease;
}

.toggle-sub {
  font-size: 12px;
  color: #929292;
}

.toggle-track {
  width: 40px;
  height: 22px;
  background: #555555;
  border-radius: 11px;
  position: relative;
  flex-shrink: 0;
  transition: background 0.2s ease;
}

.toggle-thumb {
  width: 16px;
  height: 16px;
  background: #cccccc;
  border-radius: 50%;
  position: absolute;
  top: 3px;
  left: 3px;
  transition: transform 0.2s ease, background 0.2s ease;
}

.list-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
}

.list-row:not(.last) {
  border-bottom: 1px solid #3d3d3d;
}

.row-label {
  font-size: 13px;
  color: #d0d0d0;
}

.row-value {
  font-size: 13px;
  font-weight: 500;
  color: #ffffff;
}

.kbd {
  display: inline-flex;
  align-items: center;
  height: 24px;
  padding: 0 8px;
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  color: #ffffff;
  background: #3d3d3d;
  border: 1px solid #555555;
  border-radius: 6px;
}

.kbd.inline {
  height: auto;
  padding: 1px 6px;
}

.help-text {
  font-size: 13px;
  color: #d0d0d0;
  padding: 10px 16px;
  line-height: 1.5;
}

.field-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 16px;
}

.field-row:not(.last) {
  border-bottom: 1px solid #3d3d3d;
}

.field-label {
  font-size: 13px;
  color: #d0d0d0;
  flex-shrink: 0;
}

.select-wrap {
  position: relative;
}

.select-wrap select {
  appearance: none;
  min-width: 140px;
  padding: 6px 30px 6px 10px;
  background: #242424;
  border: 1px solid #555555;
  border-radius: 8px;
  color: #ffffff;
  font-family: inherit;
  font-size: 13px;
  outline: none;
  cursor: pointer;
  transition: border-color 0.15s ease;
}

.select-wrap select:hover {
  border-color: #666666;
}

.select-wrap select:focus {
  border-color: #3584e4;
}

.select-arrow {
  position: absolute;
  right: 10px;
  top: 50%;
  transform: translateY(-50%);
  color: #929292;
  pointer-events: none;
}

.actions {
  display: flex;
  justify-content: flex-end;
  margin-top: auto;
}

.btn-suggested {
  padding: 8px 24px;
  background: #3584e4;
  border: none;
  border-radius: 8px;
  color: #ffffff;
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease;
}

.btn-suggested:hover {
  background: #4394e8;
}

.btn-suggested:active {
  background: #2e75c9;
}

.btn-suggested:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
