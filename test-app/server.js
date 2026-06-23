// Minimal test app to validate node-winsvc.
// Writes a heartbeat to a log every 3 seconds and serves a tiny HTTP endpoint.
const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = process.env.PORT || 7799;
const logPath = path.join(__dirname, 'heartbeat.log');

function log(msg) {
  const line = `[${new Date().toISOString()}] ${msg}\n`;
  fs.appendFileSync(logPath, line);
  console.log(line.trim());
}

log(`test-app starting (pid=${process.pid}, PORT=${PORT}, NODE_ENV=${process.env.NODE_ENV})`);

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ ok: true, pid: process.pid, uptime: process.uptime() }));
});

server.listen(PORT, () => log(`listening on http://localhost:${PORT}`));

setInterval(() => log(`heartbeat (uptime=${Math.round(process.uptime())}s)`), 3000);

process.on('SIGTERM', () => { log('SIGTERM received, shutting down'); process.exit(0); });
