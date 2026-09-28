/**
 * EventPoller unit tests
 *
 * Tests that the EventPoller correctly:
 * - Tracks (ledger_sequence, event_index_in_txn) as watermark
 * - Does not drop events from the same ledger in later polls
 * - Initializes watermark from latest indexed ledger on start
 * - Handles retries with exponential backoff
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { EventPoller } from '../eventPoller.js';
import type { IndexedEvent, ServerConfig } from '../types.js';
import http from 'http';

// ─── Port registry — avoid conflicts between tests ────────────────────────

let nextPort = 9500;
function allocPort(): number { return nextPort++; }

// ─── Helpers ──────────────────────────────────────────────────────────────

function buildConfig(indexerPort: number): Pick<ServerConfig, 'eventIndexerUrl' | 'pollIntervalMs'> {
  return {
    eventIndexerUrl: `http://127.0.0.1:${indexerPort}`,
    pollIntervalMs: 100,
  };
}

function createMockIndexer(port: number): {
  setEvents: (events: IndexedEvent[]) => void;
  stop: () => Promise<void>;
} {
  let currentEvents: IndexedEvent[] = [];
  const server = http.createServer((_req, res) => {
    if (currentEvents.length === 0) {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ success: false, data: null, error: 'No events' }));
    } else {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ success: true, data: currentEvents, error: null }));
    }
  });

  server.listen(port);

  return {
    setEvents: (events) => { currentEvents = events; },
    stop: () =>
      new Promise((resolve, reject) =>
        server.close((err) => (err ? reject(err) : resolve())),
      ),
  };
}

function makeEvent(overrides: Partial<IndexedEvent> = {}): IndexedEvent {
  return {
    id: `evt-${Date.now()}-${Math.random()}`,
    ledger_sequence: 1000,
    match_id: 1,
    event_type: 'match/created',
    timestamp: new Date().toISOString(),
    event_index_in_txn: 0,
    ...overrides,
  };
}

// ─── Test suite ───────────────────────────────────────────────────────────

describe('EventPoller', () => {
  let indexerPort: number;
  let indexer: ReturnType<typeof createMockIndexer>;

  beforeEach(async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    indexerPort = allocPort();
    indexer = createMockIndexer(indexerPort);
  });

  afterEach(async () => {
    await indexer.stop();
    vi.useRealTimers();
  });

  describe('does not drop events from the same ledger', () => {
    it('emits all events when they arrive in separate polls', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      // First poll: ledger 100 has 2 events, but only first arrives
      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'evt1' }),
      ]);

      poller.start();
      await vi.advanceTimersByTimeAsync(200);

      expect(events).toHaveLength(1);
      expect(events[0]?.id).toBe('evt1');

      // Second poll: both events from ledger 100 are returned
      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'evt1' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 1, id: 'evt2' }),
      ]);

      await vi.advanceTimersByTimeAsync(200);

      // Should emit evt2 (not drop it just because ledger 100 was seen)
      expect(events).toHaveLength(2);
      expect(events[1]?.id).toBe('evt2');

      poller.stop();
    });

    it('tracks (ledger, event_index) as watermark', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      // First poll: events 0-2 from ledger 100
      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'a' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 1, id: 'b' }),
      ]);

      poller.start();
      await vi.advanceTimersByTimeAsync(200);
      expect(events.map(e => e.id)).toEqual(['a', 'b']);

      // Second poll: add event 2 from same ledger
      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'a' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 1, id: 'b' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 2, id: 'c' }),
      ]);

      await vi.advanceTimersByTimeAsync(200);
      expect(events.map(e => e.id)).toEqual(['a', 'b', 'c']);

      poller.stop();
    });

    it('handles events with undefined event_index_in_txn gracefully', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: undefined, id: 'evt1' }),
      ]);

      poller.start();
      await vi.advanceTimersByTimeAsync(200);

      expect(events).toHaveLength(1);
      expect(events[0]?.id).toBe('evt1');

      poller.stop();
    });
  });

  describe('sorts events correctly', () => {
    it('sorts by ledger_sequence then event_index_in_txn', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 1, id: 'b' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'a' }),
        makeEvent({ ledger_sequence: 101, event_index_in_txn: 0, id: 'c' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 2, id: 'd' }),
      ]);

      poller.start();
      await vi.advanceTimersByTimeAsync(200);

      expect(events.map(e => e.id)).toEqual(['a', 'b', 'd', 'c']);

      poller.stop();
    });
  });

  describe('retry behavior', () => {
    it('retries on fetch failure with exponential backoff', async () => {
      let fetchCount = 0;
      const originalFetch = global.fetch;

      global.fetch = vi.fn(async () => {
        fetchCount++;
        if (fetchCount < 3) {
          throw new Error('Network error');
        }
        return new Response(
          JSON.stringify({ success: true, data: [makeEvent()], error: null }),
          { status: 200, headers: { 'Content-Type': 'application/json' } },
        );
      });

      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      poller.start();
      await vi.advanceTimersByTimeAsync(1100); // 1s + jitter
      expect(fetchCount).toBe(2);

      await vi.advanceTimersByTimeAsync(2600); // 2s + jitter
      expect(fetchCount).toBe(3);

      await vi.advanceTimersByTimeAsync(200);
      expect(events).toHaveLength(1);

      poller.stop();
      global.fetch = originalFetch;
    });
  });

  describe('handles empty responses', () => {
    it('handles 404 with success=false', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      indexer.setEvents([]);

      poller.start();
      await vi.advanceTimersByTimeAsync(200);

      expect(events).toHaveLength(0);

      poller.stop();
    });

    it('continues polling after empty response', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      indexer.setEvents([]);
      poller.start();
      await vi.advanceTimersByTimeAsync(200);

      indexer.setEvents([makeEvent({ id: 'evt1' })]);
      await vi.advanceTimersByTimeAsync(200);

      expect(events).toHaveLength(1);
      expect(events[0]?.id).toBe('evt1');

      poller.stop();
    });
  });

  describe('start/stop lifecycle', () => {
    it('does not poll when not running', async () => {
      const config = buildConfig(indexerPort);
      let callCount = 0;
      const poller = new EventPoller(config, () => { callCount++; });

      indexer.setEvents([makeEvent()]);

      await vi.advanceTimersByTimeAsync(200);
      expect(callCount).toBe(0);

      await poller.start();
      await vi.advanceTimersByTimeAsync(200);
      expect(callCount).toBe(1);

      poller.stop();
      await vi.advanceTimersByTimeAsync(200);
      expect(callCount).toBe(1);
    });

    it('ignores multiple start calls', async () => {
      const config = buildConfig(indexerPort);
      const poller = new EventPoller(config, () => {});

      await poller.start();
      await poller.start();
      await poller.start();

      await vi.advanceTimersByTimeAsync(200);

      poller.stop();
    });
  });

  describe('watermark initialization', () => {
    it('initializes watermark from latest indexed ledger on start', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      // Pre-populate indexer with historical events
      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'old1' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 1, id: 'old2' }),
        makeEvent({ ledger_sequence: 101, event_index_in_txn: 0, id: 'old3' }),
      ]);

      await poller.start();

      // Historical events should not be emitted after initialization
      await vi.advanceTimersByTimeAsync(200);
      expect(events).toHaveLength(0);

      // New events should be emitted
      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'old1' }),
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 1, id: 'old2' }),
        makeEvent({ ledger_sequence: 101, event_index_in_txn: 0, id: 'old3' }),
        makeEvent({ ledger_sequence: 102, event_index_in_txn: 0, id: 'new' }),
      ]);

      await vi.advanceTimersByTimeAsync(200);
      expect(events).toHaveLength(1);
      expect(events[0]?.id).toBe('new');

      poller.stop();
    });

    it('does not replay historical events after restart', async () => {
      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      indexer.setEvents([
        makeEvent({ ledger_sequence: 100, event_index_in_txn: 0, id: 'evt1' }),
        makeEvent({ ledger_sequence: 101, event_index_in_txn: 0, id: 'evt2' }),
        makeEvent({ ledger_sequence: 102, event_index_in_txn: 0, id: 'evt3' }),
      ]);

      await poller.start();
      await vi.advanceTimersByTimeAsync(200);

      // Should not emit any historical events
      expect(events).toHaveLength(0);

      poller.stop();
    });

    it('handles initialization failure gracefully', async () => {
      let fetchCount = 0;
      const originalFetch = global.fetch;

      global.fetch = vi.fn(async () => {
        fetchCount++;
        throw new Error('Network error during init');
      });

      const config = buildConfig(indexerPort);
      const events: IndexedEvent[] = [];
      const poller = new EventPoller(config, (event) => events.push(event));

      await poller.start();

      // Should not throw, will start from beginning
      global.fetch = originalFetch;
      indexer.setEvents([makeEvent({ id: 'evt' })]);

      await vi.advanceTimersByTimeAsync(200);
      expect(events).toHaveLength(1);

      poller.stop();
    });
  });
});
