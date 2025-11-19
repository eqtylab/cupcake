/**
 * OPA Binary Installer for Cupcake TypeScript Bindings
 *
 * This module handles automatic download and verification of the OPA binary
 * required for policy compilation.
 *
 * Version: OPA v1.7.1
 */

import * as crypto from 'crypto';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { promisify } from 'util';
import { execFile } from 'child_process';

const execFileAsync = promisify(execFile);

// OPA Version Configuration
const OPA_VERSION = 'v0.70.0';
const OPA_BASE_URL = `https://github.com/open-policy-agent/opa/releases/download/${OPA_VERSION}`;

interface BinaryInfo {
  binary: string;
  sha256: string;
  size_mb: number;
}

// Platform-specific binary mapping with SHA256 checksums
const OPA_BINARIES: Record<string, BinaryInfo> = {
  'darwin-x64': {
    binary: 'opa_darwin_amd64',
    sha256: '51da8fa6ce4ac9b963d4babbd78714e98880b20e74f30a3f45a96334e12830bd',
    size_mb: 67.3,
  },
  'darwin-arm64': {
    binary: 'opa_darwin_arm64_static',
    sha256: 'fe2a14b6ba7f587caeb62ef93ef62d1e713776a6e470f4e87326468a8ecfbfbd',
    size_mb: 43.8,
  },
  'linux-x64': {
    binary: 'opa_linux_amd64',
    sha256: '7426bf5504049d7444f9ee9a1d47a64261842f38f5308903ef6b76ba90250b5a',
    size_mb: 67.1,
  },
  'linux-arm64': {
    binary: 'opa_linux_arm64_static',
    sha256: 'a81af8cd767f1870e9e23b8ed0ad8f40b24e5c0a64c5768c75d5c292aaa81e54',
    size_mb: 43.2,
  },
  'win32-x64': {
    binary: 'opa_windows_amd64.exe',
    sha256: '205f87d0fd1e2673c3a6f9caf9d9655290e478a93eeb3ef9f211acdaa214a9ca',
    size_mb: 98.7,
  },
};

/**
 * Get the directory for caching OPA binaries
 */
function getCacheDir(): string {
  let cacheBase: string;

  if (process.platform === 'win32') {
    cacheBase = process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local');
  } else {
    cacheBase = process.env.XDG_CACHE_HOME || path.join(os.homedir(), '.cache');
  }

  const cacheDir = path.join(cacheBase, 'cupcake', 'bin');
  fs.mkdirSync(cacheDir, { recursive: true });

  return cacheDir;
}

/**
 * Get platform identifier for OPA binary lookup
 */
function getPlatformKey(): string {
  const platform = process.platform;
  const arch = process.arch;

  // Normalize platform and arch
  const platformMap: Record<string, string> = {
    darwin: 'darwin',
    linux: 'linux',
    win32: 'win32',
  };

  const archMap: Record<string, string> = {
    x64: 'x64',
    arm64: 'arm64',
  };

  const normalizedPlatform = platformMap[platform];
  const normalizedArch = archMap[arch];

  if (!normalizedPlatform || !normalizedArch) {
    throw new Error(`Unsupported platform: ${platform} ${arch}`);
  }

  return `${normalizedPlatform}-${normalizedArch}`;
}

/**
 * Verify the SHA256 checksum of a file
 */
async function verifyChecksum(filePath: string, expectedSha256: string): Promise<boolean> {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);

    stream.on('data', (data) => hash.update(data));
    stream.on('end', () => {
      const actual = hash.digest('hex');
      resolve(actual === expectedSha256);
    });
    stream.on('error', reject);
  });
}

/**
 * Download a file with progress indication
 */
async function downloadWithProgress(url: string, destPath: string, expectedSizeMb: number): Promise<void> {
  console.log(`Downloading OPA ${OPA_VERSION} (${expectedSizeMb.toFixed(1)} MB)...`);

  const fetch = require('node-fetch');
  const response = await fetch(url);

  if (!response.ok) {
    throw new Error(`Failed to download: ${response.statusText}`);
  }

  const totalSize = parseInt(response.headers.get('content-length') || '0', 10);
  let downloaded = 0;

  const fileStream = fs.createWriteStream(destPath);

  return new Promise((resolve, reject) => {
    response.body!.on('data', (chunk: Buffer) => {
      downloaded += chunk.length;
      fileStream.write(chunk);

      if (totalSize > 0) {
        const percent = (downloaded / totalSize) * 100;
        process.stdout.write(`Progress: ${percent.toFixed(1)}%\r`);
      }
    });

    response.body!.on('end', () => {
      fileStream.end();
      console.log('\nDownload complete!');
      resolve();
    });

    response.body!.on('error', (err: Error) => {
      fileStream.close();
      reject(err);
    });
  });
}

