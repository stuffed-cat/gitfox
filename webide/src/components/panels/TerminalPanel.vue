<template>
  <div class="terminal-panel">
    <div ref="terminalRef" class="terminal-container"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import 'xterm/css/xterm.css'

const terminalRef = ref<HTMLElement | null>(null)
let terminal: Terminal | null = null
let fitAddon: FitAddon | null = null

onMounted(() => {
  if (!terminalRef.value) return
  
  terminal = new Terminal({
    theme: {
      background: '#1e1e2e',
      foreground: '#cdd6f4',
      cursor: '#f5e0dc',
      cursorAccent: '#1e1e2e',
      selectionBackground: '#45475a',
      black: '#45475a',
      red: '#f38ba8',
      green: '#a6e3a1',
      yellow: '#f9e2af',
      blue: '#89b4fa',
      magenta: '#cba6f7',
      cyan: '#94e2d5',
      white: '#bac2de',
      brightBlack: '#585b70',
      brightRed: '#f38ba8',
      brightGreen: '#a6e3a1',
      brightYellow: '#f9e2af',
      brightBlue: '#89b4fa',
      brightMagenta: '#cba6f7',
      brightCyan: '#94e2d5',
      brightWhite: '#a6adc8'
    },
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    fontSize: 13,
    cursorBlink: true
  })
  
  fitAddon = new FitAddon()
  terminal.loadAddon(fitAddon)
  terminal.open(terminalRef.value)
  fitAddon.fit()
  
  // Welcome message
  terminal.writeln('\x1b[1;36m欢迎使用 GitFox WebIDE 终端\x1b[0m')
  terminal.writeln('')
  terminal.write('$ ')
  
  // Handle resize
  const resizeObserver = new ResizeObserver(() => {
    fitAddon?.fit()
  })
  resizeObserver.observe(terminalRef.value)
  
  // Handle input (for demo purposes)
  terminal.onData((data) => {
    if (data === '\r') {
      terminal?.writeln('')
      terminal?.write('$ ')
    } else if (data === '\x7f') { // Backspace
      terminal?.write('\b \b')
    } else {
      terminal?.write(data)
    }
  })
})

onUnmounted(() => {
  terminal?.dispose()
})
</script>

<style lang="scss" scoped>
.terminal-panel {
  height: 100%;
  background: #1e1e2e;
  padding: 8px;
}

.terminal-container {
  height: 100%;
}
</style>
