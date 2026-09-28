import { describe, it, expect } from 'vitest';
import { readFileSync, existsSync } from 'fs';
import path from 'path';

describe('Cleanup: remove unused config and dependencies (#1614)', () => {
  it('verifies jest.config.js has been removed from frontend', () => {
    const jestConfigPath = path.resolve(__dirname, '../../jest.config.js');
    const exists = existsSync(jestConfigPath);
    expect(exists).toBe(false);
  });

  it('verifies frontend uses vitest for testing', () => {
    const packageJsonPath = path.resolve(__dirname, '../../package.json');
    const packageContent = readFileSync(packageJsonPath, 'utf-8');
    const packageJson = JSON.parse(packageContent);

    expect(packageJson.devDependencies?.vitest).toBeDefined();
    expect(packageJson.scripts?.test).toContain('vitest');
  });

  it('verifies node-fetch has been removed from websocket-server dependencies', () => {
    const wsPackageJsonPath = path.resolve(__dirname, '../../../services/websocket-server/package.json');
    const packageContent = readFileSync(wsPackageJsonPath, 'utf-8');
    const packageJson = JSON.parse(packageContent);

    expect(packageJson.dependencies?.['node-fetch']).toBeUndefined();
    expect(packageJson.devDependencies?.['node-fetch']).toBeUndefined();
  });

  it('confirms no jest config files exist in frontend directory', () => {
    const jestConfigPath = path.resolve(__dirname, '../../jest.config.js');
    const jestConfigJsonPath = path.resolve(__dirname, '../../jest.config.json');
    const jestConfigTsPath = path.resolve(__dirname, '../../jest.config.ts');

    expect(existsSync(jestConfigPath)).toBe(false);
    expect(existsSync(jestConfigJsonPath)).toBe(false);
    expect(existsSync(jestConfigTsPath)).toBe(false);
  });

  it('confirms vitest configuration is present in vite.config.ts', () => {
    const viteConfigPath = path.resolve(__dirname, '../../vite.config.ts');
    if (existsSync(viteConfigPath)) {
      const viteContent = readFileSync(viteConfigPath, 'utf-8');
      // Should have vitest configuration
      expect(viteContent).toContain('test:');
    }
  });
});