/**
 * Make a file executable on Unix-like systems
 */
async function makeExecutable(filePath: string): Promise<void> {
  if (process.platform !== 'win32') {
    await fs.promises.chmod(filePath, 0o755);
  }
}

/**
 * Download and verify the OPA binary for the current platform
 *
 * @param force - Force re-download even if binary exists
 * @returns Path to the OPA executable
 */
export async function downloadOpa(force = false): Promise<string> {
  // Get platform info
  const platformKey = getPlatformKey();

  if (!(platformKey in OPA_BINARIES)) {
    throw new Error(
      `Unsupported platform: ${platformKey}\n` + `Supported platforms: ${Object.keys(OPA_BINARIES).join(', ')}`,
    );
  }

  const binaryInfo = OPA_BINARIES[platformKey];
  const { binary: binaryName, sha256: expectedSha256, size_mb: sizeMb } = binaryInfo;

  // Determine local path
  const cacheDir = getCacheDir();
  let localName = `opa-${OPA_VERSION}`;
  if (process.platform === 'win32') {
    localName += '.exe';
  }
  const localPath = path.join(cacheDir, localName);

  // Check if already downloaded and valid
  if (fs.existsSync(localPath) && !force) {
    console.log(`Verifying existing OPA binary at ${localPath}...`);
    if (await verifyChecksum(localPath, expectedSha256)) {
      console.log('Checksum verified successfully!');
      return localPath;
    } else {
      console.log('Checksum verification failed, re-downloading...');
      await fs.promises.unlink(localPath);
    }
  }

  // Download the binary
  const url = `${OPA_BASE_URL}/${binaryName}`;
  const tempPath = localPath + '.tmp';

  try {
    await downloadWithProgress(url, tempPath, sizeMb);

    // Verify checksum
    console.log('Verifying checksum...');
    if (!(await verifyChecksum(tempPath, expectedSha256))) {
      throw new Error(
        `Checksum verification failed for ${binaryName}\n` + `This could indicate a corrupted download or security issue.`,
      );
    }

    // Move to final location
    await fs.promises.rename(tempPath, localPath);
    await makeExecutable(localPath);

    console.log(`OPA ${OPA_VERSION} installed successfully at ${localPath}`);
    return localPath;
  } catch (error) {
    // Clean up on failure
    if (fs.existsSync(tempPath)) {
      await fs.promises.unlink(tempPath);
    }
    throw new Error(`Failed to download OPA: ${error}`);
  }
}

/**
 * Find OPA binary in order of preference:
 * 1. Cached download
 * 2. System PATH
 *
 * @returns Path to OPA or null if not found
 */
async function findOpa(): Promise<string | null> {
  // Check cache
  const cacheDir = getCacheDir();
  let localName = `opa-${OPA_VERSION}`;
  if (process.platform === 'win32') {
    localName += '.exe';
  }
  const cachedPath = path.join(cacheDir, localName);

  if (fs.existsSync(cachedPath)) {
    return cachedPath;
  }

  // Check system PATH
  const opaCmd = process.platform === 'win32' ? 'opa.exe' : 'opa';

  try {
    const { stdout } = await execFileAsync(process.platform === 'win32' ? 'where' : 'which', [opaCmd]);
    const opaPath = stdout.trim().split('\n')[0];

    if (opaPath && fs.existsSync(opaPath)) {
      // For compatibility, accept any OPA version that can be executed
      // The version check is primarily for ensuring the downloaded binary is correct
      try {
        const { stdout: versionOutput } = await execFileAsync(opaPath, ['version'], { timeout: 5000 });
        // If we can get version output, OPA is functional
        if (versionOutput && versionOutput.includes('Version:')) {
          return opaPath;
        }
      } catch {
        // Version check failed, continue
      }
    }
  } catch {
    // which/where failed, OPA not in PATH
  }

  return null;
}

/**
 * Ensure OPA is installed and return its path
 *
 * This is the main entry point for the installer.
 * It will use an existing OPA if found, or download it if needed.
 *
 * @returns Path to the OPA executable
 */
export async function ensureOpaInstalled(): Promise<string> {
  // Try to find existing OPA
  const opaPath = await findOpa();
  if (opaPath) {
    return opaPath;
  }

  // Download OPA
  return downloadOpa();
}

/**
 * Get the OPA command for subprocess calls
 *
 * @returns String path to OPA executable
 */
export async function getOpaCommand(): Promise<string> {
  return ensureOpaInstalled();
}
