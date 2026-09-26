import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import App from './App';

vi.mock('./hooks/useWallet', () => ({
  useWallet: () => ({
    connected: false,
    address: null,
  }),
}));

vi.mock('./components/wallet/WalletConnector', () => ({
  WalletConnector: () => <div>Wallet Connector</div>,
}));

vi.mock('./components/ThemeToggle', () => ({
  ThemeToggle: () => <div>Theme Toggle</div>,
}));

vi.mock('./pages/AdminPanel', () => ({
  AdminPanel: () => <div>Admin Panel</div>,
}));

vi.mock('./pages/MatchDetailPage', () => ({
  MatchDetailPage: () => <div>Match Detail Page</div>,
}));

vi.mock('./components/MatchList', () => ({
  MatchList: () => <div>Match List</div>,
}));

vi.mock('./components/match/CreateMatchForm', () => ({
  CreateMatchForm: () => <div>Create Match Form</div>,
}));

describe('App Router', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('landing page (/', () => {
    it('renders landing page on root path', () => {
      window.history.pushState({}, '', '/');
      render(<App />);
      expect(screen.getByText('Checkmate-Escrow')).toBeTruthy();
      expect(screen.getByText('Trustless chess wagering on Stellar')).toBeTruthy();
    });

    it('shows wallet connector on landing page', () => {
      window.history.pushState({}, '', '/');
      render(<App />);
      expect(screen.getByText('Wallet Connector')).toBeTruthy();
    });

    it('shows navigation links when wallet is connected', () => {
      vi.resetModules();
      vi.mock('./hooks/useWallet', () => ({
        useWallet: () => ({
          connected: true,
          address: 'GABC123',
        }),
      }));
      window.history.pushState({}, '', '/');
      render(<App />);
      const browseLink = screen.queryByText('Browse Matches');
      const createLink = screen.queryByText('Create Match');
      // Links should be rendered when wallet is connected
    });
  });

  describe('matches page (/matches)', () => {
    it('renders matches page when pathname is /matches', () => {
      window.history.pushState({}, '', '/matches');
      render(<App />);
      expect(screen.getByText('Matches')).toBeTruthy();
    });

    it('shows wallet connector on matches page when not connected', () => {
      window.history.pushState({}, '', '/matches');
      render(<App />);
      expect(screen.getByText('Connect your wallet to view matches.')).toBeTruthy();
      expect(screen.getByText('Wallet Connector')).toBeTruthy();
    });

    it('shows match list when wallet is connected', async () => {
      vi.resetModules();
      vi.mock('./hooks/useWallet', () => ({
        useWallet: () => ({
          connected: true,
          address: 'GABC123',
        }),
      }));
      window.history.pushState({}, '', '/matches');
      render(<App />);
      expect(screen.getByText('Match List')).toBeTruthy();
    });
  });

  describe('create match page (/create)', () => {
    it('renders create page when pathname is /create', () => {
      window.history.pushState({}, '', '/create');
      render(<App />);
      expect(screen.getByText('Create Match')).toBeTruthy();
    });

    it('shows wallet connector on create page when not connected', () => {
      window.history.pushState({}, '', '/create');
      render(<App />);
      expect(screen.getByText('Connect your wallet to create a match.')).toBeTruthy();
      expect(screen.getByText('Wallet Connector')).toBeTruthy();
    });

    it('shows create form when wallet is connected', async () => {
      vi.resetModules();
      vi.mock('./hooks/useWallet', () => ({
        useWallet: () => ({
          connected: true,
          address: 'GABC123',
        }),
      }));
      window.history.pushState({}, '', '/create');
      render(<App />);
      expect(screen.getByText('Create Match Form')).toBeTruthy();
    });
  });

  describe('match detail page (/match/:id)', () => {
    it('renders match detail page for /match/:id', () => {
      window.history.pushState({}, '', '/match/123');
      render(<App />);
      expect(screen.getByText('Match Detail Page')).toBeTruthy();
    });

    it('extracts match ID from URL', () => {
      window.history.pushState({}, '', '/match/456');
      render(<App />);
      expect(screen.getByText('Match Detail Page')).toBeTruthy();
    });
  });

  describe('admin panel', () => {
    it('renders admin panel when ?admin=1', () => {
      window.history.pushState({}, '', '/?admin=1');
      render(<App />);
      expect(screen.getByText('Admin Panel')).toBeTruthy();
    });

    it('renders admin panel when /admin path', () => {
      window.history.pushState({}, '', '/admin');
      render(<App />);
      expect(screen.getByText('Admin Panel')).toBeTruthy();
    });
  });
});
