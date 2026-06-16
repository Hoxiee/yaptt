<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface PttStatus {
  active: boolean;
  pid: number | null;
}

const status = ref<PttStatus>({ active: false, pid: null });
const loading = ref(false);

async function refreshStatus() {
  try {
    status.value = await invoke("get_status");
  } catch (e) {
    console.error("Failed to get status:", e);
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

onMounted(() => {
  refreshStatus();
  setInterval(refreshStatus, 2000);
});
</script>

<template>
  <div class="app">
    <header class="header">
      <h1>PTT</h1>
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
          <span class="status-label">Daemon PID</span>
          <span class="status-value">{{ status.pid ?? "—" }}</span>
        </div>
        <div class="status-row">
          <span class="status-label">PTT Key</span>
          <span class="status-value">Tilde</span>
        </div>
        <div class="status-row">
          <span class="status-label">Mic</span>
          <span class="status-value">{{ status.active ? "Muted (hold to talk)" : "Open" }}</span>
        </div>
      </div>
    </main>

    <footer class="footer">
      <span>Click to toggle • Hold Tilde to talk</span>
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
  gap: 32px;
  padding: 40px;
  width: 100%;
  max-width: 400px;
}

.header {
  text-align: center;
}

.header h1 {
  font-size: 28px;
  font-weight: 700;
  color: #88d1ec;
  letter-spacing: 2px;
}

.subtitle {
  color: #8a9296;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 3px;
  margin-top: 4px;
}

.main {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
  width: 100%;
}

.toggle-btn {
  width: 140px;
  height: 140px;
  border-radius: 50%;
  border: 2px solid #40484c;
  background: #1b2023;
  color: #8a9296;
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
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
  font-size: 48px;
  line-height: 1;
}

.label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 2px;
}

.status-card {
  width: 100%;
  background: #1b2023;
  border: 1px solid #40484c;
  border-radius: 12px;
  padding: 16px 20px;
}

.status-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
}

.status-row + .status-row {
  border-top: 1px solid #252b2d;
}

.status-label {
  color: #8a9296;
  font-size: 13px;
}

.status-value {
  color: #dee3e6;
  font-size: 13px;
  font-weight: 500;
}

.status-value.active {
  color: #a6e3a1;
}

.footer {
  color: #40484c;
  font-size: 11px;
  text-align: center;
}
</style>
