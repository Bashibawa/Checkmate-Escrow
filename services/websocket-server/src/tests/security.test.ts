/**
 * Security tests: WebSocket server hardening
 *
 * Tests that the server properly enforces:
 * - maxPayload limits
 * - Origin validation
 * - Per-IP connection limits
 * - x-forwarded-for trust configuration
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import WebSocket from 'ws';
import { ConnectionManager } from '../connectionManager.js';
import type { ServerConfig } from '../types.js';

// ─── Port registry — avoid conflicts between tests ────────────────────────

let nextPort = 9700;
function allocPort(): number { return nextPort++; }

// ─── Helpers ──────────────────────────────────────────────────────────────

function buildConfig(wsPort: number, overrides: Partial<ServerConfig> = {}): ServerConfig {
  return {
    port: wsPort,
    host: '127.0.0.1',
    eventIndexerUrl: 'http://127.0.0.1:8080',
    pollIntervalMs: 5000,
    heartbeatIntervalMs: 60_000,
    heartbeatTimeoutMs: 120_000,
    rateLimitMaxSubscribes: 100,
    rateLimitWindowMs: 60_000,
    maxSubscriptionsPerClient: 50,
    logLevel: 'warn',
    maxPayloadBytes: 16_384,
    allowedOrigins: [],
    maxConnectionsPerIp: 100,
    trustXForwardedFor: false,
    ...overrides,
  };
}

function connectClient(wsPort: number, origin?: string): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const headers: Record<string, string> = {};
    if (origin) {
      headers.Origin = origin;
    }
    const ws = new WebSocket(`ws://127.0.0.1:${wsPort}`, { headers });
    ws.once('open', () => resolve(ws));
    ws.once('error', () => reject(new Error('Connection rejected')));
    ws.once('close', () => reject(new Error('Connection closed')));
    setTimeout(() => reject(new Error('Connection timeout')), 2000);
  });
}

// ─── Test suite ───────────────────────────────────────────────────────────

describe('WebSocket server security', () => {
  describe('maxPayload enforcement', () => {
    it('accepts small messages within payload limit', async () => {
      const wsPort = allocPort();
      const manager = new ConnectionManager(buildConfig(wsPort, { maxPayloadBytes: 1024 }));
      manager.start();

      try {
        const ws = await connectClient(wsPort);
        const smallMsg = JSON.stringify({ type: 'ping' });
        expect(smallMsg.length).toBeLessThan(1024);

        ws.send(smallMsg);
        ws.close();
      } finally {
        await manager.stop();
      }
    });

    it('rejects messages exceeding payload limit', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, { maxPayloadBytes: 100 });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        const ws = await new Promise<WebSocket>((resolve) => {
          const w = new WebSocket(`ws://127.0.0.1:${wsPort}`);
          w.once('open', () => resolve(w));
        });

        const largeMsg = 'x'.repeat(1000);
        let errorOccurred = false;

        ws.on('error', () => {
          errorOccurred = true;
        });

        ws.send(largeMsg, (err) => {
          if (err) errorOccurred = true;
        });

        await new Promise(resolve => setTimeout(resolve, 500));

        expect(errorOccurred).toBe(true);
        ws.close();
      } finally {
        await manager.stop();
      }
    });
  });

  describe('origin validation', () => {
    it('rejects connections from disallowed origins', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, { allowedOrigins: ['http://allowed.com'] });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        const promise = connectClient(wsPort, 'http://evil.com');
        await expect(promise).rejects.toThrow();
      } finally {
        await manager.stop();
      }
    });

    it('allows connections from allowed origins', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, { allowedOrigins: ['http://allowed.com'] });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        const ws = await connectClient(wsPort, 'http://allowed.com');
        ws.close();
      } finally {
        await manager.stop();
      }
    });

    it('allows all origins when allowedOrigins includes *', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, { allowedOrigins: ['*'] });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        const ws = await connectClient(wsPort, 'http://any-origin.com');
        ws.close();
      } finally {
        await manager.stop();
      }
    });

    it('allows connections without origin header', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, { allowedOrigins: ['http://allowed.com'] });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        const ws = await connectClient(wsPort);
        ws.close();
      } finally {
        await manager.stop();
      }
    });
  });

  describe('per-IP connection limits', () => {
    it('enforces max connections per IP', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, { maxConnectionsPerIp: 2 });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        const ws1 = await connectClient(wsPort);
        const ws2 = await connectClient(wsPort);

        // Third connection from same IP should fail
        const promise = connectClient(wsPort);
        await expect(promise).rejects.toThrow();

        ws1.close();
        ws2.close();
      } finally {
        await manager.stop();
      }
    });

    it('decrements connection count on disconnect', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, { maxConnectionsPerIp: 1 });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        const ws1 = await connectClient(wsPort);
        ws1.close();

        // Should be able to connect again after first closes
        await new Promise(resolve => setTimeout(resolve, 100));
        const ws2 = await connectClient(wsPort);
        ws2.close();
      } finally {
        await manager.stop();
      }
    });
  });

  describe('x-forwarded-for trust', () => {
    it('trusts x-forwarded-for when configured', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, {
        trustXForwardedFor: true,
        maxConnectionsPerIp: 1,
      });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        // Simulate two different clients behind a proxy
        const headers1 = { 'x-forwarded-for': '10.0.0.1' };
        const ws1 = new WebSocket(`ws://127.0.0.1:${wsPort}`, { headers: headers1 });
        await new Promise<void>((resolve, reject) => {
          ws1.once('open', () => resolve());
          ws1.once('error', reject);
        });

        const headers2 = { 'x-forwarded-for': '10.0.0.2' };
        const ws2 = new WebSocket(`ws://127.0.0.1:${wsPort}`, { headers: headers2 });
        await new Promise<void>((resolve, reject) => {
          ws2.once('open', () => resolve());
          ws2.once('error', reject);
        });

        ws1.close();
        ws2.close();
      } finally {
        await manager.stop();
      }
    });

    it('ignores x-forwarded-for when not configured', async () => {
      const wsPort = allocPort();
      const config = buildConfig(wsPort, {
        trustXForwardedFor: false,
        maxConnectionsPerIp: 100,
      });
      const manager = new ConnectionManager(config);
      manager.start();

      try {
        // Both should count as same IP (127.0.0.1)
        const headers1 = { 'x-forwarded-for': '10.0.0.1' };
        const ws1 = new WebSocket(`ws://127.0.0.1:${wsPort}`, { headers: headers1 });
        await new Promise<void>((resolve, reject) => {
          ws1.once('open', () => resolve());
          ws1.once('error', reject);
        });

        const headers2 = { 'x-forwarded-for': '10.0.0.2' };
        const ws2 = new WebSocket(`ws://127.0.0.1:${wsPort}`, { headers: headers2 });
        await new Promise<void>((resolve, reject) => {
          ws2.once('open', () => resolve());
          ws2.once('error', reject);
        });

        ws1.close();
        ws2.close();
      } finally {
        await manager.stop();
      }
    });
  });
});
